use solana_program::{program_option::COption, program_pack::Pack};
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token_2022::{
    extension::ExtensionType,
    state::{Account as TokenAccount, Mint},
};

mod common;

use common::setup::{
    execute_call_message, get_account_info, mint_account, mint_account_with_authority,
    recent_blockhash, set_up_logger, setup_svm_with_accounts, system_account, token_account,
    TestSvm,
};

#[test]
fn test_init_mint() {
    // Setup: define Solana accounts that will be used in this test.
    let payer = Keypair::new(); // This account will be created during genesis.
    let payer_pubkey = payer.pubkey();
    let mint_authority = Keypair::new(); // Authority for minting tokens
    let minter = Keypair::new(); // This will be the new mint account holding mint data
    let minter_pubkey = minter.pubkey();

    let accounts = &[(payer_pubkey, system_account())];

    // Setup: Run genesis and get a test runner
    let mut runner = setup_svm_with_accounts(accounts);

    let recent_blockhash = recent_blockhash(&runner);

    // Create the mint account
    let create_mint_account = solana_sdk::system_instruction::create_account(
        &payer_pubkey,  // Payer (who funds the mint account creation)
        &minter_pubkey, // The new mint account to be created
        100_000_000,
        Mint::LEN as u64,      // Mint account size
        &spl_token_2022::id(), // SPL Token Program ID (account owner)
    );

    // Initialize the mint account
    let initialize_mint = spl_token_2022::instruction::initialize_mint(
        &spl_token_2022::id(),
        &minter_pubkey,
        &mint_authority.pubkey(),
        None, // No freeze authority
        6,    // Number of decimals (e.g., 6 decimals for fungible tokens)
    )
    .unwrap();

    // Create transaction: create and initialize mint account transaction
    let transaction = Transaction::new_signed_with_payer(
        &[create_mint_account, initialize_mint],
        Some(&payer.pubkey()),
        &[&payer, &minter],
        recent_blockhash,
    );

    // Execute the transaction
    execute_call_message(
        &transaction,
        &mut runner,
        "create_and_initialize_mint_account",
    );

    // Check: created and initialized mint account
    runner.query_state(|state| {
        let mint_account = get_account_info(minter_pubkey, state);

        // Ensure the mint account is owned by the SPL Token program
        assert_eq!(mint_account.owner, spl_token_2022::id());

        // Deserialize the mint account data
        let mint_data =
            Mint::unpack(&mint_account.data).expect("failed to unpack mint account data");

        // Check: the mint account has the correct properties
        assert_eq!(mint_data.decimals, 6, "expected 6 decimals");
        assert_eq!(
            mint_data.mint_authority,
            COption::Some(mint_authority.pubkey()),
            "mint authority mismatch"
        );
        assert_eq!(
            mint_data.freeze_authority,
            COption::None,
            "expected no freeze authority"
        );
    });
}

#[test]
fn test_mint_spl_tokens() {
    // Setup: define Solana accounts that will be used in this test.
    let payer = Keypair::new(); // This account will be created during genesis.
    let payer_pubkey = payer.pubkey();
    let mint_authority = Keypair::new(); // Authority for minting tokens
    let minter = Keypair::new(); // This will be mint account created during genesis
    let minter_pubkey = minter.pubkey();

    let alice = Keypair::new();
    let alice_pubkey = alice.pubkey(); // Alice created during genesis
    let alice_ata_pubkey = get_associated_token_address_with_program_id(
        &alice_pubkey,
        &minter_pubkey,
        &spl_token_2022::id(),
    );

    let accounts = &[
        (payer_pubkey, system_account()),
        (
            minter_pubkey,
            mint_account_with_authority(
                COption::Some(mint_authority.pubkey()),
                &spl_token_2022::id(),
            ),
        ),
        (alice_pubkey, system_account()),
    ];

    // Setup: Run genesis and get a test runner
    let mut runner = setup_svm_with_accounts(accounts);

    // Setup: Create the mint account and initialize it with SPL Token 2022.
    let recent_blockhash = recent_blockhash(&runner);

    // Create alice associated token account
    let create_alice_ata =
        spl_associated_token_account::instruction::create_associated_token_account(
            &payer_pubkey,
            &alice_pubkey,
            &minter_pubkey,
            &spl_token_2022::id(),
        );

    // Create mint instruction
    let mint_to = spl_token_2022::instruction::mint_to_checked(
        &spl_token_2022::id(),
        &minter_pubkey,
        &alice_ata_pubkey,
        &mint_authority.pubkey(),
        &[],
        100, // Amount to mint (100 tokens, given 0 decimals)
        0,
    )
    .unwrap();

    // Create transaction to mint tokens
    let transaction = Transaction::new_signed_with_payer(
        &[create_alice_ata, mint_to],
        Some(&payer.pubkey()),
        &[&payer, &mint_authority],
        recent_blockhash,
    );

    // Execute the transaction
    execute_call_message(&transaction, &mut runner, "create_alice_ata_and_mint");

    // Check: minted 100 token to alice token account
    runner.query_state(|state| {
        let alice_account = get_account_info(alice_ata_pubkey, state);

        assert_eq!(alice_account.owner, spl_token_2022::id());

        let alice_token_account_data = unpack_token_account_data(&alice_account.data);

        // Ensure alice has received 100 tokens from mint
        assert_eq!(
            alice_token_account_data.amount, 100,
            "expected alice to have received 100 tokens"
        );
    });
}

