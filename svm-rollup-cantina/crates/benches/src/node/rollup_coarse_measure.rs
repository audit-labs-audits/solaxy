#[macro_use]
extern crate prettytable;

use std::{
    collections::HashMap,
    default::Default,
    env,
    num::NonZero,
    path::Path,
    str::FromStr,
    time::{Duration, Instant},
};

use benches::{setup_with_genesis_and_roles, BenchmarkType};
use humantime::format_duration;
use prettytable::Table;
use solana_sdk::hash::Hash;
use sov_blob_storage::PreferredBatchData;
use sov_db::{
    ledger_db::{LedgerDb, SlotCommit},
    storage_manager::NativeStorageManager,
};
use sov_mock_da::{MockAddress, MockBlob, MockBlock, MockBlockHeader, MockDaSpec};
use sov_mock_zkvm::{MockCodeCommitment, MockZkvm, MockZkvmCryptoSpec};
use sov_modules_api::{
    capabilities::HasKernel, execution_mode::Native, ApiStateAccessor, BatchSequencerOutcome,
    CryptoSpec, StateCheckpoint,
};
use sov_modules_stf_blueprint::{GenesisParams, StfBlueprint};
use sov_rollup_interface::{
    common::SlotNumber,
    da::{BlockHeaderTrait, RelevantBlobs},
    node::da::SlotData,
    stf::{ExecutionContext, StateTransitionFunction},
    storage::HierarchicalStorageManager,
};
use sov_state::StorageRoot;
use sov_test_utils::{
    BatchType, SequencerInfo, SoftConfirmationBlobInfo, TestStorageManager, TestStorageSpec,
    TransactionType,
};
use stf::runtime::Runtime;
use svm::{authentication::SolanaAuthenticator, test_utils::SvmMessageGenerator, SVM};
use svm_types::{NativeStorage, SolanaSpec, SolanaTestSpec};
use tempfile::TempDir;

type TestSpec =
    SolanaSpec<MockDaSpec, MockZkvm, MockZkvm, MockZkvmCryptoSpec, Native, NativeStorage>;
type BenchStf = StfBlueprint<TestSpec, Runtime<TestSpec>>;

// Minimum TPS below which it is considered an issue
const MIN_TPS: f64 = 10.0; // This is 1000 in the Sov code
                           // Number to check that rollup actually executed some transactions
const MAX_TPS: f64 = 1_000_000.0;

fn print_times(
    total: Duration,
    apply_block_time: Duration,
    params: &BenchParams,
    num_success_txns: usize,
) {
    let mut table = Table::new();

    let total_txns = params.blocks * params.transactions_per_block;
    table.add_row(row!["Blocks", format!("{:?}", params.blocks)]);
    table.add_row(row![
        "Transactions per block",
        format!("{:?}", params.transactions_per_block)
    ]);
    table.add_row(row![
        "Preloaded accounts",
        format!("{:?}", params.preloaded_accounts)
    ]);
    table.add_row(row![
        "Processed transactions (success/total)",
        format!(
            "{:?}/{:?}",
            num_success_txns - params.preloaded_accounts,
            total_txns
        )
    ]);
    table.add_row(row!["Total", format_duration(total)]);
    table.add_row(row!["Apply block", format_duration(apply_block_time)]);
    let tps = (total_txns as f64) / total.as_secs_f64();
    table.add_row(row!["Transactions per sec (TPS)", format!("{:.1}", tps)]);
    table.add_row(row![
        "Microseconds per tx",
        format!("{:.0}", 1_000_000.0 / tps)
    ]);

    // Print the table to stdout
    table.printstd();

    assert!(
        tps > MIN_TPS,
        "TPS {} dropped below {}, investigation is needed",
        tps,
        MIN_TPS
    );
    assert!(
        tps < MAX_TPS,
        "TPS {} reached unrealistic number {}, investigation is needed",
        tps,
        MAX_TPS
    );
}

#[derive(Debug)]
struct BenchParams {
    blocks: usize,
    transactions_per_block: usize,
    preloaded_accounts: usize,
    timer_output: bool,
}

