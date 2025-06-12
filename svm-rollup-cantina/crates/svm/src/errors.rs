#[derive(Debug, thiserror::Error)]
pub enum SVMRollupError {
    #[error("SVM state error: {0}")]
    State(#[from] SVMStateError),
}

pub type SVMRollupResult<T = ()> = Result<T, SVMRollupError>;

#[derive(Debug, thiserror::Error)]
pub enum SVMStateError {
    #[error("Failed to read from SVM state: {0}")]
    /// Failed to read from SVM state: {0}
    Read(String),

    #[error("Failed to write to SVM state: {0}")]
    /// Failed to write to SVM state: {0}
    Write(String),

    #[error("Unknown SVM state error: {0}")]
    /// Unknown SVM state error: {0}
    CatchAll(#[from] eyre::Error),
}
