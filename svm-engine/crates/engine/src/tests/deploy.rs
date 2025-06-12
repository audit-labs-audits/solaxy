use std::collections::{BTreeMap, HashSet};

use solana_sdk::{
    account::Account,
    bpf_loader,
    bpf_loader_upgradeable::{self, UpgradeableLoaderState, get_program_data_address},
    clock::{Epoch, MAX_PROCESSING_AGE, Slot},
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    reserved_account_keys::ReservedAccountKeys,
    signature::{Keypair, keypair_from_seed},
    signer::Signer,
    system_program,
    transaction::{SanitizedTransaction, Transaction},
};
use solana_svm::transaction_processor::TransactionBatchProcessor;
use svm_test_utils::txs::prepare_deploy_program_transactions;

use crate::{
    Engine,
    programs::Program,
    storage::{AccountsDB, SVMStorage, SvmBankForks},
    tests::generate_keypairs,
    verification::{get_transaction_check_results, sanitize_and_verify_tx},
};

fn get_hello_world_keypair() -> Keypair {
    keypair_from_seed(b"hello_world_program_keypair_for_test").unwrap()
}

fn get_simple_programs() -> Vec<Program> {
    vec![Program {
        program_id: get_hello_world_keypair().pubkey(),
        owner_program: bpf_loader::id(),
        bytecode: include_bytes!("../../../bytecode/hello_world.so"),
    }]
}

pub fn run_deploy_and_call_scenario() -> eyre::Result<()> {
    // set up genesis accounts
    let payer = &generate_keypairs(1)[0];

    // Load the program keypair, which will own the program account
    let program_keypair = get_hello_world_keypair();
    let program_pubkey = program_keypair.pubkey();

    // Derive the program data address (PDA)
    let program_data_pubkey = get_program_data_address(&program_pubkey);

    // Load the bytecode of the hello world program
    let programs = get_simple_programs();
    let hello_world_program = programs.first().unwrap();

    let mut genesis_accounts: BTreeMap<Pubkey, Account> = BTreeMap::new();
    genesis_accounts.insert(
        payer.pubkey(),
        Account::new(1_000_000_000, 0, &system_program::id()),
    );

    let transactions = prepare_deploy_program_transactions(
        payer,
        &program_keypair,
        payer,
        hello_world_program.bytecode.to_vec(),
        Hash::default(),
    )
    .unwrap();

    // Transaction: created to interact with hello_world program
    let hello_world_instruction = Instruction::new_with_bincode(
        program_pubkey,
        &(), // To trigger msg!(hello world) line in the program no instruction is required
        vec![AccountMeta::new(payer.pubkey(), true)],
    );
    let hello_world_tx = Transaction::new_signed_with_payer(
        &[hello_world_instruction],
        Some(&payer.pubkey()),
        &[&payer],
        Hash::default(),
    );

    let db = AccountsDB::new(genesis_accounts);
    let programs = db.get_program_accounts();
    let processor = TransactionBatchProcessor::<SvmBankForks>::new(
        Slot::default(),
        Epoch::default(),
        HashSet::default(),
    );
    let engine = Engine::builder().db(db).processor(&processor).build();
    engine.initialize_cache();
    engine.initialize_transaction_processor();
    engine.fill_cache(&programs);

    let sanitized_transactions: Vec<SanitizedTransaction> = transactions
        .iter()
        .filter_map(|tx| sanitize_and_verify_tx(tx, &ReservedAccountKeys::default().active).ok())
        .collect();

    let check_results = get_transaction_check_results(&sanitized_transactions, MAX_PROCESSING_AGE);

    engine
        .load_execute_and_commit_transactions(&sanitized_transactions, check_results)
        .expect("failed to execute and commit tx");

    // Interact with hello world program
    let san_tx = sanitize_and_verify_tx(&hello_world_tx, &ReservedAccountKeys::default().active)?;
    let check_results = get_transaction_check_results(&[san_tx.clone()], MAX_PROCESSING_AGE);
    engine
        .load_execute_and_commit_transactions(&[san_tx], check_results)
        .expect("failed to execute and commit tx");

    // verify program account was created with data
    let program_account = engine
        .get_account(&program_pubkey)?
        .ok_or_else(|| eyre::eyre!("program account not found"))?;
    assert!(
        !program_account.data.is_empty(),
        "expect program account should contain program data account info"
    );

    // The program account must reference the `ProgramData` account where the bytecode is stored.
    // Verify that the `programdata_address` in the program account matches the expected `program_data_pubkey`.
    let program_account_data: UpgradeableLoaderState = bincode::deserialize(&program_account.data)
        .expect("Failed to deserialize hello world program account data");
    if let UpgradeableLoaderState::Program {
        programdata_address,
    } = program_account_data
    {
        assert_eq!(
            programdata_address, program_data_pubkey,
            "Expected programdata_address to match program_data_pubkey derived for program's pubkey"
        );
    }

    // Verify that program data account was created with data
    let program_data_account = engine
        .get_account(&program_data_pubkey)?
        .ok_or_else(|| eyre::eyre!("program data account not found"))?;
    assert!(
        !program_data_account.data.is_empty(),
        "expect program data account should contain owner and program bytecode info"
    );

    let program_data: UpgradeableLoaderState = bincode::deserialize(&program_data_account.data)
        .expect("Failed to deserialize hello world program account data");

    // Parse the account data as a ProgramData layout
    if let UpgradeableLoaderState::ProgramData {
        upgrade_authority_address,
        ..
    } = program_data
    {
        assert_eq!(
            upgrade_authority_address.unwrap(),
            payer.pubkey(),
            "expect upgrade authority to be payer account"
        );
    }

    Ok(())
}