impl BenchParams {
    fn new() -> Self {
        let mut blocks = 10;
        let mut transactions_per_block = 500;
        let mut preloaded_accounts = 1;
        let mut timer_output = true;

        if let Ok(val) = env::var("TXNS_PER_BLOCK") {
            transactions_per_block = val
                .parse::<usize>()
                .expect("TXNS_PER_BLOCK var should be a non-negative integer");
        }
        if let Ok(val) = env::var("BLOCKS") {
            blocks = val
                .parse::<usize>()
                .expect("BLOCKS var should be a non-negative integer");
        }
        if let Ok(val) = env::var("PRELOADED_ACCOUNTS") {
            preloaded_accounts = val
                .parse::<usize>()
                .expect("PRELOADED_ACCOUNTS var should be a non-negative integer");
        }
        if let Ok(val) = env::var("TIMER_OUTPUT") {
            match val.as_str() {
                "true" | "1" | "yes" => {
                    timer_output = true;
                }
                "false" | "0" | "no" => (),
                val => {
                    panic!(
                        "Unknown value '{}' for TIMER_OUTPUT. expected true/false/0/1/yes/no",
                        val
                    );
                }
            }
        }

        Self {
            blocks,
            transactions_per_block,
            preloaded_accounts,
            timer_output,
        }
    }
}

pub type S = SolanaTestSpec<MockDaSpec>;
pub type RT = Runtime<S>;
pub type TestSvm = SVM<S>;

fn authenticate_transactions(
    txs: &[solana_sdk::transaction::Transaction],
) -> Vec<TransactionType<RT, TestSpec>> {
    txs.iter()
        .map(|tx| {
            let serialized_tx = borsh::to_vec(tx).unwrap();
            TransactionType::PreAuthenticated(RT::encode_with_solana_auth(serialized_tx))
        })
        .collect()
}

#[allow(dead_code)]
fn batches_to_blobs(
    batches: Vec<(BatchType<RT, S>, MockAddress)>,
    nonces: &mut HashMap<<MockZkvmCryptoSpec as CryptoSpec>::PublicKey, u64>,
) -> RelevantBlobs<MockBlob> {
    let blobs = batches
        .into_iter()
        .map(|(batch, sequencer)| {
            let txns = batch
                .0
                .into_iter()
                .map(|tx| tx.to_serialized_authenticated_tx(nonces))
                .collect::<Vec<_>>();
            MockBlob::new_with_hash(borsh::to_vec(&txns).unwrap(), sequencer)
        })
        .collect::<Vec<_>>();

    RelevantBlobs {
        batch_blobs: blobs,
        proof_blobs: Vec::new(),
    }
}

fn soft_confirmation_batches_to_blobs(
    batches: Vec<SoftConfirmationBlobInfo<RT, S>>,
    nonces: &mut HashMap<<MockZkvmCryptoSpec as CryptoSpec>::PublicKey, u64>,
) -> RelevantBlobs<MockBlob> {
    let blobs = batches
        .into_iter()
        .map(
            |SoftConfirmationBlobInfo {
                 batch_type: batch,
                 sequencer_address,
                 sequencer_info,
             }| {
                let raw_txns = batch
                    .0
                    .into_iter()
                    .map(|tx| tx.to_serialized_authenticated_tx(nonces))
                    .collect::<Vec<_>>();

                let serialized_batch = match sequencer_info {
                    SequencerInfo::Preferred {
                        slots_to_advance,
                        sequence_number,
                    } => borsh::to_vec(&PreferredBatchData {
                        sequence_number,
                        data: raw_txns,
                        visible_slots_to_advance: NonZero::new(slots_to_advance).unwrap(),
                    })
                    .unwrap(),
                    SequencerInfo::Regular => borsh::to_vec(&raw_txns).unwrap(),
                };

                MockBlob::new_with_hash(serialized_batch, sequencer_address)
            },
        )
        .collect::<Vec<_>>();

    RelevantBlobs {
        batch_blobs: blobs,
        proof_blobs: Vec::new(),
    }
}

