use svm_engine::storage::SVMStorage;
use tokio::sync::mpsc;

use super::{SyncerConfigChange, SyncerMessage};

/// Errors that can occur while updating the account.
#[derive(Debug, thiserror::Error)]
pub enum UpdaterError {
    /// Error while updating the account.
    #[error("Error while updating the account: {0}")]
    UpdateAccountError(#[from] svm_engine::SVMEngineError),
}

/// Result type for the updater.
pub type UpdaterResult<T = ()> = Result<T, UpdaterError>;

pub struct Updater<Storage>
where
    Storage: SVMStorage,
{
    storage: Storage,
}

impl<Storage> Updater<Storage>
where
    Storage: SVMStorage + Clone + Send + Sync + 'static,
{
    pub fn new(storage: Storage) -> Self {
        Self { storage }
    }

    pub async fn run(
        &self,
        relay_syncer_message: mpsc::Receiver<SyncerMessage>,
        _send_account_and_owner_changes: mpsc::Sender<SyncerConfigChange>,
    ) -> UpdaterResult {
        let mut relay_syncer_message = relay_syncer_message;

        while let Some(message) = relay_syncer_message.recv().await {
            match message {
                SyncerMessage::Account {
                    account, pubkey, ..
                } => {
                    // Handle account update
                    self.storage.set_account(&pubkey, account)?;
                    // TODO: Logic to add accounts to the set of tracked accounts
                    // send_account_and_owner_changes.send(SyncerConfigChange::AddAccounts {
                    //     accounts: [pubkey].into_iter().collect(),
                    // }).await?;
                }
                SyncerMessage::Slot { .. } => {
                    // TODO: Maybe handle slot updates
                }
            }
        }
        Ok(())
    }
}
