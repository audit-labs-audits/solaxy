use solana_program::{
    bpf_loader_upgradeable, bpf_loader_upgradeable::UpgradeableLoaderState, hash::Hash, rent::Rent,
    system_instruction, system_program,
};
use solana_sdk::{
    program_pack::Pack,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token_2022::state::Mint;

/// Prepares a transaction that funds a new account and transfers SOL between participants.
pub fn prepare_sol_tx(
    payer: &Keypair,
    alice: &Keypair,
    bob: &Keypair,
    source: &Keypair,
) -> Transaction {
    let payer_pubkey = payer.pubkey();
    let source_pubkey = source.pubkey();
    let alice_pubkey = alice.pubkey();
    let bob_pubkey = bob.pubkey();

    let create_account = solana_sdk::system_instruction::create_account(
        &payer_pubkey,
        &source_pubkey,
        10_000_000_000,
        0,
        &system_program::id(),
    );

    let transfer1 = system_instruction::transfer(&source_pubkey, &alice_pubkey, 123);

    let transfer2 = system_instruction::transfer(&alice_pubkey, &bob_pubkey, 45);

    Transaction::new_signed_with_payer(
        &[create_account, transfer1, transfer2],
        Some(&payer.pubkey()),
        &[&payer, &source, &alice],
        Hash::default(),
    )
}

/// Prepares a transactions that initializes a mint, sets up associated token accounts, mints tokens, transfers,
/// burns, and closes a token account
pub fn prepare_spl_tx(
    payer: &Keypair,
    alice: &Keypair,
    bob: &Keypair,
    minter: &Keypair,
    mint_authority: &Keypair,
) -> Transaction {
    let payer_pubkey = payer.pubkey();
    // let mint_authority = Keypair::new();
    // let minter = Keypair::new();
    let spl_token_2022_id = spl_token_2022::id();

    let minter_pubkey = minter.pubkey();
    let mint_authority_pubkey = mint_authority.pubkey();
    let alice_pubkey = alice.pubkey();
    let bob_pubkey = bob.pubkey();

    let alice_ata_pubkey = get_associated_token_address_with_program_id(
        &alice_pubkey,
        &minter_pubkey,
        &spl_token_2022_id,
    );

    let bob_ata_pubkey = get_associated_token_address_with_program_id(
        &bob_pubkey,
        &minter_pubkey,
        &spl_token_2022_id,
    );

    let create_mint = solana_sdk::system_instruction::create_account(
        &payer_pubkey,  // Payer (who funds the mint account creation)
        &minter_pubkey, // The new mint account to be created
        10_000_000,
        Mint::LEN as u64,   // Mint account size
        &spl_token_2022_id, // SPL Token Program ID (account owner)
    );

    let initialize_mint = spl_token_2022::instruction::initialize_mint(
        &spl_token_2022_id,
        &minter_pubkey,
        &mint_authority_pubkey,
        None, // No freeze authority
        6,    // Number of decimals (e.g., 6 decimals for fungible tokens)
    )
    .unwrap();

    let create_alice_ata =
        spl_associated_token_account::instruction::create_associated_token_account(
            &payer_pubkey,
            &alice_pubkey,
            &minter_pubkey,
            &spl_token_2022_id,
        );

    let mint_to_alice = spl_token_2022::instruction::mint_to_checked(
        &spl_token_2022_id,
        &minter_pubkey,
        &alice_ata_pubkey,
        &mint_authority_pubkey,
        &[],
        100, // Amount to mint (100 tokens, given 0 decimals)
        6,
    )
    .unwrap();

    let create_bob_ata = spl_associated_token_account::instruction::create_associated_token_account(
        &payer_pubkey,
        &bob_pubkey,
        &minter_pubkey,
        &spl_token_2022_id,
    );

    let transfer_to_bob = spl_token_2022::instruction::transfer_checked(
        &spl_token_2022_id,
        &alice_ata_pubkey,
        &minter_pubkey,
        &bob_ata_pubkey,
        &alice_pubkey,
        &[],
        10, // 10 tokens (given 0 decimals)
        6,
    )
    .unwrap();

    // Create burn instruction: burn 10 tokens from bob's account
    let burn_bob = spl_token_2022::instruction::burn_checked(
        &spl_token_2022_id,
        &bob_ata_pubkey,
        &minter_pubkey,
        &bob_pubkey,
        &[],
        10,
        6,
    )
    .unwrap();

    // Create close account instruction: close bob's token account
    let close_bob_ata = spl_token_2022::instruction::close_account(
        &spl_token_2022_id,
        &bob_ata_pubkey,
        &payer_pubkey,
        &bob_pubkey,
        &[],
    )
    .unwrap();

    Transaction::new_signed_with_payer(
        &[
            create_mint,
            initialize_mint,
            create_alice_ata,
            mint_to_alice,
            create_bob_ata,
            transfer_to_bob,
            burn_bob,
            close_bob_ata,
        ],
        Some(&payer.pubkey()),
        &[&payer, &minter, &mint_authority, &alice, &bob],
        Hash::default(),
    )
}

/// Prepares a vector of transactions required to deploy a program
/// The logic is based on https://solana.com/docs/programs/deploying#how-object-object-works
pub fn prepare_deploy_program_transactions(
    payer: &Keypair,
    program_keypair: &Keypair,
    program_authority: &Keypair,
    bytecode: Vec<u8>,
    recent_blockhash: Hash,
) -> eyre::Result<Vec<Transaction>> {
    let payer_pubkey = payer.pubkey();
    let program_pubkey = program_keypair.pubkey();
    let program_authority_pubkey = program_authority.pubkey();

    // Derive the buffer account keypair
    let buffer_keypair = Keypair::new();
    let buffer_pubkey = buffer_keypair.pubkey();
    let buffer_lamports =
        Rent::default().minimum_balance(UpgradeableLoaderState::size_of_buffer(bytecode.len()));

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
