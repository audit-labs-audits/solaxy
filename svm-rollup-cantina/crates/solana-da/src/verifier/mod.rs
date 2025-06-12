//! This module contains the implementation of the [`sov_rollup_interface::da::DaVerifier`] for the Solana DA layer.
//!
//! It is responsible for verifying that a blob was indeed published as claimed, and that no blobs
//! have been censored. The validation is based on having the blockhash, and then verifying a set of
//! proofs to ensure that the blob contents match the blob provided by the indexer.
//!
//! The parameters used for proof verification are:
//! - The Program ID of the `blober` program used for publishing blobs (or proofs).
//! - The blockhash of the block that was published, retrieved from a third-party Solana node using RPC.

use nitro_da_proofs::compound::{
    completeness::CompoundCompletenessProofError,
    inclusion::{CompoundInclusionProofError, ProofBlob},
};
use solana_sdk::pubkey::Pubkey;
use sov_rollup_interface::da::{BlobReaderTrait, BlockHeaderTrait, DaVerifier};
use tracing::info;

use crate::{SolanaBlobTransaction, SolanaBlockHeader, SolanaChainParams, SolanaDASpec};

mod completeness;
mod inclusion;

pub use completeness::SolanaCompletenessProof;
pub use inclusion::SolanaInclusionMultiProof;

/// The verifier for the Solana DA layer, used to verify zk proofs.
#[derive(Debug, Clone)]
pub struct SolanaVerifier {
    params: SolanaChainParams,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum SolanaVerifierError {
    #[error("Expected either an inclusion proof or a completeness proof, got neither")]
    ProofMissing,
    #[error("A completeness proof cannot be valid when transactions are provided")]
    CompletenessProofWithTransactions,
    #[error(transparent)]
    Inclusion(#[from] CompoundInclusionProofError),
    #[error(transparent)]
    Completeness(#[from] CompoundCompletenessProofError),
}

/// A `DaVerifier` implements the logic required to create a zk proof that some data
/// has been processed.
///
/// This trait implements the required functionality to *verify* claims of the form
/// "If X is the most recent block in the DA layer, then Y is the ordered set of transactions that must
/// be processed by the rollup."
impl DaVerifier for SolanaVerifier {
    /// The set of types required by the DA layer.
    type Spec = SolanaDASpec;

    /// The error type returned by the DA layer's verification function
    type Error = SolanaVerifierError;

    /// Create a new da verifier with the given chain parameters
    fn new(params: <Self::Spec as sov_rollup_interface::da::DaSpec>::ChainParams) -> Self {
        Self { params }
    }

    /// Verify a claimed set of transactions against a block header.
    fn verify_relevant_tx_list(
        &self,
        block_header: &<Self::Spec as sov_rollup_interface::da::DaSpec>::BlockHeader,
        relevant_blobs: &sov_rollup_interface::da::RelevantBlobs<
            <Self::Spec as sov_rollup_interface::da::DaSpec>::BlobTransaction,
        >,
        relevant_proofs: sov_rollup_interface::da::RelevantProofs<
            <Self::Spec as sov_rollup_interface::da::DaSpec>::InclusionMultiProof,
            <Self::Spec as sov_rollup_interface::da::DaSpec>::CompletenessProof,
        >,
    ) -> Result<(), Self::Error> {
        info!("verify_relevant_tx_list START {}", block_header.height());
        self.verify_txs(
            &self.params.blober_proofs,
            block_header,
            &relevant_blobs.proof_blobs,
            &relevant_proofs.proof.inclusion_proof,
            &relevant_proofs.proof.completeness_proof,
        )?;

        self.verify_txs(
            &self.params.blober_batch,
            block_header,
            &relevant_blobs.batch_blobs,
            &relevant_proofs.batch.inclusion_proof,
            &relevant_proofs.batch.completeness_proof,
        )?;

        info!("verify_relevant_tx_list DONE {}", block_header.height());
        Ok(())
    }
}

impl SolanaVerifier {
    /// Checks that either:
    /// 1. There is an inclusion proof and the provided blob transactions match the proof.
    /// 2. There is a completeness proof and there are no blobs.
    fn verify_txs(
        &self,
        blober: &Pubkey,
        block_header: &SolanaBlockHeader,
        txs: &[SolanaBlobTransaction],
        inclusion_proof: &SolanaInclusionMultiProof,
        completeness_proof: &SolanaCompletenessProof,
    ) -> Result<(), <Self as DaVerifier>::Error> {
        if let Some(inclusion_proof) = inclusion_proof.0.as_ref() {
            let blobs = txs
                .iter()
                .map(|tx| {
                    let verified_data = tx.verified_data();
                    // If no data has been verified yet, there's no blob to verify - but other parts
                    // of the proof (for example the blob size or the sender identity) can and
                    // should still be verified.
                    let data = if verified_data.is_empty() {
                        None
                    } else {
                        Some(verified_data)
                    };
                    let blob: Pubkey = tx.blob_address().into();
                    ProofBlob { blob, data }
                })
                .collect::<Vec<_>>();
            inclusion_proof.verify(*blober, block_header.hash().0, &blobs)?;
        } else if let Some(completeness_proof) = completeness_proof.0.as_ref() {
            if !txs.is_empty() {
                return Err(SolanaVerifierError::CompletenessProofWithTransactions);
            }
            completeness_proof.verify(*blober, block_header.hash().0)?;
        } else {
            return Err(SolanaVerifierError::ProofMissing);
        }
        Ok(())
    }
}
