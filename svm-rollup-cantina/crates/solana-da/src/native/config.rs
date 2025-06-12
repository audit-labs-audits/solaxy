use nitro_da_client::Priority;
use serde::{Deserialize, Serialize};
use sov_rollup_interface::reexports::schemars;
use svm_types::SolanaAddress;

/// Service-related configuration for the Solana DA layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
pub struct SolanaServiceConfig {
    /// The URL of the Indexer server.
    pub indexer_url: String,
    /// The priority to use for Solana transactions. Higher priority transactions are more likely to
    /// be included in a block, but will cost more.
    pub priority: Priority,
    /// The program ID of the Blober program.
    pub program_id: SolanaAddress,
}
