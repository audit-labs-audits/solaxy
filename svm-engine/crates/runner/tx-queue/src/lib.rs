use jsonrpsee::{
    proc_macros::rpc,
    types::{ErrorCode, ErrorObject},
};
use tokio::sync::mpsc;

/// Errors that can occur when processing RPC requests.
#[derive(Debug, Clone, thiserror::Error)]
pub enum TxQueueError {
    /// Internal error occurred
    #[error("Internal server error: {0}")]
    InternalError(String),
}

impl From<TxQueueError> for ErrorObject<'_> {
    fn from(value: TxQueueError) -> Self {
        let (code, message) = match value {
            TxQueueError::InternalError(message) => (ErrorCode::InvalidParams, message),
        };
        Self::owned(code.code(), message, None::<()>)
    }
}

/// Result type for RPC methods.
pub type TxQueueResult<T = ()> = Result<T, TxQueueError>;

#[rpc(server, client)]
pub trait TxQueueReader {
    /// Returns the next transaction in the queue.
    #[method(name = "getNextTransaction")]
    async fn get_next_transaction(&self) -> Result<Option<String>, TxQueueError>;

    /// Marks a transaction as settled on the L1 chain.
    #[method(name = "markTransactionSettled")]
    async fn mark_transaction_settled(&self, signature: String) -> Result<(), TxQueueError>;
}

/// A base trait for the transaction queue writer which needs to be implemented for adding
/// transactions to the queue.
/// This base trait exists for implementations of the queue writer which are meant to be processes
/// ran by a single binary, compared to the other more separated micro services - like is the case
/// in the below [`TxQueueWriterRpc`] trait.
#[async_trait::async_trait]
pub trait TxQueueWriter {
    /// Appends a transaction to the queue.
    fn append_transaction(&self, transaction: String) -> TxQueueResult;

    /// Runs the transaction queue writer.
    async fn run(&self, relay_from_runner: mpsc::Receiver<String>) -> TxQueueResult {
        let mut relay_from_runner = relay_from_runner;
        while let Some(transaction) = relay_from_runner.recv().await {
            self.append_transaction(transaction)?;
        }
        Ok(())
    }
}

/// A trait for implementing RPC servers which can be ran as a separate micro service to support
/// scaling the writing of transactions separately from the rest of the system.
#[rpc(server)]
pub trait TxQueueWriterRpc: TxQueueWriter {
    /// Adds a transaction to the queue
    #[method(name = "submitTransaction")]
    async fn submit_transaction(&self, transaction: String) -> TxQueueResult;
}
