use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use solana_account_decoder::{
    parse_account_data::SplTokenAdditionalData, parse_token::token_amount_to_ui_amount_v2,
};
use solana_accounts_db::blockhash_queue::BlockhashQueue;
use solana_program::{
    clock::{Slot, UnixTimestamp},
    hash::hash,
};
use solana_rpc_client_api::{
    config::{
        RpcProgramAccountsConfig, RpcSimulateTransactionAccountsConfig,
        RpcSimulateTransactionConfig, RpcTokenAccountsFilter,
    },
    filter::{Memcmp, RpcFilterType},
    request::MAX_GET_CONFIRMED_BLOCKS_RANGE,
    response::{RpcConfirmedTransactionStatusWithSignature, RpcPrioritizationFee},
};
use solana_sdk::{
    account::{Account, AccountSharedData},
    clock::MAX_PROCESSING_AGE,
    epoch_info::EpochInfo,
    hash::Hash,
    instruction::CompiledInstruction,
    message::{Message, MessageHeader},
    native_loader,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_instruction, system_program,
    transaction::{Transaction, TransactionError},
};
use solana_transaction_status::{
    BlockEncodingOptions, ConfirmedBlock, TransactionDetails, UiTransactionEncoding,
};
use sov_modules_api::ApiStateAccessor;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token::{
    solana_program::program_pack::Pack,
    state::{Account as TokenAccount, AccountState, Mint},
};
use svm::{
    create_client_error, create_server_error,
    test_utils::{
        prepare_new_account_transaction, prepare_sol_transfer_transaction, serialize_and_encode,
        set_svm_variables,
    },
    wrappers::ExecutedTransactionData,
    ConfirmedBlockDetails, ConfirmedTransactionDetails, SVM,
};

mod common;

use common::setup::{setup_svm_with_accounts, setup_svm_with_defaults, RT, S};

use crate::common::setup::{mint_account, system_account, token_account};

#[test]
fn test_get_account_info() {
    let runner = setup_svm_with_defaults();
    let mut rt = RT::default();
    let svm = &mut rt.svm;
    runner.query_state(|api_state_accessor| {
        let (pubkey_1, _pubkey_2, pubkey_3, _pubkey_4) =
            create_test_accounts(api_state_accessor, svm);

        let test_result = svm.get_account_info(pubkey_3.into(), None, api_state_accessor);
        let test_value = Account {
            lamports: 54_321,
            owner: native_loader::id(),
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            executable: false,
            rent_epoch: 0,
        };

        assert_eq!(
            test_result.unwrap().value.unwrap().decode(),
            Some(test_value)
        );

        let test_result = svm.get_account_info(pubkey_1.into(), None, api_state_accessor);
        let test_value = Account {
            lamports: 10_000,
            owner: native_loader::id(),
            data: vec![0, 0, 0, 0, 0],
            executable: false,
            rent_epoch: 0,
        };

        assert_eq!(
            test_result.unwrap().value.unwrap().decode(),
            Some(test_value)
        );

        let nonexistent_pubkey = Pubkey::new_unique();

        let test_result = svm.get_account_info(nonexistent_pubkey.into(), None, api_state_accessor);

        assert!(test_result.unwrap().value.is_none());
    });
}

#[test]
fn test_get_balance() {
    let runner = setup_svm_with_defaults();
    let mut rt = RT::default();
    let svm = &mut rt.svm;
    runner.query_state(|api_state_accessor| {
        let (pubkey_1, _pubkey_2, pubkey_3, _pubkey_4) =
            create_test_accounts(api_state_accessor, svm);

        let mut test_result = svm.get_balance(pubkey_1.into(), None, api_state_accessor);
        assert_eq!(test_result.unwrap().value, 10_000);

        test_result = svm.get_balance(pubkey_3.into(), None, api_state_accessor);
        assert_eq!(test_result.unwrap().value, 54_321);

        let nonexistent_pubkey = Pubkey::new_unique();
        test_result = svm.get_balance(nonexistent_pubkey.into(), None, api_state_accessor);
        assert_eq!(test_result.unwrap().value, 0);
    });
}

#[test]
fn test_get_epoch_info() {
    let runner = setup_svm_with_defaults();
    let mut rt = RT::default();
    let svm = &mut rt.svm;
    runner.query_state(|api_state_accessor| {
        assert!(svm.get_epoch_info(None, api_state_accessor).is_ok());

        let epoch_info = EpochInfo {
            epoch: 0,
            slot_index: 1,
            slots_in_epoch: 2,
            absolute_slot: 3,
            block_height: 4,
            transaction_count: None,
        };
        svm.epoch_info
            .set(&epoch_info, api_state_accessor)
            .expect("Failed to set epoch information in SVM");

        assert_eq!(
            svm.get_epoch_info(None, api_state_accessor).unwrap(),
            epoch_info
        );
        assert_eq!(svm.get_slot(None, api_state_accessor).unwrap(), 3);
        assert_eq!(svm.get_block_height(None, api_state_accessor).unwrap(), 3);
    });
}

