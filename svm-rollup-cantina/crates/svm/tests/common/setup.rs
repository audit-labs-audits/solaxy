//! Helper functions and types for the SVM module integration tests.
#![allow(dead_code)]

use std::{str::FromStr, sync::Once};

use solana_program::{
    hash::Hash, native_token::LAMPORTS_PER_SOL, program_option::COption, program_pack::Pack,
};
use solana_sdk::{
    account::{Account, AccountSharedData},
    epoch_info::EpochInfo,
    pubkey::Pubkey,
    system_program,
    sysvar::{clock, epoch_rewards, epoch_schedule, rent},
    transaction::Transaction,
};
use sov_bank::Amount;
use sov_mock_da::MockDaSpec;
use sov_modules_api::{ApiStateAccessor, CredentialId, HexHash};
use sov_test_utils::{
    interface::TransactionType,
    runtime::{genesis::optimistic::HighLevelOptimisticGenesisConfig, TestRunner},
    TestUser, TransactionTestCase,
};
use svm::{authentication::SolanaAuthenticator, InitSvmConfig, SVM};
use svm_types::SolanaTestSpec;

use super::runtime::{GenesisConfig, SvmRuntime};
static INIT: Once = Once::new();

pub type S = SolanaTestSpec<MockDaSpec>;
pub type RT = SvmRuntime<S>;
pub type TestSvm = SVM<S>;

/// Setup `TestRunner` with default `SVM` genesis config.
pub fn setup_svm_with_defaults() -> TestRunner<RT, S> {
    setup_svm_with_config(InitSvmConfig::default())
}

/// Setup `TestRunner` with provided accounts.
pub fn setup_svm_with_accounts(
    genesis_accounts: &[(Pubkey, AccountSharedData)],
) -> TestRunner<RT, S> {
    let config = InitSvmConfig::new(genesis_accounts, &[]);
    setup_svm_with_config(config)
}

/// Setup `TestRunner` with provided `SVM` genesis config.
pub fn setup_svm_with_config(svm_config: InitSvmConfig) -> TestRunner<RT, S> {
    let svm_accounts = svm_config.accounts.clone();
    let test_accounts = svm_accounts
        .iter()
        .map(|(key, account)| {
            TestUser::<S>::generate(Amount(account.lamports as u128))
                .add_credential_id(CredentialId(HexHash::new((*key).into())))
        })
        .collect();
    let genesis_config = HighLevelOptimisticGenesisConfig::generate().add_accounts(test_accounts);
    let genesis_hash = svm_config.hash().to_string();
    let genesis = GenesisConfig::from_minimal_config(genesis_config.into(), svm_config);
    let runner = TestRunner::new_with_genesis(genesis.into_genesis_params(), RT::default());

    // Validate genesis state
    // Check if Solana system-level accounts are present and their `executable` flags are correctly set.
    check_system_accounts(&runner);

    // Check extra accounts defined in `InitSvmConfig::new` are persisted in post-genesis state.
    check_added_accounts(
        &runner,
        &svm_accounts
            .keys()
            .map(|pubkey| Pubkey::from(*pubkey))
            .collect::<Vec<_>>(),
    );

    runner.query_state(|state| {
        let svm_module = TestSvm::default();
        // Check epoch info is set
        let epoch_info = svm_module
            .get_epoch_info(None, state)
            .expect("epoch info must be set during genesis");
        assert_eq!(
            epoch_info,
            EpochInfo {
                epoch: 0,
                slot_index: 0,
                slots_in_epoch: 0,
                absolute_slot: 0,
                block_height: 0,
                transaction_count: None,
            }
        );

        // Check genesis hash
        let genesis = svm_module
            .get_genesis_hash(state)
            .expect("genesis hash must be set");
        assert_eq!(genesis, genesis_hash, "Genesis hash mismatch");

        // Check blockhash
        let blockhash = svm_module
            .get_latest_blockhash(None, state)
            .expect("blockhash must be set during genesis")
            .value
            .blockhash;
        assert_eq!(blockhash, genesis_hash, "Blockhash mismatch");

        // TODO: Continue checking remaining Genesis operations
    });

    runner
}

pub(crate) fn check_added_accounts(runner: &TestRunner<RT, S>, accounts: &[Pubkey]) {
    runner.query_state(|state| {
        let svm_module = TestSvm::default();
        let result = svm_module.get_multiple_accounts(
            accounts.iter().map(|key| key.into()).collect(),
            None,
            state,
        );
        assert!(result.is_ok(), "Couldn't fetch added accounts");
    });
}

pub(crate) fn check_system_accounts(runner: &TestRunner<RT, S>) {
    let system_accounts = [
        (clock::id(), false),
        (epoch_rewards::id(), false),
        (epoch_schedule::id(), false),
        (rent::id(), false),
        (spl_token::id(), true),
        (spl_associated_token_account::id(), true),
        (spl_token_2022::id(), true),
    ];

    runner.query_state(|state| {
        let system_result = TestSvm::default()
            .get_multiple_accounts(
                system_accounts.iter().map(|(key, _)| key.into()).collect(),
                None,
                state,
            )
            .expect("Couldn't fetch system accounts");
        for (account, (id, executable)) in system_result.value.iter().zip(system_accounts.iter()) {
            let account = account
                .as_ref()
                .expect("System account not found in genesis state");
            assert_eq!(
                account.executable, *executable,
                "wrong account 'executable' flag: account={id:?}, executable={executable}",
            );
            if *executable {
                assert!(
                    !account.data.decode().unwrap().is_empty(),
                    "System account {id:?} has no bytecode in its data"
                );
            }
        }
    });
}

