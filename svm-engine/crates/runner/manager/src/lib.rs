use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};

use jsonrpsee::server::ServerBuilder;
use rpc::RpcInstantiator;
use solana_sdk::{
    account::Account,
    clock::{Epoch, Slot},
    pubkey::Pubkey,
};
use solana_svm::transaction_processor::TransactionBatchProcessor;
use storage::Persistable;
use svm_engine::{
    Engine,
    programs::{Program, initialize_programs},
    storage::{SVMStorage, SvmBankForks},
};
use svm_runner::{Runner, StoreTransactionStatus};
use svm_runner_block_builder::BlockBuilder;
use svm_runner_relay::Relay;
use svm_runner_rpc::RunnerRpcServer;
use svm_runner_syncer::Syncer;
use svm_runner_tx_queue::{TxQueueReaderServer, TxQueueWriter};
use tokio::sync::mpsc;

pub mod builder;
pub mod queue;
pub mod relay;
pub mod rpc;
pub mod storage;
pub mod syncer;

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub storage_file: String,
    pub programs_to_load: Vec<Program>,
    pub persist_interval_s: u64,
    pub accounts_to_load: HashMap<Pubkey, Account>,
}

#[derive(Debug, Clone, Default)]
pub struct ProcessorConfig {
    pub slot: Slot,
    pub epoch: Epoch,
    pub program_ids: HashSet<Pubkey>,
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub storage_config: StorageConfig,
    pub processor_config: ProcessorConfig,
    pub rpc_addr: String,
}

#[derive(Debug, Clone)]
pub struct TxQueueConfig {
    pub rpc_addr: String,
}

#[derive(Debug, Clone)]
pub struct ManagerConfig {
    pub engine_config: EngineConfig,
    pub tx_queue_config: TxQueueConfig,
    pub queue_size: usize,
    pub rpc_addr: String,
}

/// A trait that defines the SVM Engine runner system.
/// It is meant to be a scaffold for assembling different parts of the system together and coordinating
/// their interactions. This approach allows the users of the trait to simply list the components
/// which adhere to the trait's associated type bounds and then use the `coordinate_tasks` method
/// to run the system.
#[async_trait::async_trait]
pub trait SVMManager {
    /// The queue type used for transaction storing and retrieval.
    type Queue: TxQueueWriter + TxQueueReaderServer + Default + Clone;
    /// The RPC type used for accepting transactions into the system.
    type Rpc: RunnerRpcServer + RpcInstantiator<Self::Storage>;
    /// The storage type used for persisting the state of the system.
    type Storage: SVMStorage + Persistable + StoreTransactionStatus + Clone + Send + Sync + 'static;
    /// The block builder to use for building blocks.
    type Builder: BlockBuilder + Default + Send + Sync + 'static;
    /// The data syncer used to synchronize data from the L1 to the SVM Engine.
    type SvmSyncer: Syncer<Self::Storage> + Send + Sync + 'static;
    /// The relay type used for relaying transactions to the L1 from the TxQueue.
    type SvmRelay: Relay + Default + Send + Sync + 'static;

    /// Creates a config to be used when coordinating the tasks.
    async fn create_config() -> ManagerConfig;

