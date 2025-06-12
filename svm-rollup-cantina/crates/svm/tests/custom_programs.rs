use borsh::BorshDeserialize;
use solana_program::{
    bpf_loader_upgradeable::{get_program_data_address, UpgradeableLoaderState},
    instruction::{AccountMeta, Instruction},
    rent::Rent,
};
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use svm::test_utils::{
    counter_account, create_deploy_program_transactions, load_program_keypair, Counter,
};

mod common;

use common::setup::{
    execute_call_message, execute_call_message_batch, get_account_info, recent_blockhash,
    setup_svm_with_accounts, system_account, UserProgram,
};

pub static USER_PROGRAMS: &[UserProgram] = &[
    UserProgram {
        keypair_path: "test-data/custom_bytecode/hello_world/hello_world_program-keypair.json",
        bytecode: include_bytes!(
            "../../test-data/custom_bytecode/hello_world/hello_world_program.so"
        ),
    },
    UserProgram {
        keypair_path: "test-data/custom_bytecode/counter/counter_program-keypair.json",
        bytecode: include_bytes!("../../test-data/custom_bytecode/counter/counter_program.so"),
    },
];

#[test]
fn test_deploy_hello_world() {
    // Public key of the hello world program: Dr96P2PHovyGZAQVoaVXhoQTgsycPnu2hviwLkCnKHAB
    // Load the program keypair, which will own the program account
    let program_keypair = load_program_keypair(USER_PROGRAMS[0].keypair_path);
    let program_pubkey = program_keypair.pubkey();

    // Load the bytecode of the hello world program
    let bytecode = USER_PROGRAMS[0].bytecode;

    // Derive the program data address (PDA)
    let program_data_pubkey = get_program_data_address(&program_pubkey);

    let payer = Keypair::new(); // Payer who funds the transaction
    let payer_pubkey = payer.pubkey();

    // Initialize the accounts
    let accounts = &[
        (payer_pubkey, system_account()), // Payer's system account, also program's authority/owner
    ];

    // Initialize the genesis configuration and runner
    let mut runner = setup_svm_with_accounts(accounts);

    // Get the recent blockhash
    let recent_blockhash = recent_blockhash(&runner);

    let deploy_txs = create_deploy_program_transactions(
        &payer,
        &program_keypair,
        &payer,
        bytecode.to_vec(),
        Rent::default()
            .minimum_balance(bytecode.len() + UpgradeableLoaderState::size_of_buffer_metadata()),
        recent_blockhash,
    )
    .unwrap();
    let len = deploy_txs.len();
    let deploy_txs = deploy_txs
        .into_iter()
        .enumerate()
        .map(|(idx, tx)| {
            if idx == 0 {
                (tx, "setup_deploy")
            } else if idx == len - 1 {
                (tx, "finalize_deploy")
            } else {
                (tx, "deploy")
            }
        })
        .collect::<Vec<_>>();

    execute_call_message_batch(&deploy_txs, &mut runner);

    // Transaction: created to interact with hello_world program
    let hello_world_instruction = Instruction::new_with_bincode(
        program_pubkey,
        &(), // To trigger msg!(hello world) line in the program no instruction is required
        vec![AccountMeta::new(payer_pubkey, true)],
    );
    let hello_world_tx = Transaction::new_signed_with_payer(
        &[hello_world_instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    execute_call_message(&hello_world_tx, &mut runner, "call_hello_world");

    // Verify the program accounts were created and persisted in the SVM state correctly
    runner.query_state(|state| {
        let program_account = get_account_info(program_pubkey, state);

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

        let program_data_account = get_account_info(program_data_pubkey, state);
        assert!(
            !program_data_account.data.is_empty(),
            "expect program data account should contain owner and program bytecode info"
        );
    });
}

#[test]
fn test_deploy_counter() {
    // account owned by counter program
    let counter = Keypair::new();
    let counter_pubkey = counter.pubkey();

    // public key of counter program: 2MXRAGGPPdZDtqR1Qz1oyG5Pt2xi5oLmy3jFUGAQdkv4
    // Load the program keypair, which will own the program account
    let program_keypair = load_program_keypair(USER_PROGRAMS[1].keypair_path);
    let program_pubkey = program_keypair.pubkey();

    // Load the bytecode of the counter program
    let bytecode = USER_PROGRAMS[1].bytecode;

    // Derive the program data address (PDA)
    let program_data_pubkey = get_program_data_address(&program_pubkey);

    let payer = Keypair::new(); // Payer who funds the transaction
    let payer_pubkey = payer.pubkey();

    // Initialize the accounts
    let accounts = &[
        (payer_pubkey, system_account()), // Payer's system account, also program's authority/owner
        (counter_pubkey, counter_account(&program_pubkey)), // Counter account, owned by counter program
    ];

    // Initialize the genesis configuration and runner
    let mut runner = setup_svm_with_accounts(accounts);

    let deploy_txs = create_deploy_program_transactions(
        &payer,
        &program_keypair,
        &payer,
        bytecode.to_vec(),
        Rent::default()
            .minimum_balance(bytecode.len() + UpgradeableLoaderState::size_of_buffer_metadata()),
        recent_blockhash(&runner),
    )
    .unwrap();
    let len = deploy_txs.len();
    let deploy_txs = deploy_txs
        .into_iter()
        .enumerate()
        .map(|(idx, tx)| {
            if idx == 0 {
                (tx, "setup_deploy")
            } else if idx == len - 1 {
                (tx, "finalize_deploy")
            } else {
                (tx, "deploy")
            }
        })
        .collect::<Vec<_>>();

    execute_call_message_batch(&deploy_txs, &mut runner);

    for _ in 0..3 {
        let blockhash = recent_blockhash(&runner);
        execute_call_message(
            &Transaction::new_signed_with_payer(
                &[Instruction::new_with_bincode(
                    program_pubkey,
                    &[0], // Instruction to increase counter by one
                    vec![AccountMeta::new(counter_pubkey, false)],
                )],
                Some(&payer.pubkey()),
                &[&payer],
                blockhash,
            ),
            &mut runner,
            "call_counter",
        );
    }

    // Verify the program accounts were created and persisted in the SVM state correctly
    runner.query_state(|state| {
        let program_account = get_account_info(program_pubkey, state);

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

        let program_data_account = get_account_info(program_data_pubkey, state);
        assert!(
            !program_data_account.data.is_empty(),
            "expect program data account should contain owner and program bytecode info"
        );

        // Verify the state of the counter account was updated
        let counter_acc = get_account_info(counter_pubkey, state);
        let post_counter =
            Counter::try_from_slice(&counter_acc.data).expect("Failed to deserialize Counter data");
        assert_eq!(post_counter.count, 3, "Counter is expected to be 1");
    });
}
