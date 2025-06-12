use std::str::FromStr;

use solana_account_decoder::{UiAccount, UiAccountEncoding};
use solana_rpc_client_api::{
    config::{RpcAccountInfoConfig, RpcSendTransactionConfig, RpcSignatureStatusConfig},
    response::{Response, RpcResponseContext},
};
use solana_sdk::{pubkey::Pubkey, transaction::Transaction};
use solana_transaction_status::{TransactionStatus, UiTransactionEncoding};
use svm_engine::storage::SVMStorage;
use svm_runner::{StoreTransactionStatus, decode_and_deserialize};
use svm_runner_rpc::{RpcResult, RunnerRpcError, RunnerRpcServer};
use tokio::sync::mpsc;

pub struct Rpc<Storage>
where
    Storage: SVMStorage,
{
    relay_to_runner: mpsc::Sender<String>,
    storage: Storage,
}

pub trait RpcInstantiator<Storage>
where
    Storage: SVMStorage,
{
    fn new(relay_to_runner: mpsc::Sender<String>, storage: Storage) -> Self;
}

impl<Storage> RpcInstantiator<Storage> for Rpc<Storage>
where
    Storage: SVMStorage,
{
    fn new(relay_to_runner: mpsc::Sender<String>, storage: Storage) -> Self {
        Self {
            relay_to_runner,
            storage,
        }
    }
}

#[async_trait::async_trait]
impl<Storage> RunnerRpcServer for Rpc<Storage>
where
    Storage: SVMStorage + StoreTransactionStatus + Send + Sync + 'static,
{
    async fn send_transaction(
        &self,
        transaction: String,
        config: Option<RpcSendTransactionConfig>,
    ) -> RpcResult<String> {
        // TODO: Handle verification logic here
        let (_, tx) = decode_and_deserialize::<Transaction>(
            transaction.clone(),
            config
                .unwrap_or_default()
                .encoding
                .unwrap_or(UiTransactionEncoding::Base64)
                .into_binary_encoding()
                .unwrap(),
        )
        .unwrap();
        self.relay_to_runner.send(transaction).await.map_err(|e| {
            RunnerRpcError::InvalidParams(format!("Failed to send transaction: {e}"))
        })?;
        Ok(tx.signatures[0].to_string())
    }

    async fn get_signature_statuses(
        &self,
        signature_strs: Vec<String>,
        _config: Option<RpcSignatureStatusConfig>,
    ) -> RpcResult<Response<Vec<Option<TransactionStatus>>>> {
        let mut results = Vec::with_capacity(signature_strs.len());

        for sig in signature_strs {
            results.push(
                self.storage
                    .get_transaction_status(&sig)
                    .expect("Error getting transaction status"),
            );
        }

        Ok(Response {
            context: RpcResponseContext {
                slot: 0,
                api_version: None,
            },
            value: results,
        })
    }

    async fn get_account_info(
        &self,
        pubkey: String,
        config: Option<RpcAccountInfoConfig>,
    ) -> RpcResult<Response<Option<UiAccount>>> {
        let pubkey = Pubkey::from_str(&pubkey)
            .map_err(|e| RunnerRpcError::InvalidParams(format!("Failed to parse pubkey: {e}")))?;
        let (encoding, data_slice) = match config {
            Some(RpcAccountInfoConfig {
                encoding,
                data_slice,
                ..
            }) => (
                encoding.unwrap_or(UiAccountEncoding::Base64Zstd),
                data_slice,
            ),
            None => (UiAccountEncoding::Base64Zstd, None),
        };
        let account = self.storage.get_account(&pubkey).map_err(|e| {
            RunnerRpcError::InvalidParams(format!("Failed to get account info: {e}"))
        })?;
        Ok(Response {
            context: RpcResponseContext {
                slot: 0,
                api_version: None,
            },
            value: account.map(|acc| UiAccount::encode(&pubkey, &acc, encoding, None, data_slice)),
        })
    }

    async fn get_balance(
        &self,
        pubkey: String,
        _config: Option<RpcAccountInfoConfig>,
    ) -> RpcResult<Response<u64>> {
        let pubkey = Pubkey::from_str(&pubkey)
            .map_err(|e| RunnerRpcError::InvalidParams(format!("Failed to parse pubkey: {e}")))?;
        let account = self.storage.get_account(&pubkey).map_err(|e| {
            RunnerRpcError::InvalidParams(format!("Failed to get account info: {e}"))
        })?;
        Ok(Response {
            context: RpcResponseContext {
                slot: 0,
                api_version: None,
            },
            value: account.map(|acc| acc.lamports).unwrap_or(0),
        })
    }
}
