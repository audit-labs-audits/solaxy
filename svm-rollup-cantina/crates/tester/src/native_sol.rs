use std::time::Duration;

use anyhow::anyhow;
use jsonrpsee::tracing::debug;
use solana_program::system_program;
use solana_sdk::{
    account::Account,
    signature::{Keypair, Signer},
};
use svm::test_utils::{prepare_new_account_transaction, prepare_sol_transfer_transaction};
use tokio::time::sleep;

use crate::{client::TestClient, SLEEP_DURATION_SECONDS};

pub(crate) async fn create_account(
    client: &TestClient,
    payer: &Keypair,
    new_account: &Keypair,
    amount: u64,
) -> anyhow::Result<()> {
    debug!("Create new account");
    // Get initial balance
    let payer_initial_balance = client
        .svm_get_account_balance(payer.pubkey())
        .await
        .map_err(|e| anyhow!("Failed to get payer's initial balance: {e}"))?;

    // Get latest blockhash
    let latest_blockhash = client
        .svm_get_latest_blockhash()
        .await
        .map_err(|e| anyhow!("Failed to retrieve the latest blockhash: {e}"))?;

    let create_acc_tx =
        prepare_new_account_transaction(payer, new_account, amount, latest_blockhash);

    let total_gas_fee_estimate = client
        .get_total_fee_for_transaction(&create_acc_tx)
        .await
        .map_err(|e| anyhow!("Failed to get total fee estimate for transaction: {e}"))?;

    let tx_hash = client
        .svm_send_transaction(&create_acc_tx)
        .await
        .map_err(|e| anyhow!("Failed to send a transaction: {e}"))?
        .to_string();
    debug!("Send Transaction Hash: {tx_hash}");

    // Wait for transactions to process and persist changes
    sleep(Duration::from_secs(SLEEP_DURATION_SECONDS)).await;

    // Check accounts and balances
    // Check payer's post balance
    let payer_post_balance = client
        .svm_get_account_balance(payer.pubkey())
        .await
        .map_err(|e| anyhow!("Failed to get payer's post balance: {e}"))?;
    let expected_balance = payer_initial_balance - amount - total_gas_fee_estimate;
    debug!("Payer's balance: {payer_post_balance}; Expected balance: {expected_balance}");
    assert_eq!(
        payer_post_balance, expected_balance,
        "Expected payer account's balance to be {expected_balance} but found {payer_post_balance}"
    );

    // Check new account's post balance
    let new_acc_balance = client
        .svm_get_account_balance(new_account.pubkey())
        .await
        .map_err(|e| anyhow!("Failed to get new account's balance: {e}"))?;
    debug!("New account's balance: {new_acc_balance}; Expected balance: {amount}");
    assert_eq!(
        new_acc_balance, amount,
        "Expected new account's balance to be {amount} but found {new_acc_balance}"
    );

    // Check new account's info
    let new_acc_info = client
        .svm_get_account_info(new_account.pubkey())
        .await
        .map_err(|e| anyhow!("Failed to retrieve new account: {e}"))?;
    let new_acc_key = new_account.pubkey();
    debug!("New account: {new_acc_key}\n{new_acc_info:?}");

    let expected_account_info = Account {
        lamports: amount,
        owner: system_program::id(),
        data: Vec::new(),
        executable: false,
        rent_epoch: new_acc_info.rent_epoch, // we do not have a function for retrieving rent epoch
    };
    assert_eq!(
        new_acc_info, expected_account_info,
        "New account's info does not match with expected info"
    );

    Ok(())
}

pub(crate) async fn transfer_sol(
    client: &TestClient,
    sender: &Keypair,
    receiver: &Keypair,
    amount: u64,
) -> anyhow::Result<()> {
    debug!("Transfer SOL");
    // Get initial balances
    let sender_initial_balance = client
        .svm_get_account_balance(sender.pubkey())
        .await
        .map_err(|e| anyhow!("Failed to get sender's initial balance: {e}"))?;
    let receiver_initial_balance = client
        .svm_get_account_balance(receiver.pubkey())
        .await
        .map_err(|e| anyhow!("Failed to get receiver's initial balance: {e}"))?;

    // Get latest blockhash
    let latest_blockhash = client
        .svm_get_latest_blockhash()
        .await
        .map_err(|e| anyhow!("Failed to retrieve the latest blockhash: {e}"))?;

    // Send and publish transaction
    let sol_transfer_tx =
        prepare_sol_transfer_transaction(sender, &receiver.pubkey(), amount, latest_blockhash);

    let total_gas_fee_estimate = client
        .get_total_fee_for_transaction(&sol_transfer_tx)
        .await
        .map_err(|e| anyhow!("Failed to get total fee estimate for transaction: {e}"))?;

    let tx_hash = client
        .svm_send_transaction(&sol_transfer_tx)
        .await
        .map_err(|e| anyhow!("Failed to send a transaction: {e}"))?
        .to_string();
    debug!("Send Transaction Hash: {tx_hash}");

    // Wait for transactions to process and persist changes
    sleep(Duration::from_secs(SLEEP_DURATION_SECONDS)).await;

    // Check balances post transfer
    // Check sender's post balance
    let sender_post_balance = client
        .svm_get_account_balance(sender.pubkey())
        .await
        .map_err(|e| anyhow!("Failed to get sender's post balance: {e}"))?;
    let expected_balance = sender_initial_balance - amount - total_gas_fee_estimate;
    debug!("Sender's balance: {sender_post_balance}; Expected balance: {expected_balance}");
    assert_eq!(
        sender_post_balance, expected_balance,
        "Expected sender's balance to be {expected_balance} but found {sender_post_balance}"
    );

    // Check receiver's post balance
    let receiver_post_balance = client
        .svm_get_account_balance(receiver.pubkey())
        .await
        .map_err(|e| anyhow!("Failed to get receiver's post balance: {e}"))?;
    let expected_balance = receiver_initial_balance + amount;
    debug!("Receiver's balance: {receiver_post_balance}; Expected balance: {expected_balance}");
    assert_eq!(
        receiver_post_balance, expected_balance,
        "Expected receiver's balance to be {expected_balance} but found {receiver_post_balance}",
    );

    Ok(())
}

