use std::time::Duration;

use anyhow::anyhow;
use jsonrpsee::tracing::debug;
use solana_program::{program_option::COption, program_pack::Pack, pubkey::Pubkey};
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token::state::{Account as TokenAccount, Mint};
use tokio::time::sleep;

use crate::{client::TestClient, SLEEP_DURATION_SECONDS};

pub(crate) async fn create_mint(client: &TestClient, payer: &Keypair) -> anyhow::Result<Keypair> {
    debug!("Create Token Mint account");
    let mint_account = Keypair::new();

    let latest_blockhash = client
        .svm_get_latest_blockhash()
        .await
        .map_err(|e| anyhow!("Failed to retrieve the latest blockhash: {e}"))?;

    let rent_balance = client
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .await
        .map_err(|e| anyhow!("Failed to get a rent exempt balance: {e}"))?;

    let create_mint_account = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &mint_account.pubkey(),
        rent_balance,
        Mint::LEN as u64,
        &spl_token::id(),
    );

    let initialize_mint = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint_account.pubkey(),
        &payer.pubkey(), // mint authority
        None,
        0,
    )
    .map_err(|e| anyhow!("Failed to create initialize mint instruction: {e}"))?;

    let create_mint_tx = Transaction::new_signed_with_payer(
        &[create_mint_account, initialize_mint],
        Some(&payer.pubkey()),
        &[payer, &mint_account],
        latest_blockhash,
    );

    let tx_hash = client
        .svm_send_transaction(&create_mint_tx)
        .await
        .map_err(|e| anyhow!("Failed to send a transaction: {e}"))?
        .to_string();
    debug!("Send Transaction Hash: {tx_hash:?}");

    sleep(Duration::from_secs(SLEEP_DURATION_SECONDS)).await;

    let mint_acc_info = client
        .svm_get_account_info(mint_account.pubkey())
        .await
        .map_err(|e| anyhow!("Mint account should exist after transaction: {e}"))?;
    debug!("Mint account retrieved: {mint_acc_info:?}");

    // Ensure the mint account is owned by the SPL Token program
    assert_eq!(
        mint_acc_info.owner,
        spl_token::id(),
        "Mint account owner does not match"
    );

    // Deserialize the mint account data
    let mint_data = Mint::unpack(&mint_acc_info.data)
        .map_err(|e| anyhow!("Failed to unpack mint account data: {e}"))?;

    // Check: the mint account has the correct properties
    assert_eq!(
        mint_data.decimals, 0,
        "Expected mint data to have 0 decimals"
    );
    assert!(
        mint_data.mint_authority.is_some(),
        "Expected mint data to have mint authority but it is missing"
    );
    assert_eq!(
        mint_data.mint_authority,
        COption::Some(payer.pubkey()), // mint authority pubkey
        "Mint authority does not match"
    );
    assert_eq!(
        mint_data.freeze_authority,
        COption::None,
        "expected no freeze authority"
    );

    debug!("Mint account created successfully");

    Ok(mint_account)
}

pub(crate) async fn create_associated_token_account(
    client: &TestClient,
    payer: &Keypair,
    target_account_pubkey: &Pubkey,
    mint_account_pubkey: &Pubkey,
) -> anyhow::Result<()> {
    debug!("Create Associated Token Account");
    let latest_blockhash = client
        .svm_get_latest_blockhash()
        .await
        .map_err(|e| anyhow!("Failed to retrieve the latest blockhash: {e}"))?;

    let create_ata = spl_associated_token_account::instruction::create_associated_token_account(
        &payer.pubkey(),
        target_account_pubkey,
        mint_account_pubkey,
        &spl_token::id(),
    );

    let create_ata_tx = Transaction::new_signed_with_payer(
        &[create_ata],
        Some(&payer.pubkey()),
        &[&payer],
        latest_blockhash,
    );

    let tx_hash = client
        .svm_send_transaction(&create_ata_tx)
        .await
        .map_err(|e| anyhow!("Failed to send a transaction: {e}"))?
        .to_string();
    debug!("Send Transaction Hash: {tx_hash:?}");

    sleep(Duration::from_secs(SLEEP_DURATION_SECONDS)).await;

    let ata_pubkey = get_associated_token_address_with_program_id(
        target_account_pubkey,
        mint_account_pubkey,
        &spl_token::id(),
    );

    let ata_acc_info = client.svm_get_account_info(ata_pubkey).await.map_err(|e| {
        anyhow!("Failed to retrieve associated token account after transaction: {e}")
    })?;
    debug!("Associated token account created and retrieved: {ata_acc_info:?}");

    Ok(())
}