#[test]
fn test_transfer_spl_tokens() {
    // Setup: define Solana accounts that will be used in this test.
    let payer = Keypair::new(); // This account will be created during genesis
    let minter = Keypair::new(); // This will be mint account holding mint data and created during genesis

    let alice = Keypair::new(); // alice token account created during genesis with 100 tokens
    let alice_ata_pubkey = get_associated_token_address_with_program_id(
        &alice.pubkey(),
        &minter.pubkey(),
        &spl_token_2022::id(),
    );

    // alice will transfer tokens to bob's account
    let bob = Keypair::new(); // bob token account created during genesis with 90 tokens
    let bob_ata_pubkey = get_associated_token_address_with_program_id(
        &bob.pubkey(),
        &minter.pubkey(),
        &spl_token_2022::id(),
    );

    let accounts = &[
        (payer.pubkey(), system_account()),
        (minter.pubkey(), mint_account(&spl_token_2022::id())),
        (alice.pubkey(), system_account()),
        (
            alice_ata_pubkey,
            token_account(
                &alice.pubkey(),
                &minter.pubkey(),
                100,
                &spl_token_2022::id(),
            ),
        ),
        (bob.pubkey(), system_account()),
        (
            bob_ata_pubkey,
            token_account(&bob.pubkey(), &minter.pubkey(), 10, &spl_token_2022::id()),
        ),
    ];

    // Setup: Run genesis and get a test runner
    let mut runner = setup_svm_with_accounts(accounts);

    // Create 10 tokens transfer instruction from alice to bob
    let transfer = spl_token_2022::instruction::transfer_checked(
        &spl_token_2022::id(),
        &alice_ata_pubkey,
        &minter.pubkey(),
        &bob_ata_pubkey,
        &alice.pubkey(),
        &[],
        10, // 10 tokens (given 0 decimals)
        0,
    )
    .unwrap();

    // Create transfer transaction
    let transaction = Transaction::new_signed_with_payer(
        &[transfer],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        recent_blockhash(&runner),
    );

    // Execute the transaction
    execute_call_message(&transaction, &mut runner, "transfer_tokens");

    // Check: alice and bob token accounts balances post transfer
    runner.query_state(|state| {
        let alice_token_account = get_account_info(alice_ata_pubkey, state);

        // Deserialize token account data to verify the balance
        let alice_token_data = TokenAccount::unpack(&alice_token_account.data)
            .expect("failed to unpack token account data");

        // alice starts with 100 tokens and transfers 10 tokens to bob => possesses 90 tokens now
        assert_eq!(
            alice_token_data.amount, 90,
            "expected alice to have 90 tokens left"
        );

        let bob_token_account = get_account_info(bob_ata_pubkey, state);

        // Deserialize bob account data to verify the balance
        let bob_token_data = TokenAccount::unpack(&bob_token_account.data)
            .expect("failed to unpack bob token account data");

        // bob has 10 tokens and receives 10 from alice => possess 20 tokens now
        assert_eq!(
            bob_token_data.amount, 20,
            "expected bob to possess 20 tokens"
        );
    });
}

