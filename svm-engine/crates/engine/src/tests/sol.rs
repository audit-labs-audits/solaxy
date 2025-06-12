use std::collections::{BTreeMap, HashSet};

use solana_sdk::{
    account::{Account, ReadableAccount},
    clock::{Epoch, MAX_PROCESSING_AGE, Slot},
    reserved_account_keys::ReservedAccountKeys,
    signer::Signer,
    system_program,
};
use solana_svm::transaction_processor::TransactionBatchProcessor;
use svm_test_utils::txs::prepare_sol_tx;

use super::generate_keypairs;
use crate::{
    Engine,
    storage::{AccountsDB, SVMStorage, SvmBankForks},
    verification::{get_transaction_check_results, sanitize_and_verify_tx},
};

pub fn run_sol_scenario() -> eyre::Result<()> {
    let keypairs = generate_keypairs(4);
    let (payer, alice, bob, source) = (&keypairs[0], &keypairs[1], &keypairs[2], &keypairs[3]);
    let system_program_id = system_program::id();

    let accounts = BTreeMap::from_iter([
        (
            payer.pubkey(),
            Account::new(1_000_000_000_000, 0, &system_program_id),
        ),
        (
            alice.pubkey(),
            Account::new(1_000_000_000, 0, &system_program_id),
        ),
        (
            bob.pubkey(),
            Account::new(1_000_000_000, 0, &system_program_id),
        ),
    ]);

    let tx = prepare_sol_tx(payer, alice, bob, source);

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

    // validate that the source account was created
    let source_account = engine
        .get_account(&source.pubkey())?
        .ok_or_else(|| eyre::eyre!("source account not found"))?;

    assert_eq!(
        source_account.owner,
        system_program::id(),
        "Source account should have been created"
    );

    assert_eq!(
        source_account.lamports,
        10_000_000_000 - 123,
        "Source account should have transferred 123 lamports"
    );

    let alice_account = engine.get_account(&alice.pubkey())?;

    assert_eq!(
        alice_account.map(|account| account.lamports()),
        Some(1_000_000_000 + 123 - 45),
        "Alice account should have received 123 lamports and then transferred 45"
    );

    let bob_account = engine.get_account(&bob.pubkey())?;

    assert_eq!(
        bob_account.map(|account| account.lamports()),
        Some(1_000_000_000 + 45),
        "Bob account should have received 45 lamports"
    );

    Ok(())
}
