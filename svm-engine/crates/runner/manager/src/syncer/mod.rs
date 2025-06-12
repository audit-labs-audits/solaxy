use std::{collections::HashSet, sync::Arc};

use dashmap::DashSet;
use listener::Listener;
use solana_sdk::{account::Account, clock::Slot, pubkey::Pubkey};
use svm_engine::storage::SVMStorage;
use svm_runner_syncer::Syncer;
use tokio::sync::mpsc;
use updater::Updater;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::geyser::CommitmentLevel;

mod listener;
mod updater;

pub enum SyncerConfigChange {
    AddAccounts { accounts: HashSet<Pubkey> },
    RemoveAccounts { accounts: HashSet<Pubkey> },
    AddOwners { owners: HashSet<Pubkey> },
    RemoveOwners { owners: HashSet<Pubkey> },
}

pub enum SyncerMessage {
    Account {
        account: Account,
        pubkey: Pubkey,
        slot: Slot,
    },
    Slot {
        slot: Slot,
        status: CommitmentLevel,
    },
}

#[derive(Debug, Default)]
pub struct SvmSyncer<Storage>
where
    Storage: SVMStorage + Clone + Send + Sync + 'static,
{
    storage: Storage,
    url: String,
}

#[async_trait::async_trait]
impl<Storage> Syncer<Storage> for SvmSyncer<Storage>
where
    Storage: SVMStorage + Clone + Send + Sync + 'static,
{
    fn new(storage: Storage, url: String) -> Self {
        Self { storage, url }
    }

    async fn run(&self) {
        let subs = GeyserGrpcClient::build_from_shared(self.url.clone())
            .unwrap()
            .connect()
            .await
            .unwrap();

        let mut listener = Listener::new(subs, Arc::new(DashSet::new()), Arc::new(DashSet::new()));
        let updater = Updater::new(self.storage.clone());

        let (send_account_and_owner_changes, relay_account_and_owner_changes) = mpsc::channel(100);
        let (send_syncer_message, relay_syncer_message) = mpsc::channel(100);

        let listener_handle = tokio::spawn(async move {
            if let Err(e) = listener
                .run(relay_account_and_owner_changes, send_syncer_message)
                .await
            {
                eprintln!("Listener error: {e:?}");
            }
        });

        let updater_handle = tokio::spawn(async move {
            if let Err(e) = updater
                .run(relay_syncer_message, send_account_and_owner_changes)
                .await
            {
                eprintln!("Updater error: {e:?}");
            }
        });

        let _ = tokio::join!(listener_handle, updater_handle);
    }
}
