use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    rc::Rc,
};

use base64::{prelude::BASE64_STANDARD, Engine};
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use solana_accounts_db::blockhash_queue::BlockhashQueue;
use solana_compute_budget::compute_budget::ComputeBudget;
use solana_program::{
    bpf_loader_upgradeable,
    clock::{Epoch, Slot},
    fee_calculator::FeeRateGovernor,
    native_token::LAMPORTS_PER_SOL,
    program_pack::Pack,
    rent::Rent,
    system_instruction, system_program,
};
use solana_sdk::{
    account::{Account, AccountSharedData, WritableAccount},
    epoch_info::EpochInfo,
    feature_set::FeatureSet,
    fee::FeeStructure,
    hash::Hash,
    pubkey::Pubkey,
    reserved_account_keys::ReservedAccountKeys,
    signature::{read_keypair_file, Keypair, Signer},
    signer::keypair::keypair_from_seed,
    transaction::Transaction,
};
use solana_transaction_status::UiTransactionEncoding;
use sov_address::FromVmAddress;
use sov_bank::Amount;
use sov_mock_da::MockDaSpec;
use sov_modules_api::{
    transaction::{PriorityFeeBips, TxDetails},
    CryptoSpec, Spec,
};
use sov_test_utils::{generators::Message, MessageGenerator};
use svm_types::{SolanaAddress, SolanaTestSpec};

use crate::SVM;

type S = SolanaTestSpec<MockDaSpec>;
use sov_modules_api::StateWriter;
use sov_state::User;
use spl_associated_token_account::get_associated_token_address_with_program_id;

// These accounts are hardcoded in the benchmark svm.json genesis files for tests
// (e.g. test-data/genesis/stf-tests, test-data/genesis/benchmark)

// corresponding pubkey is AVxHjnUapzK8C3hQuiyCz7R22bag2CWGNor9v6YZQuJh
const PAYER_SEED: &[u8; 32] = b"random_32_bytes_seed_for_testing";
// corresponding pubkey is C7zRFydqurVQArmveAKnB11j6WHkFERyH4FYMzSQDRMh
const RECEIVER_SEED: &[u8; 32] = b"another_32_bytes_string_for_test";
// corresponding pubkey is 7kGwm4g3oSFUT9HbNqz3ovVK4qShk9AESEW8oBrjPwJK
const ALICE_SEED: &[u8; 32] = b"This is the seed for alice!!!!!!";
// corresponding pubkey is HsNWbHYHfXMxBs6UJoKQm9Y5gcrPA2BJgXn2VHFawgAJ
const BOB_SEED: &[u8; 32] = b"This is the seed for bob's acct!";

pub fn get_test_payer() -> Pubkey {
    keypair_from_seed(PAYER_SEED).unwrap().pubkey()
}

pub fn get_test_receiver() -> Pubkey {
    keypair_from_seed(RECEIVER_SEED).unwrap().pubkey()
}

/// Prepares a Solana transaction for a SOL transfer instruction.
/// Returns a base58-encoded string representation of the prepared transaction.
pub fn prepare_sol_transfer_transaction(
    payer: &Keypair,
    receiver: &Pubkey,
    lamports: u64,
    blockhash: Hash,
) -> Transaction {
    let instruction = system_instruction::transfer(&payer.pubkey(), receiver, lamports);

    Transaction::new_signed_with_payer(&[instruction], Some(&payer.pubkey()), &[payer], blockhash)
}

/// Prepares a Solana transaction for a create_account instruction.
/// Returns a base58-encoded string representation of the prepared transaction.
pub fn prepare_new_account_transaction(
    payer: &Keypair,
    new_account: &Keypair,
    rent: u64,
    blockhash: Hash,
) -> Transaction {
    let instruction = system_instruction::create_account(
        &payer.pubkey(),
        &new_account.pubkey(),
        rent,
        0,
        &system_program::id(),
    );

    Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[payer, new_account],
        blockhash,
    )
}

