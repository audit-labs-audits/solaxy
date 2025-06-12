//! Integration tests for the `CallMessage::ExecuteTransactions` message.

use solana_sdk::{
    native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::Keypair, signer::Signer,
    system_program,
};
use sov_modules_api::TxEffect;
use sov_test_utils::TransactionTestCase;
use svm::test_utils::{prepare_new_account_transaction, prepare_sol_transfer_transaction};

mod common;

use common::setup::{
    check_system_accounts, create_input, execute_call_message, execute_call_message_batch,
    get_account_info, recent_blockhash, setup_svm_with_accounts, system_account, TestSvm,
};

const HALF_SOL: u64 = LAMPORTS_PER_SOL >> 1;
const QUARTER_SOL: u64 = LAMPORTS_PER_SOL >> 2;

#[test]
fn test_create_account() {
    // Setup: define Solana accounts that will be used in this test.
    let payer = Keypair::new(); // This account will be created during genesis.
    let payer_pubkey = payer.pubkey();
    let new_account = Keypair::new(); // This account will be created during SVM execution.
    let new_account_pubkey = new_account.pubkey();
    let accounts = &[(payer_pubkey, system_account())];

    // Setup: Run genesis and get a test runner
    let mut runner = setup_svm_with_accounts(accounts);

    // Check: assert the 'payer' account exist and has funds, and that 'new_account' doesn't exist yet.
    runner.query_state(|state| {
        let payer_account = get_account_info(payer_pubkey, state);
        assert_eq!(payer_account.owner, system_program::id());
        assert_eq!(payer_account.lamports, LAMPORTS_PER_SOL);
        let account_info =
            TestSvm::default().get_account_info(new_account_pubkey.into(), None, state);
        assert!(account_info.unwrap().value.is_none());
    });

    // Setup: Prepare a `create_account` system instruction and encode it in base58.
    let recent_blockhash = recent_blockhash(&runner);
    let create_acct_tx =
        prepare_new_account_transaction(&payer, &new_account, HALF_SOL, recent_blockhash);

    // Execute the transaction
    execute_call_message(&create_acct_tx, &mut runner, "create_account");

    // Check: assert that 'new_account' is now present in the module state and 'payer' has less funds.
    runner.query_state(|state| {
        let payer_account = get_account_info(payer_pubkey, state);
        // two signatures needed for creating an account
        let remaining_balance = HALF_SOL - 2 * 5_000;
        assert_eq!(payer_account.owner, system_program::id());
        assert_eq!(
            remaining_balance, payer_account.lamports,
            "expected payer account to have exactly {remaining_balance} lamports, but it has {} lamports",
            payer_account.lamports,
        );

        let new_account = get_account_info(new_account_pubkey, state);
        assert_eq!(new_account.owner, system_program::id());
        assert_eq!(
            new_account.lamports, HALF_SOL,
            "expected new account to have exactly {remaining_balance} lamports, but it has {} lamports",
            new_account.lamports,
        );
    });
}

#[test]
fn test_transfer_sol() {
    // Setup: define Solana accounts that will be used in this test.
    let payer = Keypair::new();
    let payer_pubkey = payer.pubkey();
    let recipient_pubkey = Pubkey::new_unique();
    let accounts = &[
        (payer_pubkey, system_account()),
        (recipient_pubkey, system_account()),
    ];

    // Setup: Run genesis and get a test runner
    let mut runner = setup_svm_with_accounts(accounts);

    // Check: assert both 'payer' and 'recipient' accounts exist and have the expected funds.
    runner.query_state(|state| {
        for pubkey in &[payer_pubkey, recipient_pubkey] {
            let account = get_account_info(*pubkey, state);
            assert_eq!(account.owner, system_program::id());
            assert_eq!(account.lamports, LAMPORTS_PER_SOL);
        }
    });

    // Setup: Prepare a `transfer` system instruction and encode it in base58.
    let recent_blockhash = recent_blockhash(&runner);
    let sol_transfer_tx =
        prepare_sol_transfer_transaction(&payer, &recipient_pubkey, HALF_SOL, recent_blockhash);

    // Execute the transaction
    execute_call_message(&sol_transfer_tx, &mut runner, "transfer_sol");

    // Check: assert that 'recipient' account has more funds and 'payer' account has less funds now.
    runner.query_state(|state| {
        let payer_account = get_account_info(payer_pubkey, state);
        let remaining_balance = HALF_SOL - 5_000;
        assert_eq!(payer_account.owner, system_program::id());
        assert_eq!(
            remaining_balance, payer_account.lamports,
            "expected payer account to have exactly {remaining_balance} lamports, but it has {} lamports",
            payer_account.lamports,
        );
        let recipient_account = get_account_info(recipient_pubkey, state);
        assert_eq!(recipient_account.owner, system_program::id());
        let expected_recipient_account_balance = LAMPORTS_PER_SOL + HALF_SOL;
        assert_eq!(
            recipient_account.lamports, expected_recipient_account_balance,
            "expected recipient account to have exactly {} lamports, but it has {} lamports",
            expected_recipient_account_balance, recipient_account.lamports,
        );
    });

    check_system_accounts(&runner);
}

