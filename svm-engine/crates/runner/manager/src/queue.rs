use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
};

use svm_runner_tx_queue::{TxQueueError, TxQueueReaderServer, TxQueueResult, TxQueueWriter};

#[derive(Clone, Default)]
pub struct Queue {
    transactions: Arc<RwLock<VecDeque<String>>>,
}

impl TxQueueWriter for Queue {
    fn append_transaction(&self, transaction: String) -> TxQueueResult {
        let Ok(mut transactions) = self.transactions.write() else {
            return Err(TxQueueError::InternalError(
                "Failed to acquire write lock".to_owned(),
            ));
        };

        transactions.push_back(transaction);
        Ok(())
    }
}

#[async_trait::async_trait]
impl TxQueueReaderServer for Queue {
    async fn get_next_transaction(&self) -> TxQueueResult<Option<String>> {
        let Ok(transactions) = self.transactions.read() else {
            return Err(TxQueueError::InternalError(
                "Failed to acquire read lock".to_owned(),
            ));
        };

        if transactions.is_empty() {
            return Ok(None);
        }

        Ok(transactions.front().cloned())
    }

    async fn mark_transaction_settled(&self, _signature: String) -> TxQueueResult {
        let Ok(mut transactions) = self.transactions.write() else {
            return Err(TxQueueError::InternalError(
                "Failed to acquire write lock".to_owned(),
            ));
        };

        transactions.pop_front();
        Ok(())
    }
}
