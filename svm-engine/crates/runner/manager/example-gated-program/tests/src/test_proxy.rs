use std::sync::Arc;

use anchor_lang::{
    solana_program::instruction::Instruction, AccountDeserialize, InstructionData, ToAccountMetas,
};
use example_gated_program::{find_message_address, Message};
use proxy::{
    find_lock_address, find_lock_config_address, find_lock_signer_address, Lock, LockConfig,
};
use solana_sdk::{signer::Signer, transaction::Transaction};
use solana_test_validator::TestValidatorGenesis;

#[tokio::test]
async fn test_proxy_call() {
    solana_logger::setup_with_default("solana_runtime::message=debug");

    let program_id = proxy::id();
    let target_program_id = example_gated_program::id();

    let (validator, payer) = TestValidatorGenesis::default()
        .add_program("../target/deploy/example_gated_program", target_program_id)
        .add_program("../target/deploy/proxy", program_id)
        .start_async()
        .await;

    let payer = Arc::new(payer);
    let client = validator.get_async_rpc_client();
    let lock = find_lock_address(program_id, target_program_id);
    let lock_config = find_lock_config_address(program_id, target_program_id);
    let lock_duration = 10;

    let transaction = Transaction::new_signed_with_payer(
        &[Instruction {
            accounts: proxy::accounts::Initialize {
                lock,
                lock_config,
                target: target_program_id,
                payer: payer.pubkey(),
                system_program: solana_sdk::system_program::id(),
            }
            .to_account_metas(None),
            data: proxy::instruction::Initialize { lock_duration }.data(),
            program_id,
        }],
        Some(&payer.pubkey()),
        &[payer.clone()],
        client.get_latest_blockhash().await.unwrap(),
    );

    client
        .send_and_confirm_transaction(&transaction)
        .await
        .unwrap();

    let lock_account_data = client.get_account_data(&lock).await.unwrap();
    let lock_account = Lock::try_deserialize(&mut &lock_account_data[..]).unwrap();
    assert!(
        !lock_account.is_expired(),
        "Lock account should not be expired"
    );

    let lock_config_account_data = client.get_account_data(&lock_config).await.unwrap();
    let lock_config_account =
        LockConfig::try_deserialize(&mut &lock_config_account_data[..]).unwrap();
    assert_eq!(lock_config_account.lock_duration, lock_duration);

    let lock_duration = 20;

    let transaction = Transaction::new_signed_with_payer(
        &[Instruction {
            accounts: proxy::accounts::Configure {
                lock_config,
                target: target_program_id,
                payer: payer.pubkey(),
            }
            .to_account_metas(None),
            data: proxy::instruction::Configure { lock_duration }.data(),
            program_id,
        }],
        Some(&payer.pubkey()),
        &[payer.clone()],
        client.get_latest_blockhash().await.unwrap(),
    );

    client
        .send_and_confirm_transaction(&transaction)
        .await
        .unwrap();

    let lock_config_account_data = client.get_account_data(&lock_config).await.unwrap();
    let lock_config_account =
        LockConfig::try_deserialize(&mut &lock_config_account_data[..]).unwrap();
    assert_eq!(lock_config_account.lock_duration, lock_duration);

    let message = "Hello, world!".to_owned();
    let pda_signer = find_lock_signer_address(program_id, target_program_id);
    let message_data_account = find_message_address(target_program_id, payer.pubkey());
    let target_instruction = example_gated_program::instruction::Hello {
        message: message.clone(),
    };
    let target_instruction_accounts = example_gated_program::accounts::Hello {
        pda_signer,
        message: message_data_account,
        payer: payer.pubkey(),
        system_program: solana_sdk::system_program::id(),
    }
    .to_account_metas(None);

    let mut transaction = Transaction::new_with_payer(
        &[Instruction {
            accounts: target_instruction_accounts.clone(),
            data: target_instruction.data(),
            program_id: target_program_id,
        }],
        Some(&payer.pubkey()),
    );
    transaction
        .try_sign(
            &[payer.clone()],
            client.get_latest_blockhash().await.unwrap(),
        )
        .expect_err("Should fail because there are not enough signers");

    let mut target_accounts = target_instruction_accounts.clone();
    target_accounts.first_mut().unwrap().is_signer = false; // Make sure the PDA signer is not a signer
    let transaction = Transaction::new_signed_with_payer(
        &[Instruction {
            accounts: [
                proxy::accounts::Proxy {
                    pda_signer,
                    lock_config,
                    target: target_program_id,
                    lock,
                    payer: payer.pubkey(),
                    system_program: solana_sdk::system_program::id(),
                }
                .to_account_metas(None),
                target_accounts,
            ]
            .concat(),
            data: proxy::instruction::Proxy {
                instruction_data: target_instruction.data(),
                account_metas: bincode::serialize(&target_instruction_accounts).unwrap(),
            }
            .data(),
            program_id,
        }],
        Some(&payer.pubkey()),
        &[payer.clone()],
        client.get_latest_blockhash().await.unwrap(),
    );

    // This should succeed because the proxy has called the target program.
    client
        .send_and_confirm_transaction(&transaction)
        .await
        .unwrap();

    let message_account_data = client
        .get_account_data(&message_data_account)
        .await
        .unwrap();
    let message_account = Message::try_deserialize(&mut &message_account_data[..]).unwrap();
    assert_eq!(message_account.message, message);
}