#[test]
fn test_get_multiple_accounts() {
    let runner = setup_svm_with_defaults();
    let mut rt = RT::default();
    let svm = &mut rt.svm;
    runner.query_state(|api_state_accessor| {
        let (pubkey_1, _pubkey_2, pubkey_3, _pubkey_4) =
            create_test_accounts(api_state_accessor, svm);

        let test_result = svm.get_multiple_accounts(
            vec![pubkey_3.into(), pubkey_1.into()],
            None,
            api_state_accessor,
        );
        let test_value = vec![
            Some(Account {
                lamports: 54_321,
                owner: native_loader::id(),
                data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                executable: false,
                rent_epoch: 0,
            }),
            Some(Account {
                lamports: 10_000,
                owner: native_loader::id(),
                data: vec![0, 0, 0, 0, 0],
                executable: false,
                rent_epoch: 0,
            }),
        ];

        assert_eq!(
            test_result
                .unwrap()
                .value
                .into_iter()
                .map(|op| op.map(|acc| acc.decode::<Account>().unwrap()))
                .collect::<Vec<_>>(),
            test_value
        );

        let pubkey_test = Pubkey::new_unique();

        let test_result = svm.get_multiple_accounts(
            vec![pubkey_3.into(), pubkey_1.into(), pubkey_test.into()],
            None,
            api_state_accessor,
        );

        assert!(test_result.unwrap().value.last().unwrap().is_none());
    });
}

#[test]
fn test_get_fee_for_message() {
    let runner = setup_svm_with_defaults();
    let mut rt = RT::default();
    let svm = &mut rt.svm;
    runner.query_state(|api_state_accessor| {
        let recent_blockhash = Hash::default();

        // message requires 3 signatures
        let msg = create_test_message(recent_blockhash);

        let test_result = svm.get_fee_for_message(msg.clone(), None, api_state_accessor);

        assert_eq!(
            test_result.unwrap_err(),
            create_server_error("Failed to get lamports per signature value from blockhash queue")
        );

        set_svm_variables(svm, &recent_blockhash, api_state_accessor);
        let test_result = svm.get_fee_for_message(msg, None, api_state_accessor);

        // 3 signatures cost 5_000 * 3 = 15_000
        assert_eq!(test_result.unwrap().value.unwrap(), 15_000);
    });
}

#[test]
fn test_get_latest_blockhash() {
    let runner = setup_svm_with_defaults();
    let mut rt = RT::default();
    let svm = &mut rt.svm;
    runner.query_state(|api_state_accessor| {
        // Case 1: Empty blockhash queue
        let empty_queue = BlockhashQueue::new(150);
        svm.blockhash_queue
            .set(&empty_queue, api_state_accessor)
            .expect("Failed to set blockhash_queue in SVM");
        let test_result = svm.get_latest_blockhash(None, api_state_accessor);
        assert_eq!(
            test_result.unwrap_err(),
            create_server_error("blockhash_queue is empty")
        );

        // Case 2: Non-empty blockhash queue
        let test_hash = Hash::new_unique();
        let non_empty_queue = {
            let mut queue = BlockhashQueue::new(150);
            queue.register_hash(&test_hash, 0);
            queue
        };
        svm.blockhash_queue
            .set(&non_empty_queue, api_state_accessor)
            .expect("Failed to set blockhash_queue in SVM");
        let test_result = svm.get_latest_blockhash(None, api_state_accessor);
        let unwrapped = test_result.unwrap();
        assert_eq!(
            Hash::from_str(&unwrapped.value.blockhash).unwrap(),
            test_hash
        );
        assert_eq!(
            unwrapped.value.last_valid_block_height,
            svm.epoch_info
                .get(api_state_accessor)
                .unwrap()
                .unwrap()
                .block_height
                + MAX_PROCESSING_AGE as u64
        );
    });
}

#[test]
fn test_is_blockhash_valid() {
    let runner = setup_svm_with_defaults();
    let mut rt = RT::default();
    let svm = &mut rt.svm;
    runner.query_state(|api_state_accessor| {
        let test_hash = Hash::new_unique();
        let test_slot = 1;
        let test_blockhash_queue = {
            let mut queue = BlockhashQueue::new(150);
            queue.register_hash(&test_hash, test_slot);
            queue
        };
        svm.blockhash_queue
            .set(&test_blockhash_queue, api_state_accessor)
            .expect("Failed to set blockhash_queue in SVM");

        // Case 1: Valid blockhash
        let test_result = svm.is_blockhash_valid(test_hash.to_string(), None, api_state_accessor);
        assert!(test_result.unwrap().value);

        // Case 2: Invalid blockhash
        let invalid_hash = Hash::new_unique();
        let test_result =
            svm.is_blockhash_valid(invalid_hash.to_string(), None, api_state_accessor);
        assert!(!test_result.unwrap().value);
    });
}

