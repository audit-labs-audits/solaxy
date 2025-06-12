use std::{
    collections::{BTreeMap, HashSet},
    env,
    net::SocketAddr,
};

use jsonrpsee::server::Server;
use solana_sdk::{
    account::Account,
    clock::{Epoch, Slot},
    signature::keypair_from_seed,
    signer::Signer,
    system_program,
};
use solana_svm::transaction_processor::TransactionBatchProcessor;
use svm_engine::storage::{AccountsDB, SvmBankForks};
use tower::ServiceBuilder;

mod rpc;
use rpc::{ApiKeyLayer, RpcServer, Svm};

fn set_up_environment(processor: &TransactionBatchProcessor<SvmBankForks>) -> Svm<'_, AccountsDB> {
    // set up genesis accounts
    let seed: &[u8; 32] = b"an_example_fixed_seed_for_testia";
    let payer = keypair_from_seed(seed).unwrap();
    let seed: &[u8; 32] = b"an_example_fixed_seed_for_testib";
    let alice = keypair_from_seed(seed).unwrap();
    let seed: &[u8; 32] = b"an_example_fixed_seed_for_testin";
    let bob = keypair_from_seed(seed).unwrap();

    let genesis_accounts = BTreeMap::from_iter([
        (
            payer.pubkey(),
            Account::new(1_000_000_000, 0, &system_program::id()),
        ),
        (
            alice.pubkey(),
            Account::new(10_000_000, 0, &system_program::id()),
        ),
        (
            bob.pubkey(),
            Account::new(10_000_000, 0, &system_program::id()),
        ),
    ]);

    let db = AccountsDB::new(genesis_accounts);

    Svm::new(db, processor)
}

use once_cell::sync::Lazy;
static PROCESSOR: Lazy<TransactionBatchProcessor<SvmBankForks>> = Lazy::new(|| {
    TransactionBatchProcessor::<SvmBankForks>::new(
        Slot::default(),
        Epoch::default(),
        HashSet::default(),
    )
});

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let svm = set_up_environment(&PROCESSOR);

    // Create and start a new server
    let api_key = env::var("API_KEY").expect("API_KEY not found");
    let middleware = ServiceBuilder::new().layer(ApiKeyLayer::new(api_key));
    let addr: SocketAddr = "0.0.0.0:8080".parse()?;

    let server = Server::builder()
        .set_http_middleware(middleware)
        .build(addr)
        .await?;
    let handle = server.start(svm.into_rpc());

    // Let the server run and listen for incoming requests
    println!("Listening for requests...");
    handle.stopped().await;
    Ok(())
}
