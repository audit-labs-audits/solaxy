use solana_sdk::clock::UnixTimestamp;
#[cfg(feature = "native")]
use solana_transaction_status::EncodedConfirmedBlock;
use sov_rollup_interface::da::{BlockHeaderTrait, Time};

use crate::SolanaBlockHash;

/// A block header on the Solana blockchain.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SolanaBlockHeader {
    previous_blockhash: SolanaBlockHash,
    blockhash: SolanaBlockHash,
    block_height: u64,
    block_time: UnixTimestamp,
}

impl SolanaBlockHeader {
    /// Create a new Solana block header from a block, and the latest set of vote accounts.
    #[cfg(feature = "native")]
    pub fn new(slot: solana_sdk::clock::Slot, block: &EncodedConfirmedBlock) -> Self {
        Self {
            previous_blockhash: block
                .previous_blockhash
                .as_str()
                .try_into()
                .expect("parent_blockhash to be a valid hash"),
            blockhash: block
                .blockhash
                .as_str()
                .try_into()
                .expect("blockhash to be a valid hash"),
            block_height: slot,
            block_time: block
                .block_time
                .unwrap_or_else(|| {
                    tracing::warn!("Missing block_time in SolanaBlockHeader::new. \
                    This is not expected to happen except for ancient blocks. Block data: {block:?}");
                    0
                }),
        }
    }
}

/// A block header, typically used in the context of an underlying DA blockchain.
impl BlockHeaderTrait for SolanaBlockHeader {
    /// Each block header must have a unique canonical hash.
    type Hash = SolanaBlockHash;

    /// Each block header must contain the hash of the previous block.
    fn prev_hash(&self) -> Self::Hash {
        self.previous_blockhash
    }

    /// Hash the type to get the digest.
    fn hash(&self) -> Self::Hash {
        self.blockhash
    }

    /// The current header height
    fn height(&self) -> u64 {
        self.block_height
    }

    /// The timestamp of the block
    fn time(&self) -> Time {
        Time::from_secs(self.block_time)
    }
}
