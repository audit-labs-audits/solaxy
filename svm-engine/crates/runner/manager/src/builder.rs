use std::sync::{Arc, RwLock};

use svm_runner_block_builder::{BlockBuilder, BlockBuilderError, BlockBuilderResult};

#[derive(Debug, Clone, Default)]
pub struct Builder {
    transactions: Arc<RwLock<Vec<String>>>,
}

impl BlockBuilder for Builder {
    fn accept_transaction(&self, transaction: String) -> BlockBuilderResult {
        let Ok(mut transactions) = self.transactions.write() else {
            return Err(BlockBuilderError::CatchAll(
                "Failed to acquire write lock".to_owned(),
            ));
        };

        transactions.push(transaction);
        Ok(())
    }

    fn build_block(&self) -> BlockBuilderResult<Vec<String>> {
        let Ok(mut transactions) = self.transactions.write() else {
            return Err(BlockBuilderError::CatchAll(
                "Failed to acquire write lock".to_owned(),
            ));
        };

        Ok(transactions.drain(..).collect())
    }
}