#[test]
fn test_burn_spl_tokens() {
    // Setup: define Solana accounts that will be used in this test.
    let payer = Keypair::new(); // This account will be created during genesis.
    let payer_pubkey = payer.pubkey();
    let minter = Keypair::new(); // This will be the new mint account holding mint data
    let minter_pubkey = minter.pubkey();

    let alice = Keypair::new();
    let alice_pubkey = alice.pubkey();
    let alice_ata_pubkey = get_associated_token_address_with_program_id(
        &alice_pubkey,
        &minter_pubkey,
        &spl_token_2022::id(),
    );

    let accounts = &[
        (payer_pubkey, system_account()),
        (minter_pubkey, mint_account(&spl_token_2022::id())),
        (alice_pubkey, system_account()),
        (
            alice_ata_pubkey,
            token_account(&alice_pubkey, &minter_pubkey, 100, &spl_token_2022::id()),
        ),
    ];

    // Setup: Run genesis and get a test runner
    let mut runner = setup_svm_with_accounts(accounts);

    let recent_blockhash = recent_blockhash(&runner);

    // Create burn instruction: burn alice 50 tokens
    let burn = spl_token_2022::instruction::burn_checked(
        &spl_token_2022::id(),
        &alice_ata_pubkey,
        &minter_pubkey,
        &alice_pubkey,
        &[],
        50,
        0,
    )
    .unwrap();

    // Create a transaction with burn instruction
    let transaction = Transaction::new_signed_with_payer(
        &[burn],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        recent_blockhash,
    );

    // Execute the transaction
    execute_call_message(&transaction, &mut runner, "burn");

    // Check: assert that alice has 50 tokens burned
    runner.query_state(|state| {
        let alice_account = get_account_info(alice_ata_pubkey, state);

        // Deserialize alice token account data to verify the balance
        let alice_token_account_data = TokenAccount::unpack(&alice_account.data)
            .expect("failed to unpack alice token account data");

        // Ensure alice burned 50 tokens and has 50 tokens left: 100 - 50 = 50
        assert_eq!(
            alice_token_account_data.amount, 50,
            "expected alice to have burned 50 tokens"
        );
    });
}

#[test]
fn test_close_spl_token_account() {
    // Setup: define Solana accounts that will be used in this test.
    let payer = Keypair::new(); // This account will be created during genesis.
    let payer_pubkey = payer.pubkey();
    let minter = Keypair::new(); // This will be the new mint account holding mint data
    let minter_pubkey = minter.pubkey();

    let alice = Keypair::new();
    let alice_pubkey = alice.pubkey(); // alice token account created during genesis with 0 tokens
    let alice_ata_pubkey = get_associated_token_address_with_program_id(
        &alice_pubkey,
        &minter_pubkey,
        &spl_token_2022::id(),
    );

    let accounts = &[
        (payer_pubkey, system_account()),
        (minter_pubkey, mint_account(&spl_token_2022::id())),
        (alice_pubkey, system_account()),
        (
            alice_ata_pubkey,
            token_account(&alice_pubkey, &minter_pubkey, 0, &spl_token_2022::id()),
        ),
    ];

    // Setup: Run genesis and get a test runner
    let mut runner = setup_svm_with_accounts(accounts);

    let recent_blockhash = recent_blockhash(&runner);

    // Create close account instruction: close alice token account
    let close_token_account = spl_token_2022::instruction::close_account(
        &spl_token_2022::id(),
        &alice_ata_pubkey, // Token account to be closed
        &payer_pubkey,     // Destination that receives any remaining SOL (payer in this case)
        &alice_pubkey,
        &[],
    )
    .unwrap();
    // Create a transaction with burn instruction
    let transaction = Transaction::new_signed_with_payer(
        &[close_token_account],
        Some(&payer.pubkey()),
        &[&payer, &alice],
        recent_blockhash,
    );

    // Execute the transaction
    execute_call_message(&transaction, &mut runner, "close_token_account");

    // Check: assert that alice has 50 tokens burned
    runner.query_state(|state| {
        let alice_account =
            TestSvm::default().get_account_info(alice_ata_pubkey.into(), None, state);

        // Verify account is closed
        assert!(
            alice_account.unwrap().value.is_none(),
            "expected token account to be closed and purged"
        );
    });
}