    /// Coordinates all the processes involved in the SVM Engine runner system.
    async fn coordinate_tasks(accounts_to_load: HashMap<Pubkey, Account>) {
        static PROCESSOR: OnceLock<TransactionBatchProcessor<SvmBankForks>> = OnceLock::new();
        let config = Self::create_config().await;

        // Load the storage from a file
        let processor = PROCESSOR.get_or_init(|| {
            TransactionBatchProcessor::<SvmBankForks>::new(
                config.engine_config.processor_config.slot,
                config.engine_config.processor_config.epoch,
                config.engine_config.processor_config.program_ids.clone(),
            )
        });

        let storage = Self::Storage::load(config.engine_config.storage_config.storage_file.clone())
            .await
            .unwrap_or_default();
        initialize_programs(
            &storage,
            &config.engine_config.storage_config.programs_to_load,
        )
        .expect("Failed to load initial programs");
        for (pubkey, account) in config.engine_config.storage_config.accounts_to_load.iter() {
            storage.set_account(pubkey, account.clone()).unwrap();
        }
        for (pubkey, account) in accounts_to_load.iter() {
            // Load additional specified accounts
            storage.set_account(pubkey, account.clone()).unwrap();
        }
        let engine = Engine::builder()
            .processor(processor)
            .db(storage.clone())
            .build();
        engine.initialize_cache();
        engine.initialize_transaction_processor();
        engine.fill_cache(&storage.get_program_accounts());
        let builder = Self::Builder::default();
        let tx_queue = Self::Queue::default();
        let _syncer = Self::SvmSyncer::new(storage.clone(), config.rpc_addr.clone());
        let relay = Self::SvmRelay::default();
        let runner = Runner::new(engine, builder);
        let (relay_to_runner, relay_from_rpc) = mpsc::channel(config.queue_size);
        let (relay_to_tx_queue, relay_from_runner) = mpsc::channel(config.queue_size);
        let rpc = Self::Rpc::new(relay_to_runner.clone(), storage.clone());

        // Set up a periodic task to perform a storage persist every hour.
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(
            config.engine_config.storage_config.persist_interval_s,
        ));

        let storage_file = config.engine_config.storage_config.storage_file.clone();
        // Spawn a task to persist the storage periodically.
        let persist_handle = tokio::spawn(async move {
            loop {
                ticker.tick().await;
                let _ = storage
                    .persist(storage_file.clone())
                    .await
                    .inspect_err(|e| {
                        eprintln!("Failed to persist storage: {e}");
                    });
            }
        });

        // Spawn the syncer task to handle syncing data from the L1 to the SVM Engine.
        // let syncer_handle = tokio::spawn(async move { syncer.run().await });

        // Spawn the queue task.
        let tx_queue_clone = tx_queue.clone();
        let queue_handle = tokio::spawn(async move {
            if let Err(e) = tx_queue_clone.run(relay_from_runner).await {
                eprintln!("Queue error: {e}");
            }
        });

        let queue_addr = config.tx_queue_config.rpc_addr.clone();
        // Spawn the queue reader task to handle reading transactions from the queue.
        let queue_reader_handle = tokio::spawn(async move {
            let server = ServerBuilder::new().build(queue_addr).await.unwrap();
            let handle = server.start(tx_queue.into_rpc());
            println!("Listening for queue requests...");
            handle.stopped().await;
        });

        // Spawn the runner task to handle running transactions and sending them to the queue.
        let runner_handle = tokio::spawn(async move {
            if let Err(e) = runner.run(relay_from_rpc, relay_to_tx_queue).await {
                eprintln!("Runner error: {e}");
            }
        });

        let queue_addr = config.tx_queue_config.rpc_addr.clone();
        let rpc_addr = config.rpc_addr.clone();
        // Spawn the relay task to handle relaying transactions from the TxQueue to the L1.
        let relay_handle =
            tokio::spawn(async move { relay.run(queue_addr.parse().unwrap(), rpc_addr).await });

        let engine_rpc_addr = config.engine_config.rpc_addr.clone();
        // Spawn the RPC server task to listen to requests.
        let rpc_handle = tokio::spawn(async move {
            let server = ServerBuilder::new().build(engine_rpc_addr).await.unwrap();
            let handle = server.start(rpc.into_rpc());
            println!("Listening for RPC requests...");
            handle.stopped().await;
        });

        let _ = tokio::join!(
            persist_handle,
            // syncer_handle,   TODO Jason - syncer not needed yet and not subscribing properly now
            queue_handle,
            queue_reader_handle,
            runner_handle,
            relay_handle,
            rpc_handle,
        );
    }
}
