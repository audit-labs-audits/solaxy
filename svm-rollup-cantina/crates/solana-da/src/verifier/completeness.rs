use nitro_da_indexer_api::CompoundProof;
use nitro_da_proofs::compound::completeness::CompoundCompletenessProof;

/// A proof that there are no blobs in a specific Solana block.
///
/// This proof consists of two parts:
/// 1. An [accounts delta hash proof][`ExclusionProof`] that proves that
///    the accounts_delta_hash does *not* include the [`blober`] account.
/// 2. A [bank hash proof][`BankHashProof`] that proves that the root hash of the accounts_delta_hash
///    is the same as the root in the bank hash.
///
/// The proof can then be verified by supplying the blockhash of the block in which the [`blober`]
/// was invoked.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SolanaCompletenessProof(pub Option<CompoundCompletenessProof>);

impl From<&CompoundProof> for SolanaCompletenessProof {
    fn from(value: &CompoundProof) -> Self {
        match value {
            CompoundProof::Inclusion(_) => SolanaCompletenessProof(None),
            CompoundProof::Completeness(proof) => SolanaCompletenessProof(Some(proof.clone())),
        }
    }
}
