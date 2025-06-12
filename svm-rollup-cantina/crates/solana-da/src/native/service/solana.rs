use std::sync::Arc;

use anyhow::anyhow;
use bon::Builder;
use futures::FutureExt;
use nitro_da_client::{BloberClient, BloberClientError, FeeStrategy};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::config::RpcBlockConfig;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::{EncodedConfirmedBlock, UiTransactionEncoding};
use sov_rollup_interface::{
    common::HexHash,
    node::da::{MaybeRetryable, SubmitBlobReceipt},
};

use crate::{
    SolanaBlobTransaction, SolanaChainParams, SolanaData, SolanaServiceConfig, SolanaTransactionId,
};

#[derive(Debug, thiserror::Error)]
pub enum SolanaServiceError {
    /// Error using blober client: {0}
    #[error(transparent)]
    BloberClient(#[from] BloberClientError),
    /// Incorrect number of {namespace} blobs returned for slot {slot}. Expected {expected}, got {got}
    #[error("Incorrect number of {namespace} blobs returned for slot {slot}. Expected {expected}, got {got}",)]
    BlobCount {
        namespace: String,
        slot: u64,
        expected: usize,
        got: usize,
    },
    /// Error when retrieving block from Solana RPC: {0:?}
    #[error("Error when retrieving block from Solana RPC: {0:?}")]
    RpcBlockRetrieval(#[from] MaybeRetryable<anyhow::Error>),
    /// Problem when polling the header stream: {0}
    #[error("Problem when polling the header stream: {0}")]
    HeaderPolling(anyhow::Error),
}

pub type SolanaServiceResult<T = ()> = Result<T, SolanaServiceError>;

/// A service for interacting with the Solana DA layer.
///
/// This service is responsible for fetching data from the DA layer, transforming it into a format that can be
/// efficiently verified in circuit, and also sending transactions to the DA layer. This is done using several
/// different clients, depending on the functionality required.
#[derive(Builder, Clone)]
pub struct SolanaService {
    // The reason these are all Options is to make testing easier - not all methods require each client.
    solana_rpc_client: Option<Arc<RpcClient>>,
    blober_client: Option<BloberClient>,

    pub(crate) params: SolanaChainParams,
    pub(crate) config: SolanaServiceConfig,
}

impl SolanaService {
    /// Create a new `SolanaService` instance.
    #[allow(dead_code)]
    pub fn new(
        solana_rpc_client: Option<Arc<RpcClient>>,
        blober_client: Option<BloberClient>,
        params: SolanaChainParams,
        config: SolanaServiceConfig,
    ) -> Self {
        Self {
            solana_rpc_client,
            blober_client,
            params,
            config,
        }
    }

    /// Get a reference to the Solana RPC client.
    pub(crate) fn rpc(&self) -> &Arc<RpcClient> {
        self.solana_rpc_client
            .as_ref()
            .expect("Expected Solana RPC Client to be present")
    }

    /// Get a reference to the Blober on-chain contract client.
    pub(crate) fn blober(&self) -> &BloberClient {
        self.blober_client
            .as_ref()
            .expect("Expected Blober Client to be present")
    }

    async fn send_data(
        &self,
        data: &[u8],
        blober_id: Pubkey,
    ) -> SolanaServiceResult<SubmitBlobReceipt<SolanaTransactionId>> {
        // This will not be the blob digest in the final proof, it's just used to identify the blob.
        let blob_digest = solana_sdk::hash::hash(data).to_bytes();
        let transaction_outcomes = self
            .blober()
            .upload_blob(
                data,
                FeeStrategy::BasedOnRecentFees(self.config.priority),
                blober_id,
                None,
            )
            .await?;
        let da_transaction_id = SolanaTransactionId::new(
            transaction_outcomes
                .into_iter()
                .map(|o| o.signature)
                .collect(),
        );
        Ok(SubmitBlobReceipt {
            blob_hash: HexHash::new(blob_digest),
            da_transaction_id,
        })
    }

    pub(crate) async fn send_transaction(
        &self,
        blob: &[u8],
    ) -> SolanaServiceResult<SubmitBlobReceipt<SolanaTransactionId>> {
        self.send_data(blob, self.params.blober_batch).await
    }

    pub(crate) async fn send_proof(
        &self,
        aggregated_proof_data: &[u8],
    ) -> SolanaServiceResult<SubmitBlobReceipt<SolanaTransactionId>> {
        self.send_data(aggregated_proof_data, self.params.blober_proofs)
            .await
    }

    /// Fetch the block at the given height, waiting for one to be mined if necessary.
    /// The returned block may not be final, and can be reverted without a consensus violation.
    /// Call it for the same height are allowed to return different results.
    /// Should always returns the block at that height on the best fork.
    pub(crate) async fn rpc_get_block(
        &self,
        height: u64,
    ) -> Result<EncodedConfirmedBlock, MaybeRetryable<anyhow::Error>> {
        loop {
            let res = self
                .rpc()
                .get_block_with_config(
                    height,
                    RpcBlockConfig {
                        commitment: Some(self.rpc().commitment()),
                        encoding: Some(UiTransactionEncoding::Base58),
                        ..Default::default()
                    },
                )
                .await
                .map(EncodedConfirmedBlock::from)
                .map_err(client_error_to_maybe_retryable(
                    "Problem when retrieving block from Solana RPC",
                ));

            if matches!(res, Ok(_) | Err(MaybeRetryable::Permanent(_))) {
                return res;
            }
        }
    }

    pub(crate) async fn get_indexer_data(
        &self,
        height: u64,
        namespace: &str,
        blober: &Pubkey,
    ) -> SolanaServiceResult<SolanaData> {
        let messages = self.blober().get_blob_messages(height, *blober).await?;
        let blobs = self.blober().get_blobs(height, *blober).boxed();
        let proof = self.blober().get_slot_proof(height, *blober).boxed();

        let (blobs, proof) = futures::future::try_join(blobs, proof).await?;

        if blobs.len() != messages.len() {
            return Err(SolanaServiceError::BlobCount {
                namespace: namespace.to_string(),
                slot: height,
                expected: messages.len(),
                got: blobs.len(),
            });
        }

        let transactions = blobs
            .into_iter()
            .zip(messages.into_iter())
            .map(|(blob, (blob_address, message))| {
                SolanaBlobTransaction::new(message, blob_address.into(), blob)
            })
            .collect::<Vec<_>>();

        Ok(SolanaData::new(transactions, proof))
    }
}

/// Check if a client error is a serde error with the data just being `null`.
/// This is often how the Solana RPC API indicates missing data, but it's unfortunately
/// not caught before serde deserialization or converted to an Option.
pub(crate) fn client_error_is_serde_null_data(
    e: &solana_rpc_client_api::client_error::Error,
) -> bool {
    if let solana_client::client_error::ClientErrorKind::SerdeJson(e2) = e.kind() {
        e2.classify() == serde_json::error::Category::Data && e2.column() == 0 && e2.line() == 0
    } else {
        false
    }
}

/// This function maps out the possible errors returned by the Solana RPC client to transient or
/// permanent errors, and  wraps the error in an [`anyhow::Error`] to make it easier to use in the
/// adapter implementation.
pub(crate) fn client_error_to_maybe_retryable(
    description: &str,
) -> impl FnOnce(solana_rpc_client_api::client_error::Error) -> MaybeRetryable<anyhow::Error> + '_ {
    use std::io::ErrorKind;

    use solana_client::rpc_custom_error::*;
    use solana_rpc_client_api::client_error::ErrorKind::*;
    use MaybeRetryable::{Permanent, Transient};

    move |e| {
        let anyhow = anyhow!("{description}: {e:?}");
        if client_error_is_serde_null_data(&e) {
            return Transient(anyhow);
        }

        match e.kind {
            Io(e2)
                if matches!(
                    e2.kind(),
                    ErrorKind::ConnectionRefused
                        | ErrorKind::ConnectionReset
                        | ErrorKind::ConnectionAborted
                        | ErrorKind::NotConnected
                        | ErrorKind::TimedOut
                        | ErrorKind::WriteZero
                        | ErrorKind::Interrupted
                        | ErrorKind::UnexpectedEof
                ) =>
            {
                Transient(anyhow)
            }
            solana_rpc_client_api::client_error::ErrorKind::Reqwest(e2)
                if e2.is_timeout()
                    || e2.status().is_some_and(|s| !s.is_client_error())
                    || e2.is_connect() =>
            {
                Transient(anyhow)
            }
            RpcError(solana_client::rpc_request::RpcError::RpcResponseError { code, .. })
                if [
                    JSON_RPC_SERVER_ERROR_BLOCK_NOT_AVAILABLE,
                    JSON_RPC_SERVER_ERROR_NODE_UNHEALTHY,
                    JSON_RPC_SERVER_ERROR_BLOCK_STATUS_NOT_AVAILABLE_YET,
                ]
                .contains(&code) =>
            {
                Transient(anyhow)
            }
            _ => Permanent(anyhow),
        }
    }
}