#[test]
fn test_get_program_accounts() {
    let runner = setup_svm_with_defaults();
    let mut rt = RT::default();
    let svm = &mut rt.svm;
    runner.query_state(|api_state_accessor| {
        let (pubkey_1, pubkey_2, pubkey_3, pubkey_4) = create_test_pubkeys();

        // change acc1 to executable
        let mut program_account = Account::new(10_000, 5, &pubkey_1);
        program_account.executable = true;

        svm.insert_account(&pubkey_1, &program_account, api_state_accessor)
            .unwrap();
        svm.insert_account(
            &pubkey_2,
            &Account::new(12_345, 15, &pubkey_2),
            api_state_accessor,
        )
        .unwrap();
        svm.insert_account(
            &pubkey_3,
            &Account::new(54_321, 12, &pubkey_1),
            api_state_accessor,
        )
        .unwrap();
        svm.insert_account(
            &pubkey_4,
            &Account::new(11_111, 3, &pubkey_3),
            api_state_accessor,
        )
        .unwrap();

        svm.owner_index
            .set(&pubkey_1, &vec![pubkey_1, pubkey_3], api_state_accessor)
            .expect("Failed to set owner_index in SVM for pubkey1");
        svm.owner_index
            .set(&pubkey_2, &vec![pubkey_2], api_state_accessor)
            .expect("Failed to set owner_index in SVM for pubkey2");
        svm.owner_index
            .set(&pubkey_3, &vec![pubkey_3], api_state_accessor)
            .expect("Failed to set owner_index in SVM for pubkey3");

        let mut test_value = vec![
            Account {
                lamports: 54_321,
                owner: pubkey_1,
                data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                executable: false,
                rent_epoch: 0,
            },
            Account {
                lamports: 10_000,
                owner: pubkey_1,
                data: vec![0, 0, 0, 0, 0],
                executable: true,
                rent_epoch: 0,
            },
        ];

        let mut test_result = svm
            .get_program_accounts(pubkey_1.into(), None, api_state_accessor)
            .unwrap()
            .into_iter()
            .map(|acc| acc.account.decode::<Account>().unwrap())
            .collect::<Vec<_>>();

        // Sort the test value and test result by a unique attribute, such as `lamports` or `size`
        test_value.sort_by_key(|account| (account.lamports, account.data.clone()));
        test_result.sort_by_key(|account| (account.lamports, account.data.clone()));

        assert_eq!(test_result, test_value);

        let test_result = svm.get_program_accounts(pubkey_2.into(), None, api_state_accessor);

        assert_eq!(
            test_result.unwrap_err(),
            create_client_error(&format!("Account with pubkey {pubkey_2} is not executable"))
        );

        let token_account = Account {
            lamports: 10_000,
            owner: pubkey_1,
            data: vec![0; spl_token_2022::state::Account::LEN],
            executable: false,
            rent_epoch: 0,
        };
        let pubkey = Pubkey::new_unique();

        svm.insert_account(&pubkey, &token_account, api_state_accessor)
            .unwrap();
        svm.owner_index
            .set(
                &pubkey_1,
                &vec![pubkey, pubkey_1, pubkey_3],
                api_state_accessor,
            )
            .expect("Failed to set owner_index in SVM for pubkey3");
        test_value.push(token_account);

        let cases = [
            (
                RpcProgramAccountsConfig {
                    filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                        5,
                        vec![0; 7],
                    ))]),
                    ..Default::default()
                },
                vec![test_value[2].clone(), test_value[1].clone()],
            ),
            (
                RpcProgramAccountsConfig {
                    filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                        6,
                        vec![0; 7],
                    ))]),
                    ..Default::default()
                },
                vec![test_value[2].clone()],
            ),
            (
                RpcProgramAccountsConfig {
                    filters: Some(vec![RpcFilterType::DataSize(5)]),
                    ..Default::default()
                },
                vec![test_value[0].clone()],
            ),
            (
                RpcProgramAccountsConfig {
                    filters: Some(vec![RpcFilterType::TokenAccountState]),
                    ..Default::default()
                },
                vec![test_value[2].clone()],
            ),
            (
                RpcProgramAccountsConfig {
                    filters: Some(vec![
                        RpcFilterType::DataSize(5),
                        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, vec![1; 1])),
                    ]),
                    ..Default::default()
                },
                Vec::new(),
            ),
        ];

        for (config, expected) in cases {
            let test_result = svm
                .get_program_accounts(pubkey_1.into(), Some(config.clone()), api_state_accessor)
                .unwrap()
                .into_iter()
                .map(|acc| acc.account.decode::<Account>().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(test_result, *expected);
        }
    });
}

#[test]
fn test_get_token_account_balance() {
    let runner = setup_svm_with_defaults();
    let mut rt = RT::default();
    let svm = &mut rt.svm;
    runner.query_state(|api_state_accessor| {
        let (pubkey_1, mint_pub_key, _, _) = create_test_pubkeys();

        let token_pub_key = spl_token::id();

        // NOTE: the spl token account is initialized with a full rollup runner
        // let mut test_result = svm.get_token_account_balance(token_pub_key, api_state_accessor);
        //
        // assert_eq!(
        //     test_result.unwrap_err(),
        //     create_client_error(&format!(
        //         "Account with public key {token_pub_key} not found: {}",
        //         create_client_error(&format!("Account not found with pubkey: {token_pub_key}"))
        //     ))
        // );

        let spl_pubkey = token_pub_key;

        let spl_token_account = Account {
            lamports: 100,
            data: vec![0, 12],
            owner: pubkey_1,
            executable: false,
            rent_epoch: 0,
        };

        svm.insert_account(&token_pub_key, &spl_token_account, api_state_accessor).unwrap();

        let test_result =
            svm.get_token_account_balance(token_pub_key.into(), None, api_state_accessor);

        assert_eq!(
            test_result.unwrap_err(),
            create_client_error(
                "Failed to unpack spl token account from data: An account's data contents was invalid"
            )
        );

        let token_account = TokenAccount {
            amount: 10_000,
            state: AccountState::Initialized,
            mint: mint_pub_key,
            ..Default::default()
        };

        let mut data = [0; TokenAccount::LEN];
        TokenAccount::pack(token_account, &mut data).unwrap();

        let spl_token_account = Account {
            lamports: 100,
            data: data.to_vec(),
            owner: spl_pubkey,
            executable: false,
            rent_epoch: 0,
        };

        svm.insert_account(&spl_pubkey, &spl_token_account, api_state_accessor).unwrap();

        let test_result =
            svm.get_token_account_balance(token_pub_key.into(), None, api_state_accessor);

        assert_eq!(
            test_result.unwrap_err(),
            create_client_error("Invalid param: could not find account")
        );

        let mint_account = Mint {
            decimals: 7,
            is_initialized: true,
            ..Default::default()
        };

        let mut data = [0; Mint::LEN];
        Mint::pack(mint_account, &mut data).unwrap();

        let mint_account = Account {
            lamports: 100,
            data: data.to_vec(),
            owner: pubkey_1,
            executable: false,
            rent_epoch: 0,
        };

        svm.insert_account(&mint_pub_key, &mint_account, api_state_accessor).unwrap();
        let test_result =
            svm.get_token_account_balance(token_pub_key.into(), None, api_state_accessor);

        let test_value = token_amount_to_ui_amount_v2(10000, &SplTokenAdditionalData::with_decimals(7));
        assert_eq!(test_result.unwrap().value, test_value);
    });
}

