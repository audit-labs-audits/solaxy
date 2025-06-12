use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    hash::Hash,
    instruction::Instruction,
    message::Message,
    nonce::State,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    system_instruction, system_program,
    transaction::Transaction,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// 1) Build & return an unsigned tx (fee-payer = nonce authority)
pub async fn build_unsigned_tx(
    rpc: &RpcClient,
    nonce_authority: &Keypair,
    user_pubkey: &Pubkey,
    user_instructions: Vec<Instruction>,
) -> Result<(Transaction, Hash)> {
    let nonce_account = Pubkey::find_program_address(
        &[nonce_authority.pubkey().as_ref(), user_pubkey.as_ref()],
        &system_program::id(),
    )
    .0;
    let lamports = rpc
        .get_minimum_balance_for_rent_exemption(State::size())
        .await?;
    let create_nonce_ixs = system_instruction::create_nonce_account(
        &nonce_authority.pubkey(),
        &nonce_account,
        &nonce_authority.pubkey(),
        lamports,
    );

    let create_nonce_tx = Transaction::new_signed_with_payer(
        &create_nonce_ixs,
        Some(&nonce_authority.pubkey()),
        &[nonce_authority],
        rpc.get_latest_blockhash().await?,
    );

    let _nonce_tx_sig = send_tx(rpc, &create_nonce_tx).await?;

    let data = rpc.get_account_data(&nonce_account).await?;

    let nonce_state: State = bincode::deserialize(&data)?;

    let nonce_hash = match nonce_state {
        State::Uninitialized => {
            return Err("Nonce account is uninitialized".into());
        }
        State::Initialized(data) => {
            if data.authority != nonce_authority.pubkey() {
                return Err("Nonce authority mismatch".into());
            }
            data.blockhash()
        }
    };

    let close_nonce_ix = system_instruction::withdraw_nonce_account(
        &nonce_account,
        &nonce_authority.pubkey(),
        &nonce_authority.pubkey(),
        lamports,
    );

    let mut instructions = user_instructions;
    instructions.push(close_nonce_ix);

    let message = Message::new_with_nonce(
        instructions,
        Some(&nonce_authority.pubkey()),
        &nonce_account,
        &nonce_authority.pubkey(),
    );

    let tx = Transaction::new_unsigned(message);
    Ok((tx, nonce_hash))
}

pub fn user_sign_tx(tx: &mut Transaction, user: &Keypair, nonce_hash: Hash) {
    tx.partial_sign(&[user], nonce_hash);
}

pub fn nonce_authority_sign_tx(tx: &mut Transaction, nonce_authority: &Keypair, nonce_hash: Hash) {
    tx.partial_sign(&[nonce_authority], nonce_hash);
}

pub async fn send_tx(rpc: &RpcClient, tx: &Transaction) -> Result<Signature> {
    let sig = rpc.send_and_confirm_transaction(tx).await?;
    Ok(sig)
}

// ——————————————————————————
// Example wiring in `main`:

#[tokio::main]
async fn main() -> Result<()> {
    // Setup
    let rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_owned());
    let nonce_authority = Keypair::new();
    let user = Keypair::new();

    // Your app-specific instructions (e.g. token transfers, program calls…)
    let user_ixs: Vec<Instruction> = vec![
        /* ... */
    ];

    // 1) Build
    let (mut tx, nonce_hash) =
        build_unsigned_tx(&rpc, &nonce_authority, &user.pubkey(), user_ixs).await?;

    // 2) User signs (e.g. wallet adapter)
    user_sign_tx(&mut tx, &user, nonce_hash);

    // [Off‑chain simulation / business logic here…]

    // 3) Nonce authority signs
    nonce_authority_sign_tx(&mut tx, &nonce_authority, nonce_hash);

    // 4) Send
    let signature = send_tx(&rpc, &tx).await?;
    println!("Transaction sent: {}", signature);

    Ok(())
}