pub fn create_input(transaction: &Transaction) -> TransactionType<RT, S> {
    let serialized = borsh::to_vec(transaction).unwrap();
    TransactionType::PreAuthenticated(RT::encode_with_solana_auth(serialized))
}

/// Builds the necessary scaffolding to process a Solana transaction and executes it within the
/// `TestRunner`, passing a note to identify the transaction in case of failure.
pub fn execute_call_message(transaction: &Transaction, runner: &mut TestRunner<RT, S>, note: &str) {
    let input = create_input(transaction);
    let note = note.to_string();
    let test_case = TransactionTestCase {
        input,
        assert: Box::new(move |result, _state| {
            assert!(
                result.tx_receipt.is_successful(),
                "Transaction {note} failed: {:?}",
                result.tx_receipt
            );
        }),
    };
    runner.execute_transaction(test_case);
}

/// Builds the necessary scaffolding to process a batch of solana transactions and executes them
/// within the `TestRunner`, passing a note to identify each transaction in case of failure.
pub fn execute_call_message_batch(
    messages: &[(Transaction, &str)],
    runner: &mut TestRunner<RT, S>,
) {
    let transactions: Vec<_> = messages.iter().map(|(tx, _)| create_input(tx)).collect();
    let notes = messages
        .iter()
        .map(|(_, note)| (*note).to_string())
        .collect::<Vec<_>>();
    let test_batch = sov_test_utils::BatchTestCase {
        input: sov_test_utils::BatchType(transactions),
        assert: Box::new(move |result, _state| {
            let batch_receipt = result.batch_receipt.unwrap();
            for (i, tx_receipt) in batch_receipt.tx_receipts.iter().enumerate() {
                assert!(
                    tx_receipt.receipt.is_successful(),
                    "Transaction {} failed: {tx_receipt:?}",
                    notes[i],
                );
            }
        }),
    };
    runner.execute_batch(test_batch);
}

pub struct UserProgram {
    pub keypair_path: &'static str,
    pub bytecode: &'static [u8],
}

/// Returns an account with a balance of 1 SOL, owned by the system program.
pub fn system_account() -> AccountSharedData {
    AccountSharedData::new(LAMPORTS_PER_SOL, 0, &system_program::id())
}

pub fn get_account_info(pubkey: Pubkey, state: &mut ApiStateAccessor<S>) -> Account {
    TestSvm::default()
        .get_account_info(pubkey.into(), None, state)
        .expect("account not found")
        .value
        .unwrap()
        .decode()
        .unwrap()
}

/// Returns the most recent block hash from the SVM module state.
pub fn recent_blockhash(runner: &TestRunner<RT, S>) -> Hash {
    runner
        .query_state(|state| {
            TestSvm::default()
                .get_latest_blockhash(None, state)
                .map(|res| Hash::from_str(&res.value.blockhash).unwrap())
        })
        .expect("should've obtained a recent blockhash from SVM module state")
}

pub fn set_up_logger() {
    INIT.call_once(|| {
        solana_logger::setup_with_default(
            "solana_rbpf::vm=debug,\
             solana_runtime::message_processor=debug,\
             solana_runtime::system_instruction_processor=trace,\
             solana_program_test=info",
        );
    });
}

pub fn mint_account_with_authority(
    mint_authority: COption<Pubkey>,
    program_id: &Pubkey,
) -> AccountSharedData {
    let data = if *program_id == spl_token_2022::id() {
        let mut data = [0; spl_token_2022::state::Mint::LEN]; // Correct length for spl_token_2022
        spl_token_2022::state::Mint::pack(
            spl_token_2022::state::Mint {
                mint_authority,
                supply: 100_000_000,
                decimals: 0,
                is_initialized: true,
                ..Default::default()
            },
            &mut data,
        )
        .unwrap();
        data
    } else if *program_id == spl_token::id() {
        let mut data = [0; spl_token::state::Mint::LEN]; // Correct length for spl_token
        spl_token::state::Mint::pack(
            spl_token::state::Mint {
                mint_authority,
                supply: 100_000_000,
                decimals: 0,
                is_initialized: true,
                ..Default::default()
            },
            &mut data,
        )
        .unwrap();
        data
    } else {
        panic!("Wrong program id")
    };

    let mut account = AccountSharedData::new(100_000_000, data.len(), program_id);
    account.set_data_from_slice(&data);
    account
}

pub fn mint_account(program_id: &Pubkey) -> AccountSharedData {
    mint_account_with_authority(COption::None, program_id)
}

pub fn token_account(
    owner: &Pubkey,
    mint: &Pubkey,
    amount: u64,
    program_id: &Pubkey,
) -> AccountSharedData {
    let data = if *program_id == spl_token_2022::id() {
        let mut data = [0; spl_token_2022::state::Account::LEN];
        spl_token_2022::state::Account::pack(
            spl_token_2022::state::Account {
                mint: *mint,
                owner: *owner,
                amount,
                state: spl_token_2022::state::AccountState::Initialized,
                ..Default::default()
            },
            &mut data,
        )
        .unwrap();
        data
    } else if *program_id == spl_token::id() {
        let mut data = [0; spl_token::state::Account::LEN];
        spl_token::state::Account::pack(
            spl_token::state::Account {
                mint: *mint,
                owner: *owner,
                amount,
                state: spl_token::state::AccountState::Initialized,
                ..Default::default()
            },
            &mut data,
        )
        .unwrap();
        data
    } else {
        panic!("Wrong program id")
    };

    let mut account = AccountSharedData::new(100_000_000, data.len(), program_id);
    account.set_data_from_slice(&data);
    account
}