#[test]
fn test_get_token_accounts_by_owner() {
    // alice and bob are users; doge is an spl token and pepe is an spl 2022 token
    let (alice, bob, doge, pepe) = create_test_pubkeys();
    // alice owns both doge and pepe
    let alice_doge_ata =
        get_associated_token_address_with_program_id(&alice, &doge, &spl_token::id());
    let alice_pepe_ata =
        get_associated_token_address_with_program_id(&alice, &pepe, &spl_token_2022::id());
    let alice_doge = token_account(&alice, &doge, 10_000, &spl_token::id());
    let alice_pepe = token_account(&alice, &pepe, 10_000, &spl_token_2022::id());
    // bob only owns pepe
    let bob_pepe_ata =
        get_associated_token_address_with_program_id(&bob, &pepe, &spl_token_2022::id());
    let bob_pepe = token_account(&bob, &pepe, 10_000, &spl_token_2022::id());
    let runner = setup_svm_with_accounts(&[
        (doge, mint_account(&spl_token::id())),
        (pepe, mint_account(&spl_token_2022::id())),
        (alice_doge_ata, alice_doge.clone()),
        (alice_pepe_ata, alice_pepe.clone()),
        (bob_pepe_ata, bob_pepe.clone()),
    ]);
    let rt = runner.runtime();
    let svm = &rt.svm;

    runner.query_state(|api_state_accessor| {
        let test_cases: [(Pubkey, RpcTokenAccountsFilter, Vec<AccountSharedData>); 4] = [
            (
                alice,
                RpcTokenAccountsFilter::ProgramId(spl_token::id().to_string()),
                vec![alice_doge],
            ),
            (
                alice,
                RpcTokenAccountsFilter::Mint(pepe.to_string()),
                vec![alice_pepe],
            ),
            (
                bob,
                RpcTokenAccountsFilter::ProgramId(spl_token_2022::id().to_string()),
                vec![bob_pepe],
            ),
            (
                bob,
                RpcTokenAccountsFilter::ProgramId(spl_token::id().to_string()),
                Vec::new(),
            ),
        ];

        for (i, (owner, filter, expected_accounts)) in test_cases.into_iter().enumerate() {
            let actual_accounts = svm
                .get_token_accounts_by_owner(owner.into(), filter, None, api_state_accessor)
                .unwrap()
                .value
                .into_iter()
                .map(|res| res.account.decode::<AccountSharedData>().unwrap())
                .collect::<Vec<_>>();

            assert_eq!(
                expected_accounts, actual_accounts,
                "Expected account at index: {i} to match"
            );
        }
    });
}