pub fn set_svm_variables(
    svm: &mut SVM<S>,
    blockhash: &Hash,
    working_set: &mut impl StateWriter<User>,
) {
    let queue = {
        let mut queue = BlockhashQueue::new(150);
        let lamports_per_signature = FeeStructure::default().lamports_per_signature;
        queue.register_hash(blockhash, lamports_per_signature);
        queue
    };

    // we need to set the rent sysvar for rent collector to be constructed correctly
    // (rent collector is needed to initialize the SVM's transaction processing environment)
    let rent_sysvar = bincode::serialize(&Rent::default()).expect("failed to serialize rent");
    svm.accounts
        .set(
            &solana_sdk::sysvar::rent::id(),
            &Account {
                lamports: Rent::default().minimum_balance(rent_sysvar.len()),
                data: rent_sysvar,
                owner: solana_sdk::sysvar::id(),
                executable: false,
                rent_epoch: 0,
            },
            working_set,
        )
        .expect("failed to set rent in SVM accounts");

    svm.blockhash_queue
        .set(&queue, working_set)
        .expect("failed to set blockhash_queue in SVM");

    svm.compute_budget
        .set(&ComputeBudget::default(), working_set)
        .expect("failed to set compute_budget in SVM");
    svm.fee_structure
        .set(&FeeStructure::default(), working_set)
        .expect("failed to set fee_structure in SVM");
    svm.epoch_info
        .set(
            &EpochInfo {
                epoch: Epoch::default(),
                slot_index: Slot::default(),
                slots_in_epoch: 432_000,
                absolute_slot: Slot::default(),
                block_height: 0,
                transaction_count: None,
            },
            working_set,
        )
        .expect("failed to set epoch_info in SVM");
    svm.fee_rate_governor
        .set(&FeeRateGovernor::default().into(), working_set)
        .expect("failed to set fee_rate_governor during genesis");
    svm.feature_set
        .set(&FeatureSet::all_enabled(), working_set)
        .expect("failed to set feature_set during genesis");

    let reserved_account_keys: BTreeSet<Pubkey> =
        ReservedAccountKeys::default().active.into_iter().collect();
    svm.reserved_account_keys
        .set(&reserved_account_keys, working_set)
        .expect("failed to set reserved_account_keys during genesis");
}

/// Returns Keypair from program-keypair .json file
pub fn load_program_keypair<P: AsRef<Path>>(path: P) -> Keypair {
    let mut absolute_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    absolute_path.pop(); // Go up one directory from 'crates/svm'
    absolute_path.push(path);

    // Read the keypair from the JSON file
    read_keypair_file(absolute_path).expect("Failed to read keypair file") // Return the public key (program ID)
}

/// Counter for counter-program tests
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Counter {
    pub count: u64,
}

/// Returns account owned by counter program with count of 0
pub fn counter_account(owner: &Pubkey) -> AccountSharedData {
    let counter = Counter { count: 0 };
    let data = to_vec(&counter).expect("expected to serialize counter into data");
    let mut account = AccountSharedData::new(LAMPORTS_PER_SOL, data.len(), &system_program::id());
    account.set_data_from_slice(&data);
    account.set_owner(*owner);
    account
}

/// Creates a vector of transactions required to deploy a program
/// The logic is based on https://solana.com/docs/programs/deploying#how-object-object-works
pub fn create_deploy_program_transactions(
    payer: &Keypair,
    program_keypair: &Keypair,
    program_authority: &Keypair,
    bytecode: Vec<u8>,
    buffer_lamports: u64,
    recent_blockhash: Hash,
) -> anyhow::Result<Vec<Transaction>> {
    let payer_pubkey = payer.pubkey();
    let program_pubkey = program_keypair.pubkey();
    let program_authority_pubkey = program_authority.pubkey();

    // Derive the buffer account keypair
    let buffer_keypair = Keypair::new();
    let buffer_pubkey = buffer_keypair.pubkey();

    // Create the buffer account
    let create_buffer_instructions = bpf_loader_upgradeable::create_buffer(
        &payer_pubkey,
        &buffer_pubkey,
        &program_authority_pubkey,
        buffer_lamports,
        bytecode.len(),
    )?;

    // Create the initial transaction to create the buffer
    let create_buffer_tx = Transaction::new_signed_with_payer(
        &create_buffer_instructions,
        Some(&payer_pubkey),
        &[payer, &buffer_keypair],
        recent_blockhash,
    );

    // Write the bytecode in chunks and create write transactions
    let chunk_size = 1024;
    let mut offset = 0;
    let mut transactions = vec![create_buffer_tx]; // Start with the buffer creation transaction

    for chunk in bytecode.chunks(chunk_size) {
        let write_instruction = bpf_loader_upgradeable::write(
            &buffer_pubkey,
            &program_authority_pubkey,
            offset,
            chunk.to_vec(),
        );
        offset += chunk.len() as u32;

        let transaction = Transaction::new_signed_with_payer(
            &[write_instruction],
            Some(&payer_pubkey),
            &[payer, program_authority],
            recent_blockhash,
        );
        transactions.push(transaction);
    }

    // Finalize deployment with the buffer account
    let finalize_instructions = bpf_loader_upgradeable::deploy_with_max_program_len(
        &payer_pubkey,
        &program_pubkey,
        &buffer_pubkey,
        &program_authority_pubkey,
        Rent::default().minimum_balance(bytecode.len()),
        bytecode.len(),
    )?;

    let finalize_transaction = Transaction::new_signed_with_payer(
        &finalize_instructions,
        Some(&payer_pubkey),
        &[payer, program_keypair, program_authority],
        recent_blockhash,
    );

    // Add the finalize transaction
    transactions.push(finalize_transaction);

    Ok(transactions)
}

