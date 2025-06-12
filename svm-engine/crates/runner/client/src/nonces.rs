use futures::StreamExt;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    hash::{self, Hash},
    instruction::Instruction,
    message::Message,
    nonce::State,
    pubkey::{Pubkey, PubkeyError},
    signature::{Keypair, Signature},
    signer::Signer,
    system_instruction, system_program,
    transaction::Transaction,
};

/// Errors that can occur when working with nonce accounts.
#[derive(Debug, thiserror::Error)]
pub enum NonceAccountError {
    #[error("Nonce account is uninitialized")]
    Uninitialized,
    #[error("Nonce authority mismatch")]
    AuthorityMismatch,
    #[error("Nonce account not found")]
    NotFound,
    #[error("Failed to deserialize nonce state")]
    DeserializationError(#[from] bincode::Error),
    #[error("Failed to create account with seed {0}")]
    CreateSeedAccountError(#[from] solana_sdk::pubkey::PubkeyError),
    #[error("RPC error: {0}")]
    RpcError(#[from] solana_client::client_error::ClientError),
}

pub type NonceAccountResult<T = ()> = Result<T, NonceAccountError>;

fn create_seed(user: &Keypair, index: usize) -> String {
    let sha256_hash = hash::hashv(&[&user.pubkey().to_bytes(), &index.to_le_bytes()]).to_bytes();
    let md5_hash = md5::compute(sha256_hash);
    hex::encode(md5_hash.as_ref())
}

/// Derives the PDA Pubkey for the given nonce authority, user, and index.
pub fn find_nonce_account(
    nonce_authority: &Keypair,
    user: &Keypair,
    index: usize,
) -> Result<Pubkey, PubkeyError> {
    let seed = create_seed(user, index);
    Pubkey::create_with_seed(&nonce_authority.pubkey(), &seed, &system_program::id())
}

/// Creates a specified number of nonce accounts for a user and given nonce authority.
pub async fn create_nonce_accounts(
    rpc: &RpcClient,
    nonce_authority: &Keypair,
    user: &Keypair,
    count: usize,
) -> NonceAccountResult<(Vec<Pubkey>, Vec<Signature>)> {
    let nonce_account_state_size = State::size();
    let lamports = rpc
        .get_minimum_balance_for_rent_exemption(nonce_account_state_size)
        .await?;

    let mut creation_instructions = Vec::with_capacity(count * 2);
    let mut nonce_accounts = Vec::with_capacity(count);

    for i in 0..count {
        let nonce_account = find_nonce_account(nonce_authority, user, i)?;
        let seed = create_seed(user, i);
        let mut create_nonce_ixs = system_instruction::create_nonce_account_with_seed(
            &user.pubkey(),
            &nonce_account,
            &nonce_authority.pubkey(),
            &seed,
            &nonce_authority.pubkey(),
            lamports,
        );
        nonce_accounts.push(nonce_account);
        creation_instructions.append(&mut create_nonce_ixs);
    }

    let mut creation_transactions = Vec::with_capacity(count.div_ceil(4));
    let latest_blockhash = rpc.get_latest_blockhash().await?;

    // We chunk up to 4 nonce creation instructions at a time to avoid exceeding the maximum
    // transaction size.
    for ixs in creation_instructions.chunks(4) {
        let transaction = Transaction::new_signed_with_payer(
            ixs,
            Some(&user.pubkey()),
            &[nonce_authority, user],
            latest_blockhash,
        );
        creation_transactions.push(Box::pin(async move {
            rpc.send_and_confirm_transaction(&transaction).await
        }));
    }

    let signatures = futures::stream::iter(creation_transactions)
        .buffer_unordered(10)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    Ok((nonce_accounts, signatures))
}

/// Creates a durable transaction using the nonce authority and user keypairs.
/// Only the nonce authority signs the transaction, it is up to the user to sign it and send it
/// later.
pub async fn create_durable_transaction(
    rpc: &RpcClient,
    nonce_authority: &Keypair,
    user: &Keypair,
    index: usize,
    user_instructions: Vec<Instruction>,
) -> NonceAccountResult<(Transaction, Hash)> {
    let nonce_account = find_nonce_account(nonce_authority, user, index)?;
    let nonce_state = rpc.get_account_data(&nonce_account).await?;
    let nonce_state: State = bincode::deserialize(&nonce_state)?;

    let State::Initialized(data) = nonce_state else {
        return Err(NonceAccountError::Uninitialized);
    };

    if data.authority != nonce_authority.pubkey() {
        return Err(NonceAccountError::AuthorityMismatch);
    }

    let nonce_hash = data.blockhash();

    let message = Message::new_with_nonce(
        user_instructions,
        Some(&user.pubkey()),
        &nonce_account,
        &nonce_authority.pubkey(),
    );

    let mut transaction = Transaction::new_unsigned(message);
    transaction.partial_sign(&[nonce_authority], nonce_hash);

    Ok((transaction, nonce_hash))
}

#[cfg(test)]
mod tests {
    use solana_sdk::commitment_config::CommitmentConfig;

    use super::*;

    #[tokio::test]
    #[ignore = "Requires a local Solana cluster to be running"]
    async fn test_nonce_account_creation_succeeds() {
        let rpc = RpcClient::new_with_commitment(
            "http://localhost:8899".to_owned(),
            CommitmentConfig::confirmed(),
        );
        let nonce_authority = Keypair::new();
        let user = Keypair::new();

        rpc.request_airdrop(&nonce_authority.pubkey(), 1_000_000_000)
            .await
            .expect("Failed to airdrop SOL to nonce authority");

        rpc.request_airdrop(&user.pubkey(), 1_000_000_000)
            .await
            .expect("Failed to airdrop SOL to user");

        // Wait for the airdrop to be confirmed
        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

        let count = 10;

        let (nonce_accounts, signatures) =
            create_nonce_accounts(&rpc, &nonce_authority, &user, count)
                .await
                .expect("Failed to create nonce accounts");

        assert_eq!(nonce_accounts.len(), count);

        let statuses = rpc
            .get_signature_statuses(&signatures)
            .await
            .expect("Failed to get signature statuses");

        assert!(statuses.value.iter().all(|status| {
            let Some(status) = status else {
                return false;
            };

            status.err.is_none()
        }));
    }

    #[tokio::test]
    #[ignore = "Requires a local Solana cluster to be running"]
    async fn test_nonce_account_creation_fails() {
        let rpc = RpcClient::new_with_commitment(
            "http://localhost:8899".to_owned(),
            CommitmentConfig::confirmed(),
        );
        let nonce_authority = Keypair::new();
        let user = Keypair::new();

        rpc.request_airdrop(&nonce_authority.pubkey(), 1_000_000)
            .await
            .expect("Failed to airdrop SOL to nonce authority");

        rpc.request_airdrop(&user.pubkey(), 1_000_000)
            .await
            .expect("Failed to airdrop SOL to user");

        // Wait for the airdrop to be confirmed
        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

        let count = 2;

        create_nonce_accounts(&rpc, &nonce_authority, &user, count)
            .await
            .expect_err("Should faile to create nonce accounts");
    }
}
