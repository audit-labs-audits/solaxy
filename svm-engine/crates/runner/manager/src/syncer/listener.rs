use std::{sync::Arc, time::Duration};

use dashmap::DashSet;
use futures::{SinkExt, StreamExt};
use solana_sdk::{account::Account, pubkey::Pubkey};
use tokio::sync::mpsc;
use yellowstone_grpc_client::{GeyserGrpcClient, GeyserGrpcClientError, Interceptor};
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterAccounts, SubscribeRequestFilterSlots,
    SubscribeRequestPing, SubscribeUpdateAccount, SubscribeUpdatePong, SubscribeUpdateSlot,
    subscribe_update::UpdateOneof,
};

use super::{SyncerConfigChange, SyncerMessage};

/// Errors that can occur while listening to the Geyser gRPC client.
#[derive(Debug, thiserror::Error)]
pub enum ListenerError {
    /// Grpc client error.
    #[error("Grpc client error: {0}")]
    GrpcError(#[from] GeyserGrpcClientError),
}

/// Result type for the listener.
pub type ListenerResult<T = ()> = std::result::Result<T, ListenerError>;

pub struct Listener<F>
where
    F: Interceptor,
{
    client: GeyserGrpcClient<F>,
    accounts: Arc<DashSet<Pubkey>>,
    owners: Arc<DashSet<Pubkey>>,
}

impl<F> Listener<F>
where
    F: Interceptor + 'static,
{
    pub fn new(
        client: GeyserGrpcClient<F>,
        accounts: Arc<DashSet<Pubkey>>,
        owners: Arc<DashSet<Pubkey>>,
    ) -> Self {
        Self {
            client,
            accounts,
            owners,
        }
    }

    pub async fn run(
        &mut self,
        relay_account_and_owner_changes: mpsc::Receiver<SyncerConfigChange>,
        send_syncer_message: mpsc::Sender<SyncerMessage>,
    ) -> ListenerResult {
        let mut relay_account_and_owner_changes = relay_account_and_owner_changes;
        let (mut subscribe_sender, mut stream) = self.client.subscribe().await?;

        let accounts = self.accounts.clone();
        let owners = self.owners.clone();

        let changes_listener_handle = tokio::spawn(async move {
            while let Some(change) = relay_account_and_owner_changes.recv().await {
                match change {
                    SyncerConfigChange::AddAccounts {
                        accounts: new_accounts,
                    } => {
                        for account in new_accounts {
                            accounts.insert(account);
                        }
                    }
                    SyncerConfigChange::RemoveAccounts {
                        accounts: removed_accounts,
                    } => {
                        for account in removed_accounts {
                            accounts.remove(&account);
                        }
                    }
                    SyncerConfigChange::AddOwners { owners: new_owners } => {
                        for owner in new_owners {
                            owners.insert(owner);
                        }
                    }
                    SyncerConfigChange::RemoveOwners {
                        owners: removed_owners,
                    } => {
                        for owner in removed_owners {
                            owners.remove(&owner);
                        }
                    }
                }
            }
        });

        let accounts = self.accounts.clone();
        let owners = self.owners.clone();

        let subscribe_handle = tokio::spawn(async move {
            let mut timer = tokio::time::interval(Duration::from_secs(3));
            let mut id = 0;

            loop {
                timer.tick().await;
                id += 1;
                subscribe_sender
                    .send(SubscribeRequest {
                        slots: [(
                            "all".to_owned(),
                            SubscribeRequestFilterSlots {
                                filter_by_commitment: None,
                            },
                        )]
                        .into_iter()
                        .collect(),
                        accounts: [
                            (
                                "specific".to_owned(),
                                SubscribeRequestFilterAccounts {
                                    account: accounts.iter().map(|s| s.to_string()).collect(),
                                    owner: Vec::new(),
                                    filters: Vec::new(),
                                },
                            ),
                            (
                                "pdas".to_owned(),
                                SubscribeRequestFilterAccounts {
                                    account: Vec::new(),
                                    owner: owners.iter().map(|s| s.to_string()).collect(),
                                    filters: Vec::new(),
                                },
                            ),
                        ]
                        .into_iter()
                        .collect(),
                        commitment: Some(CommitmentLevel::Confirmed as i32),
                        ping: Some(SubscribeRequestPing { id }),
                        ..Default::default()
                    })
                    .await
                    .expect("Error sending subscribe request");
            }
        });

        let accounts = self.accounts.clone();
        let owners = self.owners.clone();

        let stream_handle = tokio::spawn(async move {
            while let Some(Ok(message)) = stream.next().await {
                match message.update_oneof.expect("valid message") {
                    UpdateOneof::Slot(SubscribeUpdateSlot { slot, status, .. }) => {
                        send_syncer_message
                            .send(SyncerMessage::Slot {
                                slot,
                                status: match status {
                                    0 => CommitmentLevel::Processed,
                                    1 => CommitmentLevel::Confirmed,
                                    2 => CommitmentLevel::Finalized,
                                    _ => panic!("unknown status"),
                                },
                            })
                            .await
                            .unwrap();
                    }
                    UpdateOneof::Account(SubscribeUpdateAccount { account, slot, .. }) => {
                        if let Some(account) = account {
                            let pubkey = Pubkey::try_from(account.pubkey).unwrap();
                            if accounts.contains(&pubkey) || owners.contains(&pubkey) {
                                send_syncer_message
                                    .send(SyncerMessage::Account {
                                        account: Account {
                                            lamports: account.lamports,
                                            data: account.data,
                                            owner: Pubkey::try_from(account.owner).unwrap(),
                                            executable: account.executable,
                                            rent_epoch: account.rent_epoch,
                                        },
                                        pubkey,
                                        slot,
                                    })
                                    .await
                                    .unwrap();
                            }
                        }
                    }
                    UpdateOneof::Ping(_msg) => {
                        println!("ping received");
                    }
                    UpdateOneof::Pong(SubscribeUpdatePong { id }) => {
                        println!("pong received: id#{id}");
                    }
                    msg => panic!("received unexpected message: {msg:?}"),
                }
            }
        });

        let _ = tokio::join!(changes_listener_handle, subscribe_handle, stream_handle);

        Ok(())
    }
}