pub(crate) async fn mint_tokens(
    client: &TestClient,
    payer: &Keypair,
    target_account_pubkey: &Pubkey,
    mint_account_pubkey: &Pubkey,
    mint_authority: &Keypair,
    amount: u64,
) -> anyhow::Result<()> {
    debug!("Mint Tokens to {target_account_pubkey}");
    let latest_blockhash = client
        .svm_get_latest_blockhash()
        .await
        .map_err(|e| anyhow!("Failed to retrieve the latest blockhash: {e}"))?;

    let ata_pubkey = get_associated_token_address_with_program_id(
        target_account_pubkey,
        mint_account_pubkey,
        &spl_token::id(),
    );

    let mint_to = spl_token::instruction::mint_to_checked(
        &spl_token::id(),
        mint_account_pubkey,
        &ata_pubkey,
        &mint_authority.pubkey(),
        &[],
        amount, // Amount to mint (100 tokens, given 0 decimals)
        0,
    )
    .map_err(|e| anyhow!("Failed to create mint to checked instruction: {e}"))?;

    let mint_to_tx = Transaction::new_signed_with_payer(
        &[mint_to],
        Some(&payer.pubkey()),
        &[&payer, &mint_authority],
        latest_blockhash,
    );

    let tx_hash = client
        .svm_send_transaction(&mint_to_tx)
        .await
        .map_err(|e| anyhow!("Failed to send a transaction: {e}"))?
        .to_string();
    debug!("Send Transaction Hash: {tx_hash:?}");

    sleep(Duration::from_secs(SLEEP_DURATION_SECONDS)).await;

    let ata_pubkey = get_associated_token_address_with_program_id(
        target_account_pubkey,
        mint_account_pubkey,
        &spl_token::id(),
    );

    let ata_acc_info = client.svm_get_account_info(ata_pubkey).await.map_err(|e| {
        anyhow!("Failed to retrieve associated token account after transaction: {e}")
    })?;
    debug!("Associated token account retrieved: {ata_acc_info:?}");

    let token_account_data = TokenAccount::unpack(&ata_acc_info.data)
        .map_err(|e| anyhow!("Failed to unpack token account data: {e}"))?;

    // Ensure alice has received 100 tokens from mint
    assert_eq!(
        token_account_data.amount, 100,
        "expected target token account to have received 100 tokens"
    );
    debug!("Token data: {token_account_data:?}");
    Ok(())
}

pub(crate) async fn transfer_tokens(
    client: &TestClient,
    payer: &Keypair,
    sender: &Keypair,
    receiver_account_pubkey: &Pubkey,
    mint_account_pubkey: &Pubkey,
    amount: u64,
) -> anyhow::Result<()> {
    let sender_key = sender.pubkey();
    debug!("Transfer Tokens from {sender_key} to {receiver_account_pubkey}",);
    let latest_blockhash = client
        .svm_get_latest_blockhash()
        .await
        .map_err(|e| anyhow!("Failed to retrieve the latest blockhash: {e}"))?;

    let sender_ata_pubkey = get_associated_token_address_with_program_id(
        &sender_key,
        mint_account_pubkey,
        &spl_token::id(),
    );

    let receiver_ata_pubkey = get_associated_token_address_with_program_id(
        receiver_account_pubkey,
        mint_account_pubkey,
        &spl_token::id(),
    );

    let sender_ata_acc_info = client
        .svm_get_account_info(sender_ata_pubkey)
        .await
        .map_err(|e| anyhow!("Failed to retrieve account {sender_ata_pubkey}: {e}"))?;
    let sender_token_account_data = TokenAccount::unpack(&sender_ata_acc_info.data)
        .map_err(|e| anyhow!("Failed to unpack sender's token account data: {e}"))?;
    debug!("Sender's token account data:: {sender_token_account_data:?}");

    let receiver_ata_acc_info = client
        .svm_get_account_info(receiver_ata_pubkey)
        .await
        .map_err(|e| anyhow!("failed to retrieve account {receiver_ata_pubkey}: {e}"))?;
    let receiver_token_account_data = TokenAccount::unpack(&receiver_ata_acc_info.data)
        .map_err(|e| anyhow!("Failed to unpack receiver's token account data: {e}"))?;
    debug!("Receiver's token account data: {receiver_token_account_data:?}");

    // Get initial balances for tests
    let initial_sender_tokens = sender_token_account_data.amount;
    let initial_receiver_tokens = receiver_token_account_data.amount;

    let transfer = spl_token::instruction::transfer_checked(
        &spl_token::id(),
        &sender_ata_pubkey,
        mint_account_pubkey,
        &receiver_ata_pubkey,
        &sender_key,
        &[],
        amount,
        0,
    )
    .map_err(|e| anyhow!("Failed to create transfer checked instruction: {e}"))?;

    let transfer_tx = Transaction::new_signed_with_payer(
        &[transfer],
        Some(&payer.pubkey()),
        &[&payer, &sender],
        latest_blockhash,
    );

    let tx_hash = client
        .svm_send_transaction(&transfer_tx)
        .await
        .map_err(|e| anyhow!("Failed to send a transaction: {e}"))?
        .to_string();
    debug!("Send Transaction Hash: {tx_hash:?}");

    sleep(Duration::from_secs(SLEEP_DURATION_SECONDS)).await;

    let sender_acc_info = client
        .svm_get_account_info(sender_ata_pubkey)
        .await
        .map_err(|e| anyhow!("Failed to retrieve account {sender_ata_pubkey}: {e}"))?;
    let sender_token_account_data = TokenAccount::unpack(&sender_acc_info.data)
        .map_err(|e| anyhow!("Failed to unpack sender's token account data: {e}"))?;
    debug!("Sender's token account data: {sender_token_account_data:?}");

    assert_eq!(
        sender_token_account_data.amount,
        initial_sender_tokens - amount,
        "Sender's expected token balance is incorrect"
    );

    let receiver_acc_info = client
        .svm_get_account_info(receiver_ata_pubkey)
        .await
        .map_err(|e| anyhow!("Failed to retrieve account {receiver_ata_pubkey}: {e}"))?;
    let receiver_token_account_data = TokenAccount::unpack(&receiver_acc_info.data)
        .map_err(|e| anyhow!("Failed to unpack receiver's token account data: {e}"))?;
    debug!("Receiver's token account data: {receiver_token_account_data:?}");

    assert_eq!(
        receiver_token_account_data.amount,
        initial_receiver_tokens + amount,
        "Sender's expected token balance is incorrect"
    );

    Ok(())
}