#[test]
fn test_simulate_transaction() {
    let fee_payer1 = Keypair::new();
    let fee_payer2 = Keypair::new();
    let runner = setup_svm_with_accounts(&[
        (fee_payer1.pubkey(), system_account()),
        (fee_payer2.pubkey(), system_account()),
    ]);
    let mut rt = RT::default();
    let svm = &mut rt.svm;
    runner.query_state(|api_state_accessor| {
        let recent_blockhash = Hash::new_unique();

        set_svm_variables(svm, &recent_blockhash, api_state_accessor);

        // tx1: transfer 1_000_000 lamports from payer1 to payer2
        let tx1 = prepare_sol_transfer_transaction(
            &fee_payer1,
            &fee_payer2.pubkey(),
            1_000_000,
            recent_blockhash,
        );

        let simulate_tx_config = RpcSimulateTransactionConfig {
            accounts: Some(RpcSimulateTransactionAccountsConfig {
                addresses: vec![
                    fee_payer1.pubkey().to_string(),
                    fee_payer2.pubkey().to_string(),
                ],
                ..RpcSimulateTransactionAccountsConfig::default()
            }),
            ..RpcSimulateTransactionConfig::default()
        };

        // check if transaction was successfully simulated
        let message = serialize_and_encode(&tx1, UiTransactionEncoding::Base58).unwrap();
        let result =
            svm.simulate_transaction(message, Some(simulate_tx_config), api_state_accessor);

        let result = result.unwrap().value;
        let accounts = result.accounts.unwrap();

        // check post simulation accounts
        // acc1 = 1_000_000_000 - 1_000_000 - 5_000 (fee per signature) = 998_995_000
        let post_acc1 = accounts
            .first()
            .unwrap()
            .as_ref()
            .unwrap()
            .decode::<Account>()
            .unwrap();

        assert_eq!(post_acc1.lamports, 998_995_000);

        // acc2 = 1_000_000_000 + 1_000_000 = 1_001_000_000
        let post_acc2 = accounts
            .get(1)
            .unwrap()
            .as_ref()
            .unwrap()
            .decode::<Account>()
            .unwrap();

        assert_eq!(post_acc2.lamports, 1_001_000_000);

        let fee_payer3 = Keypair::new();

        // tx2: create payer3 account
        let tx2 =
            prepare_new_account_transaction(&fee_payer1, &fee_payer3, 5_000_000, recent_blockhash);

        let simulate_tx_config = RpcSimulateTransactionConfig {
            accounts: Some(RpcSimulateTransactionAccountsConfig {
                addresses: vec![
                    fee_payer1.pubkey().to_string(),
                    fee_payer3.pubkey().to_string(),
                ],
                ..RpcSimulateTransactionAccountsConfig::default()
            }),
            ..RpcSimulateTransactionConfig::default()
        };

        let message = serialize_and_encode(&tx2, UiTransactionEncoding::Base58).unwrap();
        let result =
            svm.simulate_transaction(message, Some(simulate_tx_config), api_state_accessor);

        let result = result.unwrap().value;
        let accounts = result.accounts.unwrap();

        // check post simulation accounts
        // acc1 = 1_000_000_000 - 5_000_000 - 2 * 5_000 (fees per signature) = 994_990_000
        let post_acc1 = accounts
            .first()
            .unwrap()
            .as_ref()
            .unwrap()
            .decode::<Account>()
            .unwrap();

        assert_eq!(post_acc1.lamports, 994_990_000);

        // acc3 = 0 + 5_000_000 = 5_000_000
        let post_acc3 = accounts
            .get(1)
            .unwrap()
            .as_ref()
            .unwrap()
            .decode::<Account>()
            .unwrap();

        assert_eq!(post_acc3.lamports, 5_000_000);

        // preparing tx3 which combines 3 instructions
        // transfer 2_000_000 from acc1 to acc2
        // create acc3 with initial balance 2_000_000 paid by acc1
        // transfer 1_000_00 from acc2 to acc3
        let tx3 = prepare_transaction_multiple_instructions(
            &fee_payer1,
            &fee_payer2,
            &fee_payer3,
            recent_blockhash,
        );

        let simulate_tx_config = RpcSimulateTransactionConfig {
            accounts: Some(RpcSimulateTransactionAccountsConfig {
                addresses: vec![
                    fee_payer1.pubkey().to_string(),
                    fee_payer2.pubkey().to_string(),
                    fee_payer3.pubkey().to_string(),
                ],
                ..RpcSimulateTransactionAccountsConfig::default()
            }),
            ..RpcSimulateTransactionConfig::default()
        };

        let message = serialize_and_encode(&tx3, UiTransactionEncoding::Base58).unwrap();
        let result =
            svm.simulate_transaction(message, Some(simulate_tx_config), api_state_accessor);

        let result = result.unwrap().value;
        let accounts = result.accounts.unwrap();

        // check post simulation accounts
        // acc1 = 1_000_000_000 - 2_000_000 - 2_000_000 - 3 * 5_000 (fees per signature) = 995_985_000
        let post_acc1 = accounts
            .first()
            .expect("post account 1 not found")
            .as_ref()
            .unwrap()
            .decode::<Account>()
            .unwrap();

        assert_eq!(post_acc1.lamports, 995_985_000);

        // acc2 = 1_000_000_000 + 2_000_000 - 1_000_000 = 1_001_000_000
        let post_acc2 = accounts
            .get(1)
            .expect("post account 2 not found")
            .as_ref()
            .unwrap()
            .decode::<Account>()
            .unwrap();

        assert_eq!(post_acc2.lamports, 1_001_000_000);

        // acc3 = 0 + 2_000_000 + 1_000_000 = 3_000_000
        let post_acc3 = accounts
            .get(2)
            .expect("post account 3 not found")
            .as_ref()
            .unwrap()
            .decode::<Account>()
            .unwrap();

        assert_eq!(post_acc3.lamports, 3_000_000);
    });
}

// #[test]
// fn test_update_feature_set() {
//     let runner = setup_svm_with_defaults();
//     let mut rt = RT::default();
//     let svm = &mut rt.svm;
//     runner.query_state(|api_state_accessor| {
//         let recent_blockhash = Hash::new_unique();
//         set_svm_variables(svm, &recent_blockhash, api_state_accessor);
//
//         // Test 1: Should fail when features overlap
//         let overlapping_key = Keypair::new().pubkey();
//         let mut active_features = HashMap::new();
//         active_features.insert(overlapping_key, 123); // slot 123
//         let mut inactive_features = HashSet::new();
//         inactive_features.insert(overlapping_key); // same key in inactive
//
//         let result = svm.update_feature_set(active_features, inactive_features, api_state_accessor);
//         assert!(result.is_err());
//
//         // Test 2: Should succeed with non-overlapping features
//         let active_key = Keypair::new().pubkey();
//         let mut active_features = HashMap::new();
//         active_features.insert(active_key, 123);
//
//         let inactive_key = Keypair::new().pubkey();
//         let mut inactive_features = HashSet::new();
//         inactive_features.insert(inactive_key);
//
//         let result = svm.update_feature_set(
//             active_features.clone(),
//             inactive_features.clone(),
//             api_state_accessor,
//         );
//         assert!(result.is_ok());
//
//         // Validate the changes
//         let feature_set = svm
//             .feature_set
//             .get(api_state_accessor)
//             .unwrap()
//             .expect("Failed to get feature set");
//
//         // Check active feature was activated
//         assert!(feature_set.is_active(&active_key));
//         assert_eq!(feature_set.activated_slot(&active_key), Some(123));
//
//         // Check inactive feature was deactivated
//         assert!(!feature_set.is_active(&inactive_key));
//         assert_eq!(feature_set.activated_slot(&inactive_key), None);
//     });
// }

