use std::{
    collections::{HashSet, VecDeque},
    sync::{Arc, RwLock},
};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    account::{Account, AccountSharedData},
    clock::Slot,
    pubkey::Pubkey,
};
use solana_svm::transaction_processing_callback::TransactionProcessingCallback;
use solana_transaction_status::TransactionStatus;
use svm_engine::{
    SVMEngineResult,
    storage::{SVMStorage, SVMStorageError},
};
use svm_runner::StoreTransactionStatus;

const SLOT_HISTORY_LENGTH: Slot = 1200;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Storage {
    pub(crate) accounts: Arc<DashMap<Pubkey, Account>>,
    pub(crate) owner_index: Arc<DashMap<Pubkey, HashSet<Pubkey>>>,

    pub(crate) tx_history_data: Arc<DashMap<String, TransactionStatus>>,
    pub(crate) tx_history_queue: Arc<RwLock<VecDeque<(String, Slot)>>>,
}

#[async_trait::async_trait]
pub trait Persistable: Sized + Default {
    async fn persist(&self, location: String) -> SVMEngineResult;

    async fn load(location: String) -> Option<Self>;
}

impl Storage {
    /// Creates a new instance of [`Storage`] with empty collections.
    pub fn new() -> Self {
        Self {
            accounts: Arc::new(DashMap::new()),
            owner_index: Arc::new(DashMap::new()),
            tx_history_data: Arc::new(DashMap::new()),
            tx_history_queue: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    /// Creates a new instance of [`Storage`] with the given accounts and owner index.
    pub async fn new_from_path(location: String) -> Self {
        Self::load(location).await.unwrap_or_default()
    }
}

#[async_trait::async_trait]
impl Persistable for Storage {
    /// Persists the current state of the storage to the specified location.
    async fn persist(&self, location: String) -> SVMEngineResult {
        let serialized =
            bincode::serialize(&self).map_err(|e| SVMStorageError::Write(e.to_string()))?;
        tokio::fs::write(&location, serialized).await.map_err(|e| {
            SVMStorageError::Write(format!("Failed to write to {}: {}", location, e))
        })?;
        Ok(())
    }

    /// Loads the storage from the specified location. If the file does not exist or cannot be
    /// read, decoded or deserialized, it returns [`None`].
    async fn load(location: String) -> Option<Self> {
        let data = tokio::fs::read(&location).await.ok()?;
        bincode::deserialize(&data).ok()
    }
}

impl StoreTransactionStatus for Storage {
    fn store_transaction_status(
        &self,
        signature: &str,
        status: TransactionStatus,
    ) -> SVMEngineResult {
        let mut tx_queue = self
            .tx_history_queue
            .write()
            .map_err(|e| SVMStorageError::Read(format!("History queue is poisoned {e}")))?;
        tx_queue.push_back((signature.to_owned(), status.slot));

        self.tx_history_data.insert(signature.to_owned(), status);

        Ok(())
    }

    fn get_transaction_status(
        &self,
        signature: &str,
    ) -> SVMEngineResult<Option<TransactionStatus>> {
        Ok(self.tx_history_data.get(signature).map(|stat| stat.clone()))
    }

    fn purge_expired_transaction_statuses(&self, slot: Slot) -> SVMEngineResult {
        let earliest_slot = slot.saturating_sub(SLOT_HISTORY_LENGTH);

        let mut tx_queue = self
            .tx_history_queue
            .write()
            .map_err(|e| SVMStorageError::Read(format!("History queue is poisoned {e}")))?;
        while (!tx_queue.is_empty())
            && (tx_queue.front().expect("Error in tx history queue").1 < earliest_slot)
        {
            self.tx_history_data.remove(&tx_queue.front().unwrap().0);
            tx_queue.pop_front();
        }

        Ok(())
    }
}

impl SVMStorage for Storage {
    fn get_account(&self, key: &Pubkey) -> SVMEngineResult<Option<Account>> {
        let account = self.accounts.view(key, |_, account| account.clone());
        Ok(account)
    }

    fn set_account(&self, key: &Pubkey, account: Account) -> SVMEngineResult {
        self.accounts.insert(*key, account);
        Ok(())
    }

    fn remove_account(&self, key: &Pubkey) -> SVMEngineResult {
        self.accounts.remove(key);
        Ok(())
    }

    fn get_owner_index(&self, owner: &Pubkey) -> SVMEngineResult<Option<Vec<Pubkey>>> {
        let owner_index = self
            .owner_index
            .view(owner, |_, owner_index| owner_index.clone());
        Ok(owner_index.map(|i| i.iter().copied().collect()))
    }

    fn set_owner_index(&self, owner: &Pubkey, key: &Pubkey) -> SVMEngineResult {
        self.owner_index.entry(*owner).or_default().insert(*key);
        Ok(())
    }
}

impl TransactionProcessingCallback for Storage {
    fn account_matches_owners(&self, account: &Pubkey, owners: &[Pubkey]) -> Option<usize> {
        self.is_account_owned_by(account, owners)
    }

    fn get_account_shared_data(&self, pubkey: &Pubkey) -> Option<AccountSharedData> {
        self.account_shared_data(pubkey)
    }

    fn add_builtin_account(&self, name: &str, program_id: &Pubkey) {
        self.add_builtin_program(name, program_id)
            .unwrap_or_else(|e| {
                panic!("Failed to add builtin program {name} with ID {program_id}: {e}")
            });
    }
}

#[cfg(test)]
mod tests {
    use solana_sdk::transaction::TransactionError;
    use solana_transaction_status::TransactionConfirmationStatus;

    use super::*;

    #[test]
    fn test_transaction_statuses() {
        let storage = Storage::new();

        let missing_sig: String = "MISSING SIGNATURE".to_string();

        let sig1: String = "SIGNATURE 1".to_string();
        let ts1 = TransactionStatus {
            slot: 1,
            confirmations: None,
            status: Ok(()),
            err: None,
            confirmation_status: Some(TransactionConfirmationStatus::Finalized),
        };
        let sig2: String = "SIGNATURE 2".to_string();
        let ts2 = TransactionStatus {
            slot: 2,
            confirmations: None,
            status: Ok(()),
            err: Some(TransactionError::ProgramAccountNotFound),
            confirmation_status: Some(TransactionConfirmationStatus::Finalized),
        };

        // Add one transaction status and confirm it.
        storage
            .store_transaction_status(&sig1, ts1)
            .expect("Store failed");
        let maybe_status = storage
            .get_transaction_status(&missing_sig)
            .expect("get failed");
        assert!(maybe_status.is_none());
        let maybe_status = storage.get_transaction_status(&sig1).expect("get failed");
        assert!(maybe_status.is_some());
        assert!(maybe_status.clone().unwrap().err.is_none());
        assert_eq!(maybe_status.unwrap().slot, 1);

        // Add a second transaction status and confirm both.
        storage
            .store_transaction_status(&sig2, ts2)
            .expect("Store failed");
        let maybe_status = storage
            .get_transaction_status(&missing_sig)
            .expect("get failed");
        assert!(maybe_status.is_none());
        let maybe_status = storage.get_transaction_status(&sig1).expect("get failed");
        assert!(maybe_status.is_some());
        assert!(maybe_status.clone().unwrap().err.is_none());
        assert_eq!(maybe_status.unwrap().slot, 1);
        let maybe_status = storage.get_transaction_status(&sig2).expect("get failed");
        assert!(maybe_status.is_some());
        assert_eq!(
            maybe_status.clone().unwrap().err,
            Some(TransactionError::ProgramAccountNotFound)
        );
        assert_eq!(maybe_status.unwrap().slot, 2);

        // Force purge of older entries. This is a no-op.
        storage
            .purge_expired_transaction_statuses(1 + SLOT_HISTORY_LENGTH)
            .expect("purge failed");
        let maybe_status = storage
            .get_transaction_status(&missing_sig)
            .expect("get failed");
        assert!(maybe_status.is_none());
        let maybe_status = storage.get_transaction_status(&sig1).expect("get failed");
        assert!(maybe_status.is_some());
        assert!(maybe_status.clone().unwrap().err.is_none());
        assert_eq!(maybe_status.unwrap().slot, 1);
        let maybe_status = storage.get_transaction_status(&sig2).expect("get failed");
        assert!(maybe_status.is_some());
        assert_eq!(
            maybe_status.clone().unwrap().err,
            Some(TransactionError::ProgramAccountNotFound)
        );
        assert_eq!(maybe_status.unwrap().slot, 2);

        // Force purge of older entries. This purges the first entry.
        storage
            .purge_expired_transaction_statuses(2 + SLOT_HISTORY_LENGTH)
            .expect("purge failed");
        let maybe_status = storage
            .get_transaction_status(&missing_sig)
            .expect("get failed");
        assert!(maybe_status.is_none());
        let maybe_status = storage.get_transaction_status(&sig1).expect("get failed");
        assert!(maybe_status.is_none());
        let maybe_status = storage.get_transaction_status(&sig2).expect("get failed");
        assert!(maybe_status.is_some());
        assert_eq!(
            maybe_status.clone().unwrap().err,
            Some(TransactionError::ProgramAccountNotFound)
        );
        assert_eq!(maybe_status.unwrap().slot, 2);
    }
}