// TODO: change implementation to close account
#[allow(dead_code)]
pub(crate) async fn close_account(
    client: &TestClient,
    sender: &Keypair,
    receiver: &Keypair,
) -> anyhow::Result<()> {
    debug!("Close Account");
    // Get initial balances
    let sender_initial_balance = client
        .svm_get_account_balance(sender.pubkey())
        .await
        .map_err(|e| anyhow!("Failed to get sender's initial balance: {e}"))?;
    let receiver_initial_balance = client
        .svm_get_account_balance(receiver.pubkey())
        .await
        .map_err(|e| anyhow!("Failed to get receiver's initial balance: {e}"))?;

    // Get latest blockhash
    let latest_blockhash = client
        .svm_get_latest_blockhash()
        .await
        .map_err(|e| anyhow!("Failed to retrieve the latest blockhash: {e}"))?;

    // Send and publish transaction
    let test_sol_transfer_tx = prepare_sol_transfer_transaction(
        sender,
        &receiver.pubkey(),
        sender_initial_balance - 5000,
        latest_blockhash,
    );

    let total_gas_fee_estimate = client
        .get_total_fee_for_transaction(&test_sol_transfer_tx)
        .await
        .map_err(|e| anyhow!("Failed to get total fee estimate for transaction: {e}"))?;

    let sol_transfer_tx = prepare_sol_transfer_transaction(
        sender,
        &receiver.pubkey(),
        sender_initial_balance - total_gas_fee_estimate,
        latest_blockhash,
    );

    let tx_hash = client
        .svm_send_transaction(&sol_transfer_tx)
        .await
        .map_err(|e| anyhow!("Failed to send a transaction: {e}"))?
        .to_string();
    debug!("Send Transaction Hash: {tx_hash}");

    // Wait for transactions to process and persist changes
    sleep(Duration::from_secs(SLEEP_DURATION_SECONDS)).await;

    // Check balances post transfer
    // Check sender's post balance
    // If transfer amount is sender's balance - gas fees for transfer, then account has to be purged
    let get_account_result = client.svm_get_account_info(sender.pubkey()).await;
    debug!("{get_account_result:?}");
    assert!(
        get_account_result.is_err(),
        "Sender's account expected to be purged"
    );
    debug!("Sender's account was drained and purged i.e. closed successfully");

    // Check receiver's post balance
    let receiver_post_balance = client
        .svm_get_account_balance(receiver.pubkey())
        .await
        .map_err(|e| anyhow!("Failed to get receiver's post balance: {e}"))?;
    let expected_balance =
        receiver_initial_balance + (sender_initial_balance - total_gas_fee_estimate);
    debug!("Receiver's balance: {receiver_post_balance}; Expected balance: {expected_balance}");
    assert_eq!(
        receiver_post_balance, expected_balance,
        "Expected receiver's balance to be {expected_balance} but found {receiver_post_balance}",
    );

    Ok(())
}

pub(crate) async fn native_sol_scenario(
    client: &TestClient,
    payer1: &Keypair,
    payer2: &Keypair,
) -> anyhow::Result<()> {
    debug!("Native SOL scenario");
    // Transfer SOL
    transfer_sol(client, payer1, payer2, 10_000)
        .await
        .map_err(|e| anyhow!("Failed to transfer sol: {e}"))?;
    debug!("Transfer from payer1 to payer2 succeeded");

    // Create new account
    let new_account = Keypair::new();
    // Calculate balance for rent exemption + gas fees to pay gas fees for transfer of all funds
    let amount = client
        .get_minimum_balance_for_rent_exemption(0)
        .await
        .map_err(|e| anyhow!("Failed to get a rent exempt balance: {e}"))?
        + 5000;
    create_account(client, payer1, &new_account, amount).await?;
    debug!("Created new account successfully");

    // TODO: change implementation to close account
    // Close account
    // close_account(client, &new_account, payer1)
    //     .await
    //     .map_err(|e| anyhow!("Failed to drain and close account: {e}"))?;
    // debug!("Successfully closed new account");

    Ok(())
}
