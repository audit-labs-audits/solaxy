use std::{fmt::Display, hash::Hash};

/// The identifier of multiple transactions on the Solana blockchain, or equivalently a single blob.
///
/// This is implemented as a list of transaction signatures since a blob can't be uploaded in a
/// single Solana transaction.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct SolanaTransactionId {
    /// Signatures
    pub signatures: Vec<solana_sdk::signature::Signature>,
}

impl Display for SolanaTransactionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.signatures
                .iter()
                .map(solana_sdk::signature::Signature::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl SolanaTransactionId {
    /// Create a new transaction ID from a list of signatures.
    #[must_use]
    pub fn new(signatures: Vec<solana_sdk::signature::Signature>) -> Self {
        Self { signatures }
    }
}