// Check spl-token with TransferFee extension enabled
#[test]
fn test_transfer_with_fees() {
    set_up_logger();
    // Setup accounts and keys
    let payer = Keypair::new();
    let mint_authority = Keypair::new();
    let minter = Keypair::new();
    let alice = Keypair::new();
    let bob = Keypair::new();

    let mint_pubkey = minter.pubkey();
    let alice_pubkey = alice.pubkey();
    let bob_pubkey = bob.pubkey();

    let alice_ata_pubkey = get_associated_token_address_with_program_id(
        &alice_pubkey,
        &mint_pubkey,
        &spl_token_2022::id(),
    );
    let bob_ata_pubkey = get_associated_token_address_with_program_id(
        &bob_pubkey,
        &mint_pubkey,
        &spl_token_2022::id(),
    );

    // Setup accounts in genesis
    let accounts = &[
        (payer.pubkey(), system_account()),
        (alice_pubkey, system_account()),
        (bob_pubkey, system_account()),
    ];

    // Setup runner and recent blockhash
    let mut runner = setup_svm_with_accounts(accounts);
    let recent_blockhash = recent_blockhash(&runner);

    let extensions = vec![ExtensionType::TransferFeeConfig];
    let required_mint_len = ExtensionType::try_calculate_account_len::<Mint>(&extensions).unwrap();

    // Create mint account
    let create_mint_account = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &mint_pubkey,
        100_000_000,
        required_mint_len as u64,
        &spl_token_2022::id(),
    );

    // Initialize Transfer Fee extension Config
    let initialize_transfer_fee =
        spl_token_2022::extension::transfer_fee::instruction::initialize_transfer_fee_config(
            &spl_token_2022::id(),
            &mint_pubkey,
            None,
            None,
            500,    // 0.5% fee
            10_000, // Maximum fee of 100 tokens
        )
        .unwrap();

    // Initialize Mint Account
    let initialize_mint = spl_token_2022::instruction::initialize_mint(
        &spl_token_2022::id(),
        &mint_pubkey,
        &mint_authority.pubkey(),
        None,
        0,
    )
    .unwrap();

    // Create associated token accounts for Alice and Bob
    let create_alice_ata =
        spl_associated_token_account::instruction::create_associated_token_account(
            &payer.pubkey(),
            &alice_pubkey,
            &mint_pubkey,
            &spl_token_2022::id(),
        );
    let create_bob_ata = spl_associated_token_account::instruction::create_associated_token_account(
        &payer.pubkey(),
        &bob_pubkey,
        &mint_pubkey,
        &spl_token_2022::id(),
    );

    // Mint tokens to Alice
    let mint_to_alice = spl_token_2022::instruction::mint_to_checked(
        &spl_token_2022::id(),
        &mint_pubkey,
        &alice_ata_pubkey,
        &mint_authority.pubkey(),
        &[],
        1_000, // Mint 1000 tokens to Alice
        0,     // 0 decimals
    )
    .unwrap();

    // Transfer tokens from Alice to Bob with fees applied
    // 5% of 500 tokens = 25 tokens as fee
    let transfer_with_fees = spl_token_2022::instruction::transfer_checked(
        &spl_token_2022::id(),
        &alice_ata_pubkey,
        &mint_pubkey,
        &bob_ata_pubkey,
        &alice.pubkey(),
        &[],
        500, // Transfer 500 tokens
        0,   // 0 decimals
    )
    .unwrap();

    // Create a transaction
    let transaction = Transaction::new_signed_with_payer(
        &[
            create_mint_account,
            initialize_transfer_fee,
            initialize_mint,
            create_alice_ata,
            create_bob_ata,
            mint_to_alice,
            transfer_with_fees,
        ],
        Some(&payer.pubkey()),
        &[&payer, &minter, &mint_authority, &alice],
        recent_blockhash,
    );

    // Execute the transactions
    execute_call_message(
        &transaction,
        &mut runner,
        "create_init_fee_init_mint_create_ata_mint_transfer",
    );

    // Check final state
    runner.query_state(|state| {
        let alice_account = get_account_info(alice_ata_pubkey, state);

        let alice_token_data = unpack_token_account_data(&alice_account.data);

        assert_eq!(
            alice_token_data.amount, 500,
            "Expected Alice to have 500 tokens after transfer"
        );

        let bob_account = get_account_info(bob_ata_pubkey, state);

        let bob_token_data = unpack_token_account_data(&bob_account.data);

        assert_eq!(
            bob_token_data.amount,
            475, // 0.5% fee applied
            "Expected Bob to receive 475 tokens after transfer with fees"
        );
    });
}

// In spl-token-2022, the account's data can be longer than the usual TokenAccount size of 165 bytes.
// This occurs because the spl-token-2022 program supports extensions, which require additional space.
// As a result, both TokenAccount and Mint data can have variable sizes depending on the enabled extensions.
pub fn unpack_token_account_data(account_data: &[u8]) -> TokenAccount {
    // Ensure the data is the correct length for a Token Account
    let trimmed_data = &account_data[..TokenAccount::LEN];

    // Unpack the trimmed data into a TokenAccount struct
    TokenAccount::unpack(trimmed_data).expect("Failed to unpack token account data")
}