pub(crate) async fn burn_tokens(
    client: &TestClient,
    payer: &Keypair,
    target_account: &Keypair,
    mint_account_pubkey: &Pubkey,
    amount: u64,
) -> anyhow::Result<()> {
    let target_key = target_account.pubkey();
    debug!("Burn Tokens from {target_key}");
    let latest_blockhash = client
        .svm_get_latest_blockhash()
        .await
        .map_err(|e| anyhow!("Failed to retrieve the latest blockhash: {e}"))?;

    let target_ata_pubkey = get_associated_token_address_with_program_id(
        &target_key,
        mint_account_pubkey,
        &spl_token::id(),
    );

    let target_acc_info = client
        .svm_get_account_info(target_ata_pubkey)
        .await
        .map_err(|e| anyhow!("failed to retrieve account {target_ata_pubkey}: {e}"))?;
    let target_token_account_data = TokenAccount::unpack(&target_acc_info.data)
        .map_err(|e| anyhow!("Failed to unpack token account data: {e}"))?;
    debug!("Target's token account data:: {target_token_account_data:?}");

    let initial_target_tokens = target_token_account_data.amount;

    let burn = spl_token::instruction::burn_checked(
        &spl_token::id(),
        &target_ata_pubkey,
        mint_account_pubkey,
        &target_key,
        &[],
        amount,
        0,
    )
    .map_err(|e| anyhow!("Failed to create burn tx: {e}"))?;

    let burn_tx = Transaction::new_signed_with_payer(
        &[burn],
        Some(&payer.pubkey()),
        &[&payer, &target_account],
        latest_blockhash,
    );

    let tx_hash = client
        .svm_send_transaction(&burn_tx)
        .await
        .map_err(|e| anyhow!("Failed to send a transaction: {e}"))?
        .to_string();
    debug!("Send Transaction Hash: {tx_hash:?}");

    sleep(Duration::from_secs(SLEEP_DURATION_SECONDS)).await;

    let target_acc_info = client
        .svm_get_account_info(target_ata_pubkey)
        .await
        .map_err(|e| anyhow!("failed to retrieve account {target_ata_pubkey}: {e}"))?;
    let target_token_account_data = TokenAccount::unpack(&target_acc_info.data)
        .map_err(|e| anyhow!("Failed to unpack token account data: {e}"))?;
    debug!("Sender's token account data: {target_token_account_data:?}");

    assert_eq!(
        target_token_account_data.amount,
        initial_target_tokens - amount,
        "Target's expected token balance is incorrect"
    );

    Ok(())
}

