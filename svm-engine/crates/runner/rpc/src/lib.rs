use jsonrpsee::{
    proc_macros::rpc,
    types::{ErrorCode, ErrorObject},
};
use solana_account_decoder::UiAccount;
use solana_rpc_client_api::{
    config::{RpcAccountInfoConfig, RpcSendTransactionConfig, RpcSignatureStatusConfig},
    response::Response,
};
use solana_transaction_status::TransactionStatus;

/// Errors that can occur when processing RPC requests.
#[derive(Debug, Clone, thiserror::Error)]
pub enum RunnerRpcError {
    /// Invalid params provided
    #[error("Invalid params provided: {0}")]
    InvalidParams(String),
}

impl From<RunnerRpcError> for ErrorObject<'_> {
    fn from(value: RunnerRpcError) -> Self {
        let (code, message) = match value {
            RunnerRpcError::InvalidParams(message) => (ErrorCode::InvalidParams, message),
        };
        Self::owned(code.code(), message, None::<()>)
    }
}

/// Result type for RPC methods.
pub type RpcResult<T = ()> = Result<T, RunnerRpcError>;

/// The RPC server trait for the SVM engine runner.
#[rpc(client, server)]
pub trait RunnerRpc {
    /// Sends a transaction to the SVM engine and returns the transaction signature.
    #[method(name = "sendTransaction")]
    async fn send_transaction(
        &self,
        transaction: String,
        config: Option<RpcSendTransactionConfig>,
    ) -> RpcResult<String>;

    /// Returns the transaction status for the given signature.
    #[method(name = "getSignatureStatuses")]
    async fn get_signature_statuses(
        &self,
        signature_strs: Vec<String>,
        config: Option<RpcSignatureStatusConfig>,
    ) -> RpcResult<Response<Vec<Option<TransactionStatus>>>>;

    /// Returns the account for the given public key.
    #[method(name = "getAccountInfo")]
    async fn get_account_info(
        &self,
        pubkey: String,
        config: Option<RpcAccountInfoConfig>,
    ) -> RpcResult<Response<Option<UiAccount>>>;

    /// Returns the balance for the given public key.
    #[method(name = "getBalance")]
    async fn get_balance(
        &self,
        pubkey: String,
        config: Option<RpcAccountInfoConfig>,
    ) -> RpcResult<Response<u64>>;
}