#[test]
fn test_get_block() {
    let runner = setup_svm_with_defaults();
    let mut rt = RT::default();
    let svm = &mut rt.svm;
    runner.query_state(|api_state_accessor| {
        // Case 1: Existing block
        let test_prev_hash = Hash::new_unique();
        let test_block = create_test_block(1, test_prev_hash, None);
        svm.confirmed_blocks
            .set(&1, &test_block, api_state_accessor)
            .expect("Failed to set confirmed_blocks");

        let test_result = svm.get_block(1, None, api_state_accessor);
        let test_ui_confirmed_block = Into::<ConfirmedBlock>::into(test_block)
            .encode_with_options(
                UiTransactionEncoding::Json,
                BlockEncodingOptions {
                    transaction_details: TransactionDetails::default(),
                    show_rewards: true,
                    max_supported_transaction_version: None,
                },
            )
            .expect("Failed to encode test confirmed block");
        assert_eq!(test_result.unwrap().unwrap(), test_ui_confirmed_block);

        // Case 2: Non-existing block, returns None
        let test_result = svm.get_block(2, None, api_state_accessor);
        assert!(test_result.unwrap().is_none());
    });
}

#[test]
fn test_get_block_time() {
    let runner = setup_svm_with_defaults();
    let mut rt = RT::default();
    let svm = &mut rt.svm;
    runner.query_state(|api_state_accessor| {
        let test_prev_hash = Hash::new_unique();

        // Case 1: Block with no time, returns None
        let test_block = create_test_block(1, test_prev_hash, None);
        svm.confirmed_blocks
            .set(&1, &test_block, api_state_accessor)
            .expect("Failed to set confirmed_blocks");

        let test_result = svm.get_block_time(1, api_state_accessor);
        assert!(test_result.is_err());

        // Case 2: Block with existing time
        let test_time = 12345 as UnixTimestamp;
        let test_block = create_test_block(2, test_prev_hash, Some(test_time));
        svm.confirmed_blocks
            .set(&2, &test_block, api_state_accessor)
            .expect("Failed to set confirmed_blocks");
        let test_result = svm.get_block_time(2, api_state_accessor);
        assert_eq!(test_result.unwrap().unwrap(), test_time);

        // Case 3: Non-existing block, returns None
        let test_result = svm.get_block_time(3, api_state_accessor);
        assert!(test_result.unwrap().is_none());
    });
}

#[test]
fn test_get_blocks() {
    let runner = setup_svm_with_defaults();
    let mut rt = RT::default();
    let svm = &mut rt.svm;
    runner.query_state(|api_state_accessor| {
        // Case 1: Blocks exist for slot 1 to 3 and 5
        let blocks = vec![1, 2, 3, 5];

        svm.confirmed_block_slots
            .set_all(blocks, api_state_accessor)
            .expect("Failed to set confirmed_blocks");

        let config_wrapper =
            solana_rpc_client_api::config::RpcBlocksConfigWrapper::EndSlotOnly(Some(5));
        let test_result = svm.get_blocks(1, Some(config_wrapper), None, api_state_accessor);
        let expected_result: Vec<Slot> = vec![1, 2, 3, 5];
        assert_eq!(test_result.unwrap(), expected_result);

        // Case 2: No exisiting blocks for slots 6 to 10, return empty vector
        let config_wrapper =
            solana_rpc_client_api::config::RpcBlocksConfigWrapper::EndSlotOnly(Some(10));
        let test_result = svm.get_blocks(6, Some(config_wrapper), None, api_state_accessor);
        assert!(test_result.unwrap().is_empty());

        // Case 3: Start slot < End slot, return empty vector
        let config_wrapper =
            solana_rpc_client_api::config::RpcBlocksConfigWrapper::EndSlotOnly(Some(1));
        let test_result = svm.get_blocks(4, Some(config_wrapper), None, api_state_accessor);
        assert!(test_result.unwrap().is_empty());

        // Case 4: Large range of slots
        let config_wrapper = solana_rpc_client_api::config::RpcBlocksConfigWrapper::EndSlotOnly(
            Some(MAX_GET_CONFIRMED_BLOCKS_RANGE + 2),
        );
        let test_result = svm.get_blocks(1, Some(config_wrapper), None, api_state_accessor);
        assert_eq!(
            test_result.unwrap_err(),
            create_client_error(&format!(
                "Slot range too large; max {MAX_GET_CONFIRMED_BLOCKS_RANGE}"
            ))
        );
    });
}

