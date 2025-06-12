/// Errors that can occur within the block builder process.
#[derive(Debug, Clone, thiserror::Error)]
pub enum BlockBuilderError {
    /// Catch all error
    #[error("Catch all error: {0}")]
    CatchAll(String),
}

/// Block builder result type.
pub type BlockBuilderResult<T = ()> = Result<T, BlockBuilderError>;

/// This trait specifies the required interface of a block builder to be used within the SVM engine
/// runner system.
pub trait BlockBuilder {
    /// Accepts a transaction to the mempool.
    fn accept_transaction(&self, transaction: String) -> BlockBuilderResult;

    /// Triggers building a block of transactions.
    fn build_block(&self) -> BlockBuilderResult<Vec<String>>;
}