// Sets up storage and returns blocks that should be benchmarked.
fn setup(
    params: &BenchParams,
    path: impl AsRef<Path>,
    benchmark_type: BenchmarkType,
) -> (
    TestStorageManager,
    StorageRoot<TestStorageSpec>,
    Vec<MockBlock>,
) {
    let mut storage_manager =
        NativeStorageManager::new(path.as_ref()).expect("StorageManager initialization failed");
    let stf = BenchStf::new();
    let do_initial_block = params.preloaded_accounts > 0;
    let mut initial_block_count = 0; // genesis block

    let (genesis_config, roles) =
        setup_with_genesis_and_roles(params.blocks as u64, MockCodeCommitment::default());

    // Genesis
    let genesis_params = {
        let mut rt_params = genesis_config;

        // Funding gas for senders
        let remaining_gas_token_amount = u64::MAX
            - rt_params
                .bank
                .gas_token_config
                .address_and_balances
                .iter()
                .map(|(_, amount)| u64::try_from(amount.0).expect("balance not to overflow u64"))
                .sum::<u64>();
        let gas_per_sender = remaining_gas_token_amount / roles.senders.len() as u64;

        for sender in roles.senders.iter() {
            rt_params
                .bank
                .gas_token_config
                .address_and_balances
                .push((sender.address(), gas_per_sender.into()));
        }

        GenesisParams { runtime: rt_params }
    };
    let genesis_block_header = MockBlockHeader::from_height(0);
    let (stf_state, ledger_state) = storage_manager
        .create_state_for(&genesis_block_header)
        .expect("Getting genesis storage failed");

    let mut ledger_db = LedgerDb::with_reader(ledger_state).unwrap();

    let (current_root, stf_state) = stf.init_chain(&Default::default(), stf_state, genesis_params);
    let data_to_commit: SlotCommit<MockBlock, BatchSequencerOutcome, ()> =
        SlotCommit::new(MockBlock {
            header: genesis_block_header.clone(),
            ..Default::default()
        });
    let mut ledger_change_set = ledger_db
        .materialize_slot(data_to_commit, current_root.as_ref())
        .unwrap();
    let finalized_slot_changes = ledger_db
        .materialize_latest_finalize_slot(SlotNumber::new(0))
        .unwrap();
    ledger_change_set.merge(finalized_slot_changes);

    storage_manager
        .save_change_set(&genesis_block_header, stf_state, ledger_change_set)
        .expect("Saving genesis storage failed");

    let setup_block_header = MockBlockHeader::from_height(1);

    let (stf_state, ledger_storage) = storage_manager
        .create_state_for(&setup_block_header)
        .unwrap();
    ledger_db.replace_reader(ledger_storage);

    let mut nonces = HashMap::new();
    // Blocks for benchmark
    let mut blocks = Vec::with_capacity(params.blocks);

    let mut rt = RT::default();
    let empty_checkpoint = StateCheckpoint::new(stf_state, &rt.kernel());
    let mut state =
        ApiStateAccessor::new(&empty_checkpoint, stf.runtime().kernel_with_slot_mapping());

    let block_hash = stf
        .runtime()
        .svm
        .get_latest_blockhash(None, &mut state)
        .map(|res| Hash::from_str(&res.value.blockhash).unwrap())
        .unwrap();

    // If we're preloading a bunch of accounts, then set up an initial block with those transactions
    if do_initial_block && !roles.senders.is_empty() {
        initial_block_count += 1;

        let msg_gen: SvmMessageGenerator<TestSpec> =
            SvmMessageGenerator::generate_sample_sol_transactions(
                params.preloaded_accounts,
                0,
                block_hash,
                roles.senders[0].private_key.clone(),
            );

        let fully_baked_txs = authenticate_transactions(&msg_gen.txs);

        let blobs = soft_confirmation_batches_to_blobs(
            vec![SoftConfirmationBlobInfo {
                batch_type: BatchType::from(fully_baked_txs),
                sequencer_address: roles.preferred_sequencer.sequencer_info.da_address,
                sequencer_info: SequencerInfo::Preferred {
                    slots_to_advance: 1,
                    sequence_number: 0,
                },
            }],
            &mut nonces,
        );

        blocks.push(MockBlock {
            header: MockBlockHeader::from_height(1),
            batch_blobs: blobs.batch_blobs,
            proof_blobs: blobs.proof_blobs,
        });
    }

    for (idx, test_user) in roles.senders.iter().enumerate() {
        let msg_gen: SvmMessageGenerator<TestSpec> = match benchmark_type {
            BenchmarkType::SolTransfer => SvmMessageGenerator::generate_sample_sol_transactions(
                0,
                params.transactions_per_block,
                block_hash,
                test_user.private_key.clone(),
            ),
            BenchmarkType::SplSuite => SvmMessageGenerator::generate_sample_spl_transactions(
                params.transactions_per_block,
                block_hash,
                test_user.private_key.clone(),
            ),
        };

        let fully_baked_txs = authenticate_transactions(&msg_gen.txs);
        let blobs = soft_confirmation_batches_to_blobs(
            vec![SoftConfirmationBlobInfo {
                batch_type: BatchType::from(fully_baked_txs),
                sequencer_address: roles.preferred_sequencer.sequencer_info.da_address,
                sequencer_info: SequencerInfo::Preferred {
                    slots_to_advance: 1,
                    sequence_number: idx as u64 + initial_block_count,
                },
            }],
            &mut nonces,
        );
        blocks.push(MockBlock {
            header: MockBlockHeader::from_height(idx as u64 + initial_block_count + 1),
            batch_blobs: blobs.batch_blobs,
            proof_blobs: blobs.proof_blobs,
        });
    }

    (storage_manager, current_root, blocks)
}

