use std::collections::{BTreeMap, HashSet};

use solana_sdk::{
    account::Account,
    bpf_loader, bpf_loader_upgradeable,
    clock::{Epoch, MAX_PROCESSING_AGE, Slot},
    program_option::COption,
    program_pack::Pack,
    reserved_account_keys::ReservedAccountKeys,
    signer::Signer,
    system_program,
};
use solana_svm::transaction_processor::TransactionBatchProcessor;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token_2022::state::{Account as TokenAccount, Mint};
use svm_test_utils::txs::prepare_spl_tx;

use super::generate_keypairs;
use crate::{
    Engine,
    programs::Program,
    storage::{AccountsDB, SVMStorage, SvmBankForks},
    verification::{get_transaction_check_results, sanitize_and_verify_tx},
};

pub static SPL_PROGRAMS: &[Program] = &[
    Program {
        program_id: spl_associated_token_account::id(),
        owner_program: bpf_loader::id(),
        bytecode: include_bytes!("../../../bytecode/spl_associated_token_account.so"),
    },
    Program {
        program_id: spl_token_2022::id(),
        owner_program: bpf_loader_upgradeable::id(),
        bytecode: include_bytes!("../../../bytecode/spl_token_2022.so"),
    },
];

fn unpack_token_account_data(account_data: &[u8]) -> TokenAccount {
    // Ensure the data is the correct length for a Token Account
    let trimmed_data = &account_data[..TokenAccount::LEN];

    // Unpack the trimmed data into a TokenAccount struct
    TokenAccount::unpack(trimmed_data).expect("Failed to unpack token account data")
}

pub fn run_spl_scenario() -> eyre::Result<()> {
    // set up genesis state
    let mut accounts = BTreeMap::new();
    // add wallet accounts
    let keypairs = generate_keypairs(5);
    let (payer, alice, bob, minter, mint_authority) = (
        &keypairs[0],
        &keypairs[1],
        &keypairs[2],
        &keypairs[3],
        &keypairs[4],
    );

    let system_program_id = system_program::id();

    accounts.extend([
        (
            payer.pubkey(),
            Account::new(1_000_000_000, 0, &system_program_id),
        ),
        (
            alice.pubkey(),
            Account::new(1_000_000, 0, &system_program_id),
        ),
        (bob.pubkey(), Account::new(1_000_000, 0, &system_program_id)),
    ]);

    let tx = prepare_spl_tx(payer, alice, bob, minter, mint_authority);

    let db = AccountsDB::new(accounts);
    // add program accounts
    crate::programs::initialize_programs(&db, SPL_PROGRAMS).expect("Failed to add SPL programs");
    let processor = TransactionBatchProcessor::<SvmBankForks>::new(
        Slot::default(),
        Epoch::default(),
        HashSet::default(),
    );
    let engine = Engine::builder().db(db).processor(&processor).build();
    engine.initialize_cache();
    engine.initialize_transaction_processor();
    engine.fill_cache(
        &SPL_PROGRAMS
            .iter()
            .map(|p| p.program_id)
            .collect::<Vec<_>>(),
    );

    let spl_token_2022_id = spl_token_2022::id();
    let san_tx = sanitize_and_verify_tx(&tx, &ReservedAccountKeys::default().active)?;
    let check_results = get_transaction_check_results(&[san_tx.clone()], MAX_PROCESSING_AGE);

    engine.load_execute_and_commit_transactions(&[san_tx], check_results)?;

    // validate mint data
    let mint_account = engine
        .get_account(&minter.pubkey())?
        .ok_or_else(|| eyre::eyre!("mint account not found"))?;
    assert_eq!(mint_account.owner, spl_token_2022_id);
    let mint_data = Mint::unpack(&mint_account.data).expect("failed to unpack mint account data");
    assert_eq!(mint_data.decimals, 6, "expected 6 decimals");
    assert_eq!(
        mint_data.mint_authority,
        COption::Some(mint_authority.pubkey()),
        "mint authority mismatch"
    );

    // validate alice data
    let alice_ata_pubkey = get_associated_token_address_with_program_id(
        &alice.pubkey(),
        &minter.pubkey(),
        &spl_token_2022_id,
    );
    let alice_account = engine
        .get_account(&alice_ata_pubkey)?
        .ok_or_else(|| eyre::eyre!("alice account not found"))?;
    let alice_token_account_data = unpack_token_account_data(&alice_account.data);
    // minted 50 and transferred 10 to bob: 100 - 10 = 40
    assert_eq!(
        alice_token_account_data.amount, 90,
        "expected alice to have received 40 tokens"
    );

    // validate bob data
    let bob_ata_pubkey = get_associated_token_address_with_program_id(
        &bob.pubkey(),
        &minter.pubkey(),
        &spl_token_2022_id,
    );
    let bob_account = engine.get_account(&bob_ata_pubkey)?;

    assert!(
        bob_account.is_none(),
        "expected bob to have closed his account"
    );

    Ok(())
}
