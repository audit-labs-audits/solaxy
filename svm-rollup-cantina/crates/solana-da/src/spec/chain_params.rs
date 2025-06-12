#[cfg(feature = "native")]
use blober::find_blober_address;
use solana_sdk::pubkey::Pubkey;

/// Proof-related parameters for the Solana DA layer.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SolanaChainParams {
    /// The [`Pubkey`] of the [`blober`] PDA used for publishing blobs.
    #[serde(deserialize_with = "svm_engine_keys::pubkey::single::deserialize")]
    pub blober_batch: Pubkey,
    /// The [`Pubkey`] of the [`blober`] PDA used for publishing proofs.
    #[serde(deserialize_with = "svm_engine_keys::pubkey::single::deserialize")]
    pub blober_proofs: Pubkey,
}

#[cfg(feature = "native")]
impl SolanaChainParams {
    /// Creates a new [`SolanaChainParams`] instance.
    #[must_use]
    pub fn new(program_id: Pubkey, payer: Pubkey) -> Self {
        Self {
            blober_batch: find_blober_address(program_id, payer, "batch"),
            blober_proofs: find_blober_address(program_id, payer, "proof"),
        }
    }
}
