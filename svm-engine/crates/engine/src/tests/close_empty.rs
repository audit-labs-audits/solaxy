use std::collections::{BTreeMap, HashSet};

use solana_sdk::{
    account::{Account, ReadableAccount},
    clock::{Epoch, MAX_PROCESSING_AGE, Slot},
    hash::Hash,
    reserved_account_keys::ReservedAccountKeys,
    signature::Keypair,
    signer::Signer,
    system_instruction, system_program,
    transaction::Transaction,
};
use solana_svm::transaction_processor::TransactionBatchProcessor;

use super::generate_keypairs;
use crate::{
    Engine,
    storage::{AccountsDB, SVMStorage, SvmBankForks},
    verification::{get_transaction_check_results, sanitize_and_verify_tx},
};

fn prepare_close_empty_tx(payer: &Keypair, new: &Keypair, recepient: &Keypair) -> Transaction {
    let payer_pubkey = payer.pubkey();
    let new_pubkey = new.pubkey();
    let recepient_pubkey = recepient.pubkey();

    let create_account = solana_sdk::system_instruction::create_account(
        &payer_pubkey,
        &new_pubkey,
        10_000_000_000,
        0,
        &system_program::id(),
    );

    let transfer = system_instruction::transfer(&new_pubkey, &recepient_pubkey, 10_000_000_000);

    Transaction::new_signed_with_payer(
        &[create_account, transfer],
        Some(&payer.pubkey()),
        &[&payer, &new],
        Hash::default(),
    )
}

pub fn run_close_empty_scenario() -> eyre::Result<()> {
    let keypairs = generate_keypairs(3);
    let (payer, recepient, new) = (&keypairs[0], &keypairs[1], &keypairs[2]);
    let system_program_id = system_program::id();

    let accounts = BTreeMap::from_iter([
        (
            payer.pubkey(),
            Account::new(1_000_000_000_000, 0, &system_program_id),
        ),
        (
            recepient.pubkey(),
            Account::new(1_000_000_000, 0, &system_program_id),
        ),
    ]);

    let tx = prepare_close_empty_tx(payer, new, recepient);

    let db = AccountsDB::new(accounts);
    let processor = TransactionBatchProcessor::<SvmBankForks>::new(
        Slot::default(),
        Epoch::default(),
        HashSet::default(),
    );
    let engine = Engine::builder().db(db).processor(&processor).build();
    engine.initialize_cache();
    engine.initialize_transaction_processor();

    let san_tx = sanitize_and_verify_tx(&tx, &ReservedAccountKeys::default().active)?;
    let check_results = get_transaction_check_results(&[san_tx.clone()], MAX_PROCESSING_AGE);

    engine.load_execute_and_commit_transactions(&[san_tx], check_results)?;

    // validate that the new account is closed
    let new_account = engine.get_account(&new.pubkey())?;

    assert!(
        new_account.is_none(),
        "New account should have been created and closed immediately"
    );

    let recepient_account = engine.get_account(&recepient.pubkey())?;

    assert_eq!(
        recepient_account.map(|account| account.lamports()),
        Some(1_000_000_000 + 10_000_000_000),
        "The recepient account should have received 10_000_000_000 lamports"
    );

    Ok(())
}
