use anyhow::anyhow;
use futures::FutureExt;
use jsonrpsee::core::async_trait;
use solana_sdk::commitment_config::CommitmentConfig;
use sov_rollup_interface::{
    da::{DaSpec, RelevantBlobs, RelevantProofs},
    node::da::{DaService, MaybeRetryable, SubmitBlobReceipt},
};
use tokio::sync::oneshot;
use tracing::info;

use crate::{
    native::service::{client_error_to_maybe_retryable, SolanaService},
    SolanaBlock, SolanaBlockHeader, SolanaDASpec, SolanaServiceConfig, SolanaVerifier,
};

#[async_trait]
impl DaService for SolanaService {
    type Spec = SolanaDASpec;
    type Config = SolanaServiceConfig;
    type Verifier = SolanaVerifier;
    type FilteredBlock = SolanaBlock;
    type Error = anyhow::Error;

    /// Fetch the block at the given height, waiting for one to be mined if necessary.
    /// The returned block may not be final, and can be reverted without a consensus violation.
    /// Call it for the same height are allowed to return different results.
    /// Should always returns the block at that height on the best fork.
    async fn get_block_at(&self, height: u64) -> Result<Self::FilteredBlock, Self::Error> {
        let block = self.rpc_get_block(height).await?;

        let batch = self
            .get_indexer_data(height, "batch", &self.params.blober_batch)
            .boxed();
        let proof = self
            .get_indexer_data(height, "proof", &self.params.blober_proofs)
            .boxed();

        let (batch_data, proof_data) = futures::future::try_join(batch, proof).await?;

        Ok(SolanaBlock::new(height, block, batch_data, proof_data))
    }

    /// Subscribe to finalized headers as they are finalized.
    /// Expect only to receive headers which were finalized after subscription
    /// Optimized version of `get_last_finalized_block_header`.
    async fn get_last_finalized_block_header(
        &self,
    ) -> Result<<Self::Spec as DaSpec>::BlockHeader, Self::Error> {
        info!("get_last_finalized_block_header START");
        let slot = self
            .rpc()
            .get_slot_with_commitment(CommitmentConfig::finalized())
            .await
            .map_err(client_error_to_maybe_retryable(
                "Problem when retrieving last finalized slot from Solana RPC",
            ))?;
        let block = self
            .rpc()
            .get_block_with_config(
                slot,
                solana_client::rpc_config::RpcBlockConfig {
                    encoding: None,
                    transaction_details: Some(solana_transaction_status::TransactionDetails::Full),
                    rewards: Some(false),
                    commitment: Some(CommitmentConfig::finalized()),
                    max_supported_transaction_version: None,
                },
            )
            .await
            .map_err(client_error_to_maybe_retryable(
                "Problem when retrieving last finalized block from Solana RPC",
            ))?
            .into();

        info!("get_last_finalized_block_header DONE");
        Ok(SolanaBlockHeader::new(slot, &block))
    }

    /// Fetch the head block of the most popular fork.
    ///
    /// More like utility method, to provide better user experience
    async fn get_head_block_header(
        &self,
    ) -> Result<<Self::Spec as DaSpec>::BlockHeader, Self::Error> {
        info!("get_head_block_header START");
        let slot = self
            .rpc()
            .get_slot_with_commitment(CommitmentConfig::confirmed())
            .await
            .map_err(client_error_to_maybe_retryable(
                "Problem when retrieving last confirmed slot from Solana RPC",
            ))?;
        let block = self
            .rpc()
            .get_block_with_config(
                slot,
                solana_client::rpc_config::RpcBlockConfig {
                    encoding: None,
                    transaction_details: Some(solana_transaction_status::TransactionDetails::Full),
                    rewards: Some(false),
                    commitment: Some(CommitmentConfig::confirmed()),
                    max_supported_transaction_version: None,
                },
            )
            .await
            .map_err(client_error_to_maybe_retryable(
                "Problem when retrieving last confirmed block from Solana RPC",
            ))?
            .into();

        info!("get_head_block_header DONE");
        Ok(SolanaBlockHeader::new(slot, &block))
    }

    /// Extract the relevant transactions from a block.
    fn extract_relevant_blobs(
        &self,
        block: &Self::FilteredBlock,
    ) -> RelevantBlobs<<Self::Spec as DaSpec>::BlobTransaction> {
        info!("extract_relevant_blobs START");
        let relevant_blobs = block.into();
        info!("extract_relevant_blobs DONE");
        relevant_blobs
    }

    /// Generate a proof that the relevant blob transactions have been extracted correctly from the DA layer
    /// block.
    async fn get_extraction_proof(
        &self,
        block: &Self::FilteredBlock,
        _blobs: &RelevantBlobs<<Self::Spec as DaSpec>::BlobTransaction>,
    ) -> RelevantProofs<
        <Self::Spec as DaSpec>::InclusionMultiProof,
        <Self::Spec as DaSpec>::CompletenessProof,
    > {
        info!("get_extraction_proof START");
        let extraction_proof = block.into();
        info!("get_extraction_proof DONE");
        extraction_proof
    }

    /// Send a transaction directly to the DA layer.
    /// blob is the serialized and signed transaction.
    /// Returns transaction id if it was successfully sent.
    async fn send_transaction(
        &self,
        blob: &[u8],
    ) -> tokio::sync::oneshot::Receiver<
        Result<SubmitBlobReceipt<<Self::Spec as DaSpec>::TransactionId>, Self::Error>,
    > {
        let (tx, rx) = oneshot::channel();

        let res = self
            .send_transaction(blob)
            .await
            .map_err(|e| anyhow::anyhow!(e));

        tx.send(res).unwrap();
        rx
    }

    /// Sends a proof to the DA layer.
    async fn send_proof(
        &self,
        aggregated_proof_data: &[u8],
    ) -> tokio::sync::oneshot::Receiver<
        Result<SubmitBlobReceipt<<Self::Spec as DaSpec>::TransactionId>, Self::Error>,
    > {
        let (tx, rx) = oneshot::channel();

        let res = self
            .send_proof(aggregated_proof_data)
            .await
            .map_err(|e| anyhow::anyhow!(e));

        tx.send(res).unwrap();
        rx
    }

    /// Fetches all proofs at a specified block height.
    async fn get_proofs_at(&self, height: u64) -> Result<Vec<Vec<u8>>, Self::Error> {
        info!("get_proofs_at {height} START");
        let blobs = self
            .blober()
            .get_blobs(height, self.params.blober_proofs)
            .await
            .map_err(|err| {
                MaybeRetryable::Permanent(anyhow!("Error when retrieving proof blobs: {err:?}"))
            })?;
        info!("get_proofs_at {height} DONE");
        Ok(blobs)
    }
}