/// This test combines `test_create_account` and `test_transfer_sol` to check if
/// state changes persist across multiple Solana transactions. We skip checking basic
/// premises because that's already covered by the other tests.
#[test]
fn test_create_account_and_transfer_sol() {
    // Setup: define Solana accounts that will be used in this test.
    let participant_a = Keypair::new(); // Created during genesis, 1 SOL
    let participant_b = Keypair::new(); // Not created during genesis, will be created by a tx
    let participant_c = Keypair::new(); // Also created in genesis, 1 SOL
    let accounts = &[
        (participant_a.pubkey(), system_account()),
        (participant_c.pubkey(), system_account()),
    ];

    // Setup: Run genesis and get a test runner
    let mut runner = setup_svm_with_accounts(accounts);

    // Prepare and execute a `CallMessage` with three transactions
    let txs = {
        let recent_blockhash = recent_blockhash(&runner);
        let create_account_b = prepare_new_account_transaction(
            &participant_a,
            &participant_b,
            HALF_SOL,
            recent_blockhash,
        );

        let transfer_from_a_to_c = prepare_sol_transfer_transaction(
            &participant_a,
            &participant_c.pubkey(),
            QUARTER_SOL,
            recent_blockhash,
        );

        let transfer_from_c_to_b = prepare_sol_transfer_transaction(
            &participant_c,
            &participant_b.pubkey(),
            HALF_SOL,
            recent_blockhash,
        );

        vec![
            (create_account_b, "create_account_b"),
            (transfer_from_a_to_c, "transfer_from_a_to_c"),
            (transfer_from_c_to_b, "transfer_from_c_to_b"),
        ]
    };
    execute_call_message_batch(&txs, &mut runner);

    // Check: assert that all account balances changed accordingly
    runner.query_state(|state| {
        // participant A transfers:
        // ------------------------------------------
        // participant A initial balance   : 1.00 SOL
        // participant A --> participant B : 0.50 SOL
        // participant A --> participant C : 0.25 SOL
        // participant A final balance     : 0.25 SOL
        let account_a = TestSvm::default()
            .get_account_info(participant_a.pubkey().into(), None, state)
            .expect("participant_a account should still exist post transaction")
            .value
            .unwrap();
        // two signatures for creating account, one signature for transferring funds
        let expected_a_balance = QUARTER_SOL - 3 * 5_000;
        assert_eq!(
            expected_a_balance, account_a.lamports,
            "expected participant_a account to have exactly {expected_a_balance} lamports, but it has {} lamports",
            account_a.lamports,
        );

        // participant B transfers:
        // ------------------------------------------
        // participant B initial balance   : 0.50 SOL
        // participant C --> participant B : 0.50 SOL
        // participant B final balance     : 1.00 SOL
        let account_b = TestSvm::default()
            .get_account_info(participant_b.pubkey().into(), None, state)
            .expect("participant_b account should exist post transaction")
            .value
            .unwrap();
        assert_eq!(
            LAMPORTS_PER_SOL, account_b.lamports,
            "expected participant_b account to have exactly {LAMPORTS_PER_SOL} lamports, but it has {} lamports",
            account_b.lamports,
        );

        // participant C transfers:
        // ------------------------------------------
        // participant C initial balance   :  1.00 SOL
        // participant A --> participant C :  0.25 SOL
        // participant C --> participant B : -0.50 SOL
        // participant C final balance     :  0.75 SOL
        let account_c = TestSvm::default()
            .get_account_info(participant_c.pubkey().into(), None, state)
            .expect("participant_c account should exist post transaction")
            .value
            .unwrap();
        // one signature for transferring funds
        let expected_account_c_balance = LAMPORTS_PER_SOL - QUARTER_SOL - 5_000;
        assert_eq!(
            expected_account_c_balance, account_c.lamports,
            "expected participant_c account to have exactly {expected_account_c_balance} lamports, but it has {} lamports",
            account_c.lamports,
        );
    });
}

