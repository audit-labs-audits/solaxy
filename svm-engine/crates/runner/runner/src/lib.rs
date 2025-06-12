use std::{any::type_name, time::Duration};

use base64::{Engine as Base64Engine, prelude::BASE64_STANDARD};
use bincode::Options;
use solana_sdk::{clock::Slot, packet::PACKET_DATA_SIZE, transaction::Transaction};
use solana_svm::{
    account_loader::CheckedTransactionDetails, transaction_results::TransactionExecutionResult,
};
use solana_transaction_status::{
    TransactionBinaryEncoding, TransactionConfirmationStatus, TransactionStatus,
};
use svm_engine::{
    Engine, SVMEngineResult, storage::SVMStorage, verification::sanitize_and_verify_tx,
};
use svm_runner_block_builder::BlockBuilder;
use tokio::{
    select,
    sync::mpsc::{Receiver, Sender},
};

pub trait StoreTransactionStatus {
    fn store_transaction_status(
        &self,
        signature: &str,
        status: TransactionStatus,
    ) -> SVMEngineResult;

    fn get_transaction_status(&self, signature: &str)
    -> SVMEngineResult<Option<TransactionStatus>>;

    fn purge_expired_transaction_statuses(&self, slot: Slot) -> SVMEngineResult;
}

pub struct Runner<'a, Storage, Builder>
where
    Storage: SVMStorage,
    Builder: BlockBuilder,
{
    engine: Engine<'a, Storage>,
    block_builder: Builder,
}

impl<'a, Storage, Builder> Runner<'a, Storage, Builder>
where
    Storage: SVMStorage + StoreTransactionStatus,
    Builder: BlockBuilder,
{
    pub fn new(engine: Engine<'a, Storage>, block_builder: Builder) -> Self {
        Self {
            engine,
            block_builder,
        }
    }

    pub async fn run(
        &self,
        relay_from_rpc: Receiver<String>,
        relay_to_tx_queue: Sender<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut relay_from_rpc = relay_from_rpc;
        let mut interval = tokio::time::interval(Duration::from_millis(40));
        loop {
            select! {
                _ = interval.tick() => {
                    let block = self.block_builder.build_block();
                    let txs = block.unwrap();
                    let (sanitized, check_results): (Vec<_>, Vec<_>) =
                        txs.iter().map(|transaction| {
                            let (_, transaction) = decode_and_deserialize::<Transaction>(
                                transaction.clone(),
                                TransactionBinaryEncoding::Base64,
                            ).unwrap();
                            let sanitized = sanitize_and_verify_tx(&transaction, &Default::default())
                                .expect("Failed to sanitize and verify transaction");
                            let check_results = Ok(CheckedTransactionDetails {
                                nonce: None,
                                lamports_per_signature: 5000,
                            });
                            (sanitized, check_results)
                        }).unzip();
                    let res = self
                        .engine
                        .load_execute_and_commit_transactions(&sanitized, check_results)
                        .inspect_err(|e| {
                            eprintln!("Failed to execute transaction: {e:?}");
                        });
                    match res {
                        Ok(results) => {
                            for (transaction, result, sanitized_tx) in
                                            txs.into_iter().zip(results).zip(sanitized).map(|((t, r), z)| (t, r, z)) {
                                if result.was_executed()
                                {
                                    relay_to_tx_queue.send(transaction).await.unwrap(); // Don't relay to L1 if tx was not executed

                                    // Build and store a TransactionStatus. Note that non-executed txs don't get stored, but
                                    // txs that executed and failed do get stored.
                                    if let TransactionExecutionResult::Executed { details, .. } = result {
                                        let status = TransactionStatus {
                                            slot: self.engine.slot(),
                                            confirmations: None,
                                            status: Ok(()),
                                            err: details.status.err(),
                                            confirmation_status: Some(TransactionConfirmationStatus::Finalized)
                                        };
                                        self.engine.get_storage().store_transaction_status(
                                            &sanitized_tx.signatures()[0].to_string(), status).expect("Failed to store transaction status");
                                    }
                                }
                            }
                            self.engine.get_storage().purge_expired_transaction_statuses(self.engine.slot()).expect("Failed to purge expired transaction statuses");
                        }
                        Err(e) => {
                            eprintln!("Failed to execute transactions: {e:?}");
                        }
                    }
                }
                Some(transaction) = relay_from_rpc.recv() => {
                    // Process the transaction
                    self.block_builder.accept_transaction(transaction.clone()).expect("Block builder failed to accept transaction");
                }
            }
        }
    }
}

const MAX_BASE58_SIZE: usize = 1683; // Golden, bump if PACKET_DATA_SIZE changes
const MAX_BASE64_SIZE: usize = 1644; // Golden, bump if PACKET_DATA_SIZE changes
/// Helper function to decode and deserialize a base64-encoded string.
/// Simplified version of https://github.com/anza-xyz/agave/blob/fa620250ab962feb9bd8b1977affe14b94302291/svm/examples/json-rpc/server/src/rpc_process.rs#L807
pub fn decode_and_deserialize<T>(
    encoded: String,
    encoding: TransactionBinaryEncoding,
) -> Result<(Vec<u8>, T), Box<dyn std::error::Error>>
where
    T: serde::de::DeserializeOwned,
{
    let decoded = match encoding {
        TransactionBinaryEncoding::Base58 => {
            if encoded.len() > MAX_BASE58_SIZE {
                return Err(format!(
                    "base58 encoded {} too large: {} bytes (max: encoded/raw {}/{})",
                    type_name::<T>(),
                    encoded.len(),
                    MAX_BASE58_SIZE,
                    PACKET_DATA_SIZE,
                )
                .into());
            }
            solana_sdk::bs58::decode(encoded).into_vec()?
        }
        TransactionBinaryEncoding::Base64 => {
            if encoded.len() > MAX_BASE64_SIZE {
                return Err(format!(
                    "base64 encoded {} too large: {} bytes (max: encoded/raw {}/{})",
                    type_name::<T>(),
                    encoded.len(),
                    MAX_BASE64_SIZE,
                    PACKET_DATA_SIZE,
                )
                .into());
            }
            BASE64_STANDARD.decode(encoded)?
        }
    };
    if decoded.len() > PACKET_DATA_SIZE {
        return Err(format!(
            "decoded {} too large: {} bytes (max: {} bytes)",
            type_name::<T>(),
            decoded.len(),
            PACKET_DATA_SIZE
        )
        .into());
    }
    bincode::options()
        .with_limit(PACKET_DATA_SIZE as u64)
        .with_fixint_encoding()
        .allow_trailing_bytes()
        .deserialize_from(&decoded[..])
        .map_err(|err| {
            format!(
                "failed to deserialize {}: {}",
                type_name::<T>(),
                &err.to_string()
            )
            .into()
        })
        .map(|output| (decoded, output))
}