pub(crate) async fn close_token_account(
    client: &TestClient,
    payer: &Keypair,
    target_account: &Keypair,
    mint_account_pubkey: &Pubkey,
) -> anyhow::Result<()> {
    let target_key = target_account.pubkey();
    debug!("Close token account under pubkey {target_key}");
    let latest_blockhash = client
        .svm_get_latest_blockhash()
        .await
        .map_err(|e| anyhow!("Failed to retrieve the latest blockhash: {e}"))?;

    let target_ata_pubkey = get_associated_token_address_with_program_id(
        &target_key,
        mint_account_pubkey,
        &spl_token::id(),
    );

    let initial_payer_balance = client
        .svm_get_account_balance(payer.pubkey())
        .await
        .map_err(|e| anyhow!("Failed to get payer's initial balance: {e}"))?;

    let target_ata_balance = client
        .svm_get_account_balance(target_ata_pubkey)
        .await
        .map_err(|e| anyhow!("Failed to get balance of ata account: {e}"))?;

    let close = spl_token::instruction::close_account(
        &spl_token::id(),
        &target_ata_pubkey,
        &payer.pubkey(),
        &target_key,
        &[],
    )
    .map_err(|e| anyhow!("Failed to create close token account tx: {e}"))?;

    let close_tx = Transaction::new_signed_with_payer(
        &[close],
        Some(&payer.pubkey()),
        &[&payer, &target_account],
        latest_blockhash,
    );

    let total_gas_fee_estimate = client
        .get_total_fee_for_transaction(&close_tx)
        .await
        .map_err(|e| anyhow!("Failed to get total fee estimate for transaction: {e}"))?;

    let tx_hash = client
        .svm_send_transaction(&close_tx)
        .await
        .map_err(|e| anyhow!("Failed to send a transaction: {e}"))?
        .to_string();
    debug!("Send Transaction Hash: {tx_hash:?}");

    sleep(Duration::from_secs(SLEEP_DURATION_SECONDS)).await;

    let target_account_response = client.svm_get_account_info(target_ata_pubkey).await;
    assert!(
        target_account_response.is_err(),
        "Expected token account to be closed and purged"
    );

    let payer_balance = client
        .svm_get_account_balance(payer.pubkey())
        .await
        .map_err(|e| anyhow!("Failed to get payer's post balance: {e}"))?;
    // initial balance + ata balance - gas fees
    let expected_balance = initial_payer_balance + target_ata_balance - total_gas_fee_estimate;
    debug!("Payer's balance: {payer_balance}; Expected balance: {expected_balance}",);
    assert_eq!(
        payer_balance, expected_balance,
        "Expected payer account's balance to be {expected_balance} but found {payer_balance}"
    );

    Ok(())
}

pub(crate) async fn spl_token_scenario(client: &TestClient, payer: &Keypair) -> anyhow::Result<()> {
    debug!("SPL-TOKEN program scenario");
    let mint_account = create_mint(client, payer)
        .await
        .map_err(|e| anyhow!("Failed to create mint account: {e}"))?;

    let alice = Keypair::new();
    let bob = Keypair::new();

    create_associated_token_account(client, payer, &alice.pubkey(), &mint_account.pubkey())
        .await
        .map_err(|e| anyhow!("Failed to create Alice associated token account: {e}"))?;
    debug!("Created Alice token account");

    mint_tokens(
        client,
        payer,
        &alice.pubkey(),
        &mint_account.pubkey(),
        payer,
        100,
    )
    .await
    .map_err(|e| anyhow!("Failed to mint tokens: {e}"))?;
    debug!("Minted 100 tokens to Alice");

    create_associated_token_account(client, payer, &bob.pubkey(), &mint_account.pubkey())
        .await
        .map_err(|e| anyhow!("Failed to create Bob associated token account: {e}"))?;
    debug!("Created Bob token account");

    transfer_tokens(
        client,
        payer,
        &alice,
        &bob.pubkey(),
        &mint_account.pubkey(),
        10,
    )
    .await
    .map_err(|e| anyhow!("Failed to transfer tokens from ALice to Bob: {e}"))?;
    debug!("Transfer from Alice to Bob succeeded");

    burn_tokens(client, payer, &alice, &mint_account.pubkey(), 20)
        .await
        .map_err(|e| anyhow!("Failed to burn Alice tokens: {e}"))?;
    debug!("Burned 20 tokens from Alice");

    // Only token accounts with zero tokens can be closed. Therefore, do transfer first
    transfer_tokens(
        client,
        payer,
        &bob,
        &alice.pubkey(),
        &mint_account.pubkey(),
        10,
    )
    .await
    .map_err(|e| anyhow!("Failed to transfer tokens from Bob to Alice: {e}"))?;
    debug!("Transfer from Bob to Alice succeeded");

    close_token_account(client, payer, &bob, &mint_account.pubkey())
        .await
        .map_err(|e| anyhow!("Failed to close Bob token account: {e}"))?;
    debug!("Closed Bob's token account and purged from state successfully");

    Ok(())
}