pub fn close_program_scenario() -> eyre::Result<()> {
    // set up genesis accounts
    let payer = &generate_keypairs(1)[0];

    // Load the program keypair, which will own the program account
    let program_keypair = get_hello_world_keypair();
    let program_pubkey = program_keypair.pubkey();

    // Derive the program data address (PDA)
    let program_data_pubkey = get_program_data_address(&program_pubkey);

    // Load the bytecode of the hello world program
    let programs = get_simple_programs();
    let hello_world_program = programs.first().unwrap();

    let mut genesis_accounts: BTreeMap<Pubkey, Account> = BTreeMap::new();
    genesis_accounts.insert(
        payer.pubkey(),
        Account::new(1_000_000_000, 0, &system_program::id()),
    );

    // Program data
    let mut data = bincode::serialize(&UpgradeableLoaderState::ProgramData {
        upgrade_authority_address: Some(payer.pubkey()),
        slot: 0,
    })?;
    data.extend_from_slice(hello_world_program.bytecode);

    // Program Account
    genesis_accounts.insert(
        program_pubkey,
        Account {
            lamports: 1_000_000,
            data: bincode::serialize(&UpgradeableLoaderState::Program {
                programdata_address: program_data_pubkey,
            })?,
            owner: bpf_loader_upgradeable::id(),
            executable: true,
            rent_epoch: 0,
        },
    );

    // Program Data Account
    genesis_accounts.insert(
        program_data_pubkey,
        Account {
            lamports: 10_000_000,
            data,
            owner: bpf_loader_upgradeable::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    let db = AccountsDB::new(genesis_accounts);
    let programs = db.get_program_accounts();
    let processor = TransactionBatchProcessor::<SvmBankForks>::new(
        Slot::default(),
        Epoch::default(),
        HashSet::default(),
    );
    let engine = Engine::builder().db(db).processor(&processor).build();
    engine.initialize_cache();
    engine.initialize_transaction_processor();
    engine.fill_cache(&programs);

    let close_program_instruction = bpf_loader_upgradeable::close_any(
        &program_data_pubkey,
        &payer.pubkey(),
        Some(&payer.pubkey()),
        Some(&program_pubkey),
    );

    let close_program_tx = Transaction::new_signed_with_payer(
        &[close_program_instruction],
        Some(&payer.pubkey()),
        &[&payer],
        Hash::default(),
    );

    let sanitized_transaction =
        sanitize_and_verify_tx(&close_program_tx, &ReservedAccountKeys::default().active).unwrap();

    let check_results =
        get_transaction_check_results(&[sanitized_transaction.clone()], MAX_PROCESSING_AGE);

    engine.update_slot()?;
    engine
        .load_execute_and_commit_transactions(&[sanitized_transaction], check_results)
        .expect("failed to execute and commit tx for closing program account");

    // Verify program account was not purged
    let program_account = engine.get_account(&program_pubkey)?;
    assert!(
        program_account.is_some(),
        "expect program account to be present"
    );

    // Verify that program data account was purged
    let program_data_account = engine.get_account(&program_data_pubkey)?;
    assert!(
        program_data_account.is_none(),
        "expect program data account to be purged"
    );
    Ok(())
}
