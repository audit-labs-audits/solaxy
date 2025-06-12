use std::{fs, path::Path};

use anyhow::anyhow;
use client::TestClient;
use jsonrpsee::{core::__reexports::serde::Deserialize, tracing::debug};
use solana_sdk::signature::{read_keypair_file, Keypair};

use crate::{
    custom_programs::{counter_program_scenario, hello_world_program_scenario},
    native_sol::native_sol_scenario,
    spl_token::spl_token_scenario,
    spl_token_2022::spl_token_2022_scenario,
};

mod client;
mod custom_programs;
mod native_sol;
mod spl_token;
mod spl_token_2022;

// Sleep duration between transactions in seconds
static SLEEP_DURATION_SECONDS: u64 = 2;

#[derive(Deserialize, Debug)]
struct Config {
    rpc: RpcConfig,
    accounts: AccountsConfig,
}

#[derive(Debug, Deserialize)]
pub struct AccountsConfig {
    pub keypairs: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct RpcConfig {
    http_addr: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    // Initialize a config from config.toml
    let config = load_config("config.toml")?;
    debug!("Loaded configuration: {:?}", config);

    // Initialize a client to interact with rollup node
    let client = TestClient::new(config.rpc.http_addr).await;

    // Check SVM state response
    let response = client.svm_query_state().await.map_err(|e| anyhow!("{e}"))?;
    debug!("Query State Response: {}", response);

    // load saved keypairs for accounts initialized in a mock rollup from '../test-data/genesis/demo/mock/keypairs/'
    // each account's balance is 1 SOL (1_000_000_000 lamports)
    let keypairs: Vec<Keypair> = config
        .accounts
        .keypairs
        .into_iter()
        .map(|path| load_keypair(format!("../test-data/genesis/demo/mock/keypairs/{path}")))
        .collect();

    native_sol_scenario(&client, &keypairs[0], &keypairs[1]).await?;

    spl_token_2022_scenario(&client, &keypairs[0]).await?;

    spl_token_scenario(&client, &keypairs[1]).await?;

    hello_world_program_scenario(&client, &keypairs[2]).await?;

    counter_program_scenario(&client, &keypairs[3]).await?;

    Ok(())
}

/// Loads config from path to initialize API client and a list of keypairs
fn load_config(path: &str) -> anyhow::Result<Config> {
    let config_str = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&config_str)?;
    Ok(config)
}

/// Loads keypair from JSON file
pub fn load_keypair<P: AsRef<Path>>(path: P) -> Keypair {
    // Read the keypair from the JSON file
    read_keypair_file(path).expect("Failed to read keypair file") // Return the public key (program ID)
}