#[test]
fn test_get_blocks_with_limit() {
    let runner = setup_svm_with_defaults();
    let mut rt = RT::default();
    let svm = &mut rt.svm;
    runner.query_state(|api_state_accessor| {
        // Case 1: Blocks exist for slot 1 to 5, query from 2 to 4
        let blocks = vec![1, 2, 3, 4, 5];

        svm.confirmed_block_slots
            .set_all(blocks, api_state_accessor)
            .expect("Failed to set confirmed_blocks");

        let test_result = svm.get_blocks_with_limit(2, 3, None, api_state_accessor);
        let expected_result: Vec<Slot> = vec![2, 3, 4];
        assert_eq!(test_result.unwrap(), expected_result);

        // Case 2: No exisiting blocks for slots 6 to 9, return empty vector
        let test_result = svm.get_blocks_with_limit(6, 3, None, api_state_accessor);
        assert!(test_result.unwrap().is_empty());

        // Case 3: Some existing blocks, return vector with 3 to 5
        let test_result = svm.get_blocks_with_limit(3, 6, None, api_state_accessor);
        let expected_result: Vec<Slot> = vec![3, 4, 5];
        assert_eq!(test_result.unwrap(), expected_result);

        // Case 4: Limit = 0, return empty vector
        let test_result = svm.get_blocks_with_limit(3, 0, None, api_state_accessor);
        assert!(test_result.unwrap().is_empty());

        // Case 5: Large range of slots, return error
        let test_result = svm.get_blocks_with_limit(
            1,
            (MAX_GET_CONFIRMED_BLOCKS_RANGE + 1) as usize,
            None,
            api_state_accessor,
        );
        assert_eq!(
            test_result.unwrap_err(),
            create_client_error(&format!(
                "Limit too large; max {MAX_GET_CONFIRMED_BLOCKS_RANGE}"
            ))
        );
    });
}

#[test]
fn test_get_first_available_block() {
    let runner = setup_svm_with_defaults();
    let mut rt = RT::default();
    let svm = &mut rt.svm;
    runner.query_state(|api_state_accessor| {
        // Case 1: No blocks in the queue
        let result = svm.get_first_available_block(api_state_accessor);
        assert_eq!(
            result.unwrap_err(),
            create_server_error("First available block not found")
        );

        // Case 2: Blocks exist in the queue
        svm.confirmed_block_slots
            .push(&1, api_state_accessor)
            .expect("Failed to push to confirmed_block_slots");

        let result = svm.get_first_available_block(api_state_accessor);
        assert_eq!(result.unwrap(), 1);

        svm.confirmed_block_slots
            .push(&2, api_state_accessor)
            .expect("Failed to push to confirmed_block_slots");

        let result = svm.get_first_available_block(api_state_accessor);
        assert_eq!(result.unwrap(), 1);

        svm.confirmed_block_slots
            .set_all(vec![3, 5, 6], api_state_accessor)
            .expect("Failed to push to confirmed_block_slots");

        let result = svm.get_first_available_block(api_state_accessor);
        assert_eq!(result.unwrap(), 3);
    });
}

#[test]
fn test_get_recent_prioritization_fees() {
    let runner = setup_svm_with_defaults();
    let mut rt = RT::default();
    let svm = &mut rt.svm;
    runner.query_state(|api_state_accessor| {
        // Case 1: No slot advances so we return 0 slot
        let result = svm.get_recent_prioritization_fees(None, api_state_accessor);
        assert_eq!(
            result.unwrap(),
            vec![RpcPrioritizationFee {
                slot: 0,
                prioritization_fee: 0,
            }]
        );

        // Case 2: Slot advances, so we return higher slot number
        svm.epoch_info
            .set(
                &EpochInfo {
                    epoch: 0,
                    slot_index: 0,
                    slots_in_epoch: 0,
                    absolute_slot: 4,
                    block_height: 0,
                    transaction_count: None,
                },
                api_state_accessor,
            )
            .expect("Failed to set epoch_info");

        let result = svm.get_recent_prioritization_fees(None, api_state_accessor);
        assert_eq!(
            result.unwrap(),
            vec![RpcPrioritizationFee {
                slot: 4,
                prioritization_fee: 0,
            }]
        );
    });
}

#[test]
fn test_get_signatures_for_address() {
    let runner = setup_svm_with_defaults();
    let mut rt = RT::default();
    let svm = &mut rt.svm;
    runner.query_state(|api_state_accessor| {
        let account1 = Keypair::new();
        let account2 = Keypair::new();
        let account3 = Keypair::new();
        let mut expected_result = Vec::with_capacity(5);
        for slot in 1..=5 {
            let some_hash = Hash::new_unique();
            let tx_data = ExecutedTransactionData {
                transaction: prepare_transaction_multiple_instructions(
                    &account1, &account2, &account3, some_hash,
                ),
                ..Default::default()
            };
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as UnixTimestamp;
            let block =
                create_test_block_with_txs(slot, some_hash, Some(now), vec![tx_data.clone()]);
            svm.confirmed_blocks
                .set(&slot, &block, api_state_accessor)
                .expect("Failed to set confirmed_blocks");
            svm.confirmed_block_slots
                .push(&slot, api_state_accessor)
                .expect("Failed to push to confirmed_block_slots");
            expected_result.push(RpcConfirmedTransactionStatusWithSignature {
                signature: tx_data.transaction.signatures[0].to_string(),
                slot,
                block_time: block.block_time,
                err: Some(TransactionError::AccountNotFound),
                memo: None,
                confirmation_status: None,
            });
            svm.confirmed_transactions
                .set(
                    &tx_data.transaction.signatures[0],
                    &ConfirmedTransactionDetails {
                        slot,
                        block_time: block.block_time,
                        transaction: tx_data.clone(),
                    },
                    api_state_accessor,
                )
                .expect("Failed to set confirmed_transactions");
        }
        // We reverse because the expected result is in reverse chronological order
        expected_result.reverse();
        let test_result =
            svm.get_signatures_for_address(account1.pubkey().into(), None, api_state_accessor);
        assert_eq!(test_result.unwrap(), expected_result);
    });
}