pub struct SvmMessageGenerator<S: Spec>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    pub txs: Vec<Transaction>,
    pub sender: <<S as Spec>::CryptoSpec as CryptoSpec>::PrivateKey,
}

impl<S: Spec> SvmMessageGenerator<S>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    pub fn generate_sample_sol_transactions(
        create_count: usize,
        transfer_count: usize,
        blockhash: Hash,
        sender: <<S as Spec>::CryptoSpec as CryptoSpec>::PrivateKey,
    ) -> Self {
        let payer = keypair_from_seed(PAYER_SEED).unwrap();

        let mut txs: Vec<Transaction> = Vec::with_capacity(create_count + transfer_count);
        for _ in 0..create_count {
            let new_acct = Keypair::new();
            let tx = prepare_new_account_transaction(&payer, &new_acct, 1_000_000, blockhash);
            txs.push(tx);
        }

        let receiver = get_test_receiver();
        for _ in 0..transfer_count {
            let tx = prepare_sol_transfer_transaction(&payer, &receiver, 1, blockhash);
            txs.push(tx);
        }

        Self { txs, sender }
    }

    // Creates a list of complex SPL token transactions
    pub fn generate_sample_spl_transactions(
        tx_count: usize,
        blockhash: Hash,
        sender: <<S as Spec>::CryptoSpec as CryptoSpec>::PrivateKey,
    ) -> Self {
        let payer = keypair_from_seed(PAYER_SEED).unwrap();
        let alice = keypair_from_seed(ALICE_SEED).unwrap();
        let bob = keypair_from_seed(BOB_SEED).unwrap();

        let spl_token_2022_id = spl_token_2022::id();

        let mut txs: Vec<Transaction> = Vec::with_capacity(tx_count);

        for _ in 0..tx_count {
            let minter = Keypair::new();
            let mint_auth = Keypair::new();

            let alice_ata_pubkey = get_associated_token_address_with_program_id(
                &alice.pubkey(),
                &minter.pubkey(),
                &spl_token_2022_id,
            );
            let bob_ata_pubkey = get_associated_token_address_with_program_id(
                &bob.pubkey(),
                &minter.pubkey(),
                &spl_token_2022_id,
            );

            let create_mint = system_instruction::create_account(
                &payer.pubkey(),  // Payer (who funds the mint account creation)
                &minter.pubkey(), // The new mint account to be created
                10_000_000,
                spl_token::state::Mint::LEN as u64,
                &spl_token_2022_id, // SPL Token Program ID (account owner)
            );
            let initialize_mint = spl_token_2022::instruction::initialize_mint(
                &spl_token_2022_id,
                &minter.pubkey(),
                &mint_auth.pubkey(),
                None, // No freeze authority
                6,    // Number of decimals (e.g., 6 decimals for fungible tokens)
            )
            .unwrap();
            let create_alice_ata =
                spl_associated_token_account::instruction::create_associated_token_account(
                    &payer.pubkey(),
                    &alice.pubkey(),
                    &minter.pubkey(),
                    &spl_token_2022_id,
                );
            let create_bob_ata =
                spl_associated_token_account::instruction::create_associated_token_account(
                    &payer.pubkey(),
                    &bob.pubkey(),
                    &minter.pubkey(),
                    &spl_token_2022_id,
                );
            let close_alice_ata = spl_token_2022::instruction::close_account(
                &spl_token_2022_id,
                &alice_ata_pubkey,
                &payer.pubkey(),
                &alice.pubkey(),
                &[],
            )
            .unwrap();
            let close_bob_ata = spl_token_2022::instruction::close_account(
                &spl_token_2022_id,
                &bob_ata_pubkey,
                &payer.pubkey(),
                &bob.pubkey(),
                &[],
            )
            .unwrap();

            let tx = Transaction::new_signed_with_payer(
                &[
                    create_mint,
                    initialize_mint,
                    create_alice_ata.clone(),
                    create_bob_ata,
                    close_alice_ata.clone(),
                    close_bob_ata,
                    create_alice_ata,
                    close_alice_ata,
                ],
                Some(&payer.pubkey()),
                &[&payer, &minter, &alice, &bob],
                blockhash,
            );

            txs.push(tx);
        }

        Self { txs, sender }
    }

    pub fn generate_default_sample_transactions(
        blockhash: Hash,
        sender: <<S as Spec>::CryptoSpec as CryptoSpec>::PrivateKey,
    ) -> Self {
        Self::generate_sample_sol_transactions(2, 2, blockhash, sender)
    }

    /// Creates a `transfer` transaction from a random payer to the test receiver.
    ///
    /// We use the test receiver (rather than a random one) to isolate and test a single
    /// invalid case: the payer doesn't have enough funds.
    ///
    /// A preceding `create_account` transaction is included to verify that the batch can
    /// contain both successful and failed transactions.
    pub fn generate_invalid_transfer_transaction(
        blockhash: Hash,
        sender: <<S as Spec>::CryptoSpec as CryptoSpec>::PrivateKey,
    ) -> Self {
        // Accounts
        let payer = keypair_from_seed(PAYER_SEED).unwrap(); // Pays for the first `create_account` transaction.
        let receiver = keypair_from_seed(RECEIVER_SEED).unwrap(); // The ultimate recipient.
        let account_with_insufficient_funds = Keypair::new(); // Attempts to transfer more funds than it holds.

        // Transactions
        let create_account_tx = prepare_new_account_transaction(
            &payer,
            &account_with_insufficient_funds,
            1_000_000,
            blockhash,
        );

        let invalid_transfer_tx = prepare_sol_transfer_transaction(
            &account_with_insufficient_funds,
            &receiver.pubkey(),
            2_000_000, // Will fail because the payer (`account_with_insufficient_funds`) doesn't have that many lamports
            blockhash,
        );

        Self {
            txs: vec![create_account_tx, invalid_transfer_tx],
            sender,
        }
    }
}

