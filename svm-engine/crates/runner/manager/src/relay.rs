use std::{net::SocketAddr, time::Duration};

use jsonrpsee::ws_client::WsClientBuilder;
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig};
use solana_sdk::{commitment_config::CommitmentLevel, transaction::Transaction};
use solana_transaction_status::TransactionBinaryEncoding;
use svm_runner::decode_and_deserialize;
use svm_runner_relay::Relay;
use svm_runner_tx_queue::TxQueueReaderClient;
use tokio::time::sleep;

#[derive(Debug, Default)]
pub struct SvmRelay {}

#[async_trait::async_trait]
impl Relay for SvmRelay {
    async fn run(&self, tx_queue_addr: SocketAddr, rpc_addr: String) {
        // Implement the relay logic here
        let queue_client = WsClientBuilder::default()
            .build(&format!("ws://{}", tx_queue_addr))
            .await
            .unwrap();

        let solana_client = RpcClient::new(rpc_addr);

        loop {
            // Get the next transaction from the queue
            let Ok(Some(transaction)) = queue_client.get_next_transaction().await else {
                // If no transaction is available, wait for a short period before retrying
                sleep(Duration::from_millis(40)).await;
                continue;
            };

            let (_, transaction_decoded) = decode_and_deserialize::<Transaction>(
                transaction,
                TransactionBinaryEncoding::Base64,
            )
            .expect("Queue couldn't deserialize a stored transaction");
            loop {
                match solana_client
                    .send_transaction_with_config(
                        &transaction_decoded,
                        RpcSendTransactionConfig {
                            skip_preflight: true,
                            preflight_commitment: Some(CommitmentLevel::Confirmed),
                            max_retries: Some(0),
                            encoding: None,
                            min_context_slot: None,
                        },
                    )
                    .await
                {
                    Ok(signature) => {
                        queue_client
                            .mark_transaction_settled(signature.to_string())
                            .await
                            .unwrap();
                        break;
                    }
                    Err(_e) => {
                        // If the transaction fails, wait for a short period before retrying
                        sleep(Duration::from_millis(40)).await;
                    }
                }
            }
        }
    }
}