// Helper functions

/// Creates and returns a tuple of four unique `Pubkey` values.
fn create_test_pubkeys() -> (Pubkey, Pubkey, Pubkey, Pubkey) {
    (
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
    )
}

/// Creates and inserts four test accounts into the `SVM` and returns a tuple of four corresponding `Pubkey` values.
/// Accounts created:
/// 1. `pubkey_1`: 10,000 lamports, 5 bytes of data.
/// 2. `pubkey_2`: 12,345 lamports, 15 bytes of data.
/// 3. `pubkey_3`: 54,321 lamports, 12 bytes of data.
/// 4. `pubkey_4`: 11,111 lamports, 3 bytes of data.
fn create_test_accounts(
    api_state_accessor: &mut ApiStateAccessor<S>,
    svm: &mut SVM<S>,
) -> (Pubkey, Pubkey, Pubkey, Pubkey) {
    let (pubkey_1, pubkey_2, pubkey_3, pubkey_4) = create_test_pubkeys();

    svm.insert_account(
        &pubkey_1,
        &Account::new(10_000, 5, &native_loader::id()),
        api_state_accessor,
    )
    .unwrap();
    svm.insert_account(
        &pubkey_2,
        &Account::new(12_345, 15, &native_loader::id()),
        api_state_accessor,
    )
    .unwrap();
    svm.insert_account(
        &pubkey_3,
        &Account::new(54_321, 12, &native_loader::id()),
        api_state_accessor,
    )
    .unwrap();
    svm.insert_account(
        &pubkey_4,
        &Account::new(11_111, 3, &native_loader::id()),
        api_state_accessor,
    )
    .unwrap();

    (pubkey_1, pubkey_2, pubkey_3, pubkey_4)
}

/// Creates and returns a `Message` with a header, four unique account keys, a recent blockhash, and two dummy instructions.
fn create_test_message(recent_blockhash: Hash) -> String {
    let header = MessageHeader {
        num_required_signatures: 3,
        num_readonly_signed_accounts: 0,
        num_readonly_unsigned_accounts: 1,
    };
    let (pubkey_1, pubkey_2, pubkey_3, pubkey_4) = create_test_pubkeys();
    let account_keys = vec![pubkey_1, pubkey_2, pubkey_3, pubkey_4];

    let instructions = vec![
        CompiledInstruction {
            program_id_index: 1,
            accounts: vec![0],
            data: vec![4, 5, 6],
        },
        CompiledInstruction {
            program_id_index: 2,
            accounts: vec![0],
            data: vec![1, 1, 1, 1],
        },
    ];

    serialize_and_encode(
        &Message {
            header,
            account_keys,
            recent_blockhash,
            instructions,
        },
        UiTransactionEncoding::Base64,
    )
    .unwrap()
}

/// Prepares a `Transaction` that includes three instructions:
/// 1. Transfers 2,000,000 lamports from `acc1` to `acc2`.
/// 2. Creates a new account for `acc2` with 2,000,000 lamports, funded by `acc1`.
/// 3. Transfers 1,000,000 lamports from `acc2` to `acc3`.
fn prepare_transaction_multiple_instructions(
    acc1: &Keypair,
    acc2: &Keypair,
    acc3: &Keypair,
    blockhash: Hash,
) -> Transaction {
    let instruction1 = system_instruction::transfer(&acc1.pubkey(), &acc2.pubkey(), 2_000_000);
    let instruction2 = system_instruction::create_account(
        &acc1.pubkey(),
        &acc3.pubkey(),
        2_000_000,
        0,
        &system_program::id(),
    );
    let instruction3 = system_instruction::transfer(&acc2.pubkey(), &acc3.pubkey(), 1_000_000);

    Transaction::new_signed_with_payer(
        &[instruction1, instruction2, instruction3],
        Some(&acc1.pubkey()),
        &[acc1, acc2, acc3],
        blockhash,
    )
}

/// Creates and returns `ConfirmedBlockDetails` with empty transactions and provided slot and hash
fn create_test_block(
    slot: Slot,
    prev_hash: Hash,
    block_time: Option<UnixTimestamp>,
) -> ConfirmedBlockDetails {
    create_test_block_with_txs(slot, prev_hash, block_time, Vec::new())
}

fn create_test_block_with_txs(
    slot: Slot,
    prev_hash: Hash,
    block_time: Option<UnixTimestamp>,
    transactions: Vec<ExecutedTransactionData>,
) -> ConfirmedBlockDetails {
    let test_hash = hash(&prev_hash.to_bytes());
    ConfirmedBlockDetails {
        previous_blockhash: prev_hash.to_string(),
        blockhash: test_hash.to_string(),
        parent_slot: slot.saturating_sub(1),
        transactions,
        block_time,
        block_height: None,
    }
}