impl<S: Spec> MessageGenerator for SvmMessageGenerator<S>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    type Module = SVM<S>;
    type Spec = S;

    fn create_messages(
        &self,
        chain_id: u64,
        max_priority_fee_bips: PriorityFeeBips,
        max_fee: Amount,
        gas_usage: Option<<Self::Spec as Spec>::Gas>,
    ) -> Vec<Message<Self::Spec, Self::Module>> {
        let mut messages = Vec::<Message<Self::Spec, Self::Module>>::new();

        for (generation, tx) in self.txs.iter().enumerate() {
            let msg: Message<Self::Spec, Self::Module> = Message {
                sender_key: Rc::new(self.sender.clone()),
                content: borsh::to_vec(tx).unwrap(),
                details: TxDetails {
                    chain_id,
                    max_priority_fee_bips,
                    max_fee,
                    gas_limit: gas_usage.clone(),
                },
                generation: generation as u64,
            };
            messages.push(msg);
        }

        messages
    }
}

/// Helper function to serialize and encode a value.
/// Simplified version of https://github.com/anza-xyz/agave/blob/fa620250ab962feb9bd8b1977affe14b94302291/rpc-client/src/nonblocking/rpc_client.rs#L4642
pub fn serialize_and_encode<T>(input: &T, encoding: UiTransactionEncoding) -> anyhow::Result<String>
where
    T: serde::ser::Serialize,
{
    let serialized =
        bincode::serialize(input).map_err(|e| anyhow::anyhow!("Serialization failed: {e}"))?;
    let encoded = match encoding {
        UiTransactionEncoding::Base58 => solana_sdk::bs58::encode(serialized).into_string(),
        UiTransactionEncoding::Base64 => BASE64_STANDARD.encode(serialized),
        _ => {
            anyhow::bail!("unsupported encoding: {encoding}. Supported encodings: base58, base64")
        }
    };
    Ok(encoded)
}