fn benchmark(benchmark_type: BenchmarkType) -> anyhow::Result<()> {
    let params = BenchParams::new();
    let mut num_success_txns: usize = 0;
    let temp_dir = TempDir::new().expect("Unable to create temporary directory");
    let (mut storage_manager, mut current_root, blocks) =
        setup(&params, temp_dir.path(), benchmark_type);
    // 3 blocks to finalization
    let fork_length = 3;
    let first_block = blocks
        .first()
        .expect("There should be at least 1 block")
        .header();
    let first_block_height = first_block.height();

    let (_, ledger_storage) = storage_manager.create_state_after(first_block).unwrap();
    let mut ledger_db = LedgerDb::with_reader(ledger_storage).unwrap();
    let stf = BenchStf::new();

    let mut total = Instant::now();
    let mut apply_block_time = Duration::default();

    for (idx, filtered_block) in blocks.into_iter().enumerate() {
        let (stf_state, ledger_storage) = storage_manager
            .create_state_for(filtered_block.header())
            .unwrap();
        ledger_db.replace_reader(ledger_storage);

        let mut data_to_commit = SlotCommit::new(filtered_block.clone());
        let MockBlock {
            header: filtered_header,
            proof_blobs,
            batch_blobs,
            ..
        } = filtered_block;

        let mut relevant_blobs = RelevantBlobs::<MockBlob> {
            proof_blobs,
            batch_blobs,
        };

        let now = Instant::now();
        let apply_block_result = stf.apply_slot(
            &current_root,
            stf_state,
            Default::default(),
            &filtered_header,
            relevant_blobs.as_iters(),
            ExecutionContext::Node,
        );
        apply_block_time += now.elapsed();
        current_root = apply_block_result.state_root;

        for receipt in apply_block_result.batch_receipts {
            for t in &receipt.tx_receipts {
                if t.receipt.is_successful() {
                    num_success_txns += 1;
                } else {
                    println!("E: {:?}", t.receipt);
                }
            }
            data_to_commit.add_batch(receipt);
        }

        let mut ledger_change_set = ledger_db
            .materialize_slot(data_to_commit, current_root.as_ref())
            .unwrap();

        let header_to_finalize = match filtered_header.height().checked_sub(fork_length) {
            None => None,
            Some(height_to_finalize) => {
                if height_to_finalize >= first_block_height {
                    let finalized_slot_changes = ledger_db
                        .materialize_latest_finalize_slot(SlotNumber::new(height_to_finalize))
                        .unwrap();
                    ledger_change_set.merge(finalized_slot_changes);
                    Some(MockBlockHeader::from_height(height_to_finalize))
                } else {
                    None
                }
            }
        };
        storage_manager
            .save_change_set(
                &filtered_header,
                apply_block_result.change_set,
                ledger_change_set,
            )
            .unwrap();

        if let Some(header) = header_to_finalize {
            storage_manager.finalize(&header).unwrap();
        }

        // restart the timer if the first block is filled with initial account creation
        if idx == 0 && params.preloaded_accounts > 0 {
            total = Instant::now();
        }
    }

    let total = total.elapsed();
    assert_eq!(
        params.blocks * params.transactions_per_block + params.preloaded_accounts,
        num_success_txns,
        "Not enough successful transactions, something is broken"
    );

    println!("\nBenchmark: {:?}", benchmark_type);

    if params.timer_output {
        print_times(total, apply_block_time, &params, num_success_txns);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    benchmark(BenchmarkType::SolTransfer)?;
    benchmark(BenchmarkType::SplSuite)?;
    Ok(())
}
