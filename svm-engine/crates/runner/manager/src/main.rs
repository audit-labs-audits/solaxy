use std::collections::{HashMap, HashSet};

use svm_runner_manager::{
    EngineConfig, ManagerConfig, ProcessorConfig, SVMManager, StorageConfig, TxQueueConfig,
    builder::Builder, queue::Queue, relay::SvmRelay, rpc::Rpc, storage::Storage, syncer::SvmSyncer,
};

struct DefaultManager;

#[async_trait::async_trait]
impl SVMManager for DefaultManager {
    type Queue = Queue;
    type Storage = Storage;
    type Rpc = Rpc<Self::Storage>;
    type Builder = Builder;
    type SvmSyncer = SvmSyncer<Self::Storage>;
    type SvmRelay = SvmRelay;

    async fn create_config() -> ManagerConfig {
        ManagerConfig {
            rpc_addr: "rpc_url".to_owned(),
            queue_size: 1_000,
            engine_config: EngineConfig {
                rpc_addr: "0.0.0.0:8989".to_owned(),
                processor_config: ProcessorConfig {
                    slot: 1,
                    epoch: 1,
                    program_ids: HashSet::new(),
                },
                storage_config: StorageConfig {
                    storage_file: "./db".to_owned(),
                    persist_interval_s: 3600,
                    programs_to_load: Vec::new(),
                    accounts_to_load: HashMap::new(),
                },
            },
            tx_queue_config: TxQueueConfig {
                rpc_addr: "0.0.0.0:9696".to_owned(),
            },
        }
    }
}

#[tokio::main]
async fn main() {
    DefaultManager::coordinate_tasks(HashMap::new()).await;
}
