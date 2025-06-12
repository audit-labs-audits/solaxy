use nitro_da_indexer_api::CompoundProof;
use nitro_da_proofs::compound::inclusion::CompoundInclusionProof;

/// A proof that a specific Solana block contains blobs, and that there are no other blobs in the block.
///
/// This proof consists of four parts:
/// 1. A list of [blob proofs][`BlobProof`] that prove that the blobs uploaded to the [`blober`] program
///    hash to the given blob digest.
/// 2. A [blober account state proof][`BloberAccountStateProof`] that proves that the [`blober`] was
///    invoked exactly as many times as there are blobs.
/// 3. An [accounts delta hash proof][`InclusionProof`] that proves that
///    the accounts_delta_hash *does* include the [`blober`] account.
/// 4. A [bank hash proof][`BankHashProof`] that proves that the root hash of the accounts_delta_hash
///    is the same as the root in the bank hash.
///
/// The proof can then be verified by supplying the blockhash of the block in which the [`blober`] was
/// invoked, as well as the blobs of data which were published.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SolanaInclusionMultiProof(pub Option<CompoundInclusionProof>);

impl From<&CompoundProof> for SolanaInclusionMultiProof {
    fn from(value: &CompoundProof) -> Self {
        match value {
            CompoundProof::Inclusion(proof) => SolanaInclusionMultiProof(Some(proof.clone())),
            CompoundProof::Completeness(_) => SolanaInclusionMultiProof(None),
        }
    }
}