#[test]
fn test_transfer_insufficient_funds() {
    // Setup: define Solana accounts that will be used in this test.
    let payer = Keypair::new();
    let payer_pubkey = payer.pubkey();
    let recipient_pubkey = Pubkey::new_unique();
    let accounts = &[
        (payer_pubkey, system_account()),
        (recipient_pubkey, system_account()),
    ];

    // Setup: Run genesis and get a test runner
    let mut runner = setup_svm_with_accounts(accounts);

    // Setup: Prepare a `transfer` system instruction and encode it in base58.
    let recent_blockhash = recent_blockhash(&runner);
    let two_sol = LAMPORTS_PER_SOL << 1;
    let sol_transfer_tx =
        prepare_sol_transfer_transaction(&payer, &recipient_pubkey, two_sol, recent_blockhash);

    // Execute the transaction.
    // It should be reverted since `payer` account doesn't have sufficient funds
    {
        let input = create_input(&sol_transfer_tx);
        let test_case = TransactionTestCase {
            input,
            assert: Box::new(move |result, _state| {
                if let TxEffect::Reverted(contents) = result.tx_receipt {
                    assert_eq!(
                        contents.reason.to_string(),
                        "Failed to load execute and commit transactions: Engine execution error: Transaction number 0 failed to execute: Error processing Instruction 0: custom program error: 0x1"
                    );
                } else {
                    panic!(
                        "The transaction should have reverted, instead the outcome was {:?}",
                        result.tx_receipt
                    );
                }
            }),
        };
        runner.execute_transaction(test_case);
    };

    // Check: The transaction should've failed because 'payer' doesn't have enough funds,
    // so the state should remain unchanged with the aborted transaction.
    runner.query_state(|state| {
        let payer_account = TestSvm::default()
            .get_account_info(payer_pubkey.into(), None, state)
            .expect("payer account should still exist post transaction")
            .value
            .unwrap();
        assert_eq!(
            LAMPORTS_PER_SOL, payer_account.lamports,
            "expected payer account to have exactly {LAMPORTS_PER_SOL} lamports, but it has {} lamports",
            payer_account.lamports,
        );
        let recipient_account = TestSvm::default()
            .get_account_info(recipient_pubkey.into(), None, state)
            .expect("recipient account should exist post transaction")
            .value
            .unwrap();
        assert_eq!(
            LAMPORTS_PER_SOL, recipient_account.lamports,
            "expected recipient account to have exactly {LAMPORTS_PER_SOL} lamports, but it has {} lamports",
            recipient_account.lamports,
        );
    });
}

#[test]
fn test_close_account() {
    let sender = Keypair::new(); // Created during genesis, 1 SOL
    let receiver = Keypair::new(); // Also created in genesis, 1 SOL
    let accounts = &[
        (sender.pubkey(), system_account()),
        (receiver.pubkey(), system_account()),
    ];

    let mut runner = setup_svm_with_accounts(accounts);

    let recent_blockhash = recent_blockhash(&runner);

    // sender account sends entire balance except what's needed for gas
    // by default without priority fees, only 5k lamports are collected per signature
    let tx = prepare_sol_transfer_transaction(
        &sender,
        &receiver.pubkey(),
        LAMPORTS_PER_SOL - 5_000,
        recent_blockhash,
    );
    execute_call_message(&tx, &mut runner, "transfer_sol");

    runner.query_state(|state| {
        let res = TestSvm::default().get_account_info(sender.pubkey().into(), None, state);
        assert!(
            res.unwrap().value.is_none(),
            "expected sender account to be closed because it doesn't meet rent exemption"
        );

        let receiver = TestSvm::default()
            .get_account_info(receiver.pubkey().into(), None, state)
            .expect("receiver account should still exist post transaction")
            .value
            .unwrap();
        // sender only sent `LAMPORTS_PER_SOL - 5_000` because it needs the 5K to pay for gas
        let remaining_balance = LAMPORTS_PER_SOL * 2 - 5_000;
        assert_eq!(
            remaining_balance, receiver.lamports,
            "expected receiver account to have {remaining_balance} lamports, but it has {} lamports",
            receiver.lamports,
        );
    });
}
