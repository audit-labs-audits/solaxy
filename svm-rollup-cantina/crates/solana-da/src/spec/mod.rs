use sov_rollup_interface::da::DaSpec;
use svm_types::SolanaAddress;

use crate::{SolanaCompletenessProof, SolanaInclusionMultiProof};

mod blob_transaction;
mod block_hash;
mod block_header;
mod chain_params;
mod transaction_id;

pub use blob_transaction::SolanaBlobTransaction;
pub use block_hash::SolanaBlockHash;
pub use block_header::SolanaBlockHeader;
pub use chain_params::SolanaChainParams;
pub use transaction_id::SolanaTransactionId;

/// The specification for the types used by the Solana DA layer.
#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    borsh::BorshSerialize,
    borsh::BorshDeserialize,
)]
pub struct SolanaDASpec;

impl DaSpec for SolanaDASpec {
    type SlotHash = SolanaBlockHash;
    type BlockHeader = SolanaBlockHeader;
    type BlobTransaction = SolanaBlobTransaction;
    type TransactionId = SolanaTransactionId;
    type Address = SolanaAddress;
    type InclusionMultiProof = SolanaInclusionMultiProof;
    type CompletenessProof = SolanaCompletenessProof;
    type ChainParams = SolanaChainParams;
}
