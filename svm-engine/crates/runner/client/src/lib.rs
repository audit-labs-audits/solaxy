use std::{ops::Deref, str::FromStr};

use anchor_lang::{InstructionData, ToAccountMetas};
use base64::{Engine, prelude::BASE64_STANDARD};
use jsonrpsee::ws_client::{WsClient, WsClientBuilder};
use proxy::{find_lock_address, find_lock_config_address, find_lock_signer_address};
use solana_account_decoder::UiAccount;
use solana_client::{
    client_error::{ClientError, ClientErrorKind},
    nonblocking::rpc_client::RpcClient,
    rpc_client::SerializableTransaction,
    rpc_config::{RpcAccountInfoConfig, RpcSendTransactionConfig, RpcSignatureStatusConfig},
    rpc_response::Response,
};
use solana_sdk::{
    bs58,
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    system_program,
    transaction::Transaction,
};
use solana_transaction_status::{TransactionStatus, UiTransactionEncoding};
use svm_runner_rpc::RunnerRpcClient;

pub mod nonces;

#[derive(Debug, thiserror::Error)]
pub enum RunnerClientError {
    /// Solana [`RpcClient`] error.
    #[error(transparent)]
    ClientError(#[from] ClientError),
    /// Catch all error
    #[error("Error: {0}")]
    CatchAll(String),
}

pub type RunnerClientResult<T = ()> = Result<T, RunnerClientError>;

pub struct RunnerClient {
    inner: RpcClient,
    runner_client: WsClient,
}

impl Deref for RunnerClient {
    type Target = RpcClient;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl RunnerClient {
    /// Creates a new [`RunnerClient`] instance.
    pub async fn new(cluster_url: String, runner_url: String) -> Self {
        let inner = RpcClient::new(cluster_url);
        let runner_client = WsClientBuilder::default().build(runner_url).await.unwrap();
        Self {
            inner,
            runner_client,
        }
    }

    /// Creates a new [`RunnerClient`] instance with a specified CommitmentConfig
    pub async fn new_with_commitment(
        cluster_url: String,
        runner_url: String,
        commitment_config: CommitmentConfig,
    ) -> Self {
        let inner = RpcClient::new_with_commitment(cluster_url, commitment_config);
        let runner_client = WsClientBuilder::default().build(runner_url).await.unwrap();
        Self {
            inner,
            runner_client,
        }
    }

    /// Sends a transaction to the SVM engine and returns the transaction signature.
    pub async fn runner_send_transaction(
        &self,
        transaction: &impl SerializableTransaction,
    ) -> RunnerClientResult<Signature> {
        self.runner_send_transaction_with_config(
            transaction,
            RpcSendTransactionConfig {
                preflight_commitment: Some(self.commitment().commitment),
                ..RpcSendTransactionConfig::default()
            },
        )
        .await
    }

    /// Sends a transaction to the SVM engine with the specified configuration and returns the transaction signature.
    pub async fn runner_send_transaction_with_config(
        &self,
        transaction: &impl SerializableTransaction,
        config: RpcSendTransactionConfig,
    ) -> RunnerClientResult<Signature> {
        let encoding = config.encoding.unwrap_or(UiTransactionEncoding::Base64);
        let preflight_commitment = CommitmentConfig {
            commitment: config.preflight_commitment.unwrap_or_default(),
        };
        let config = RpcSendTransactionConfig {
            encoding: Some(encoding),
            preflight_commitment: Some(preflight_commitment.commitment),
            ..config
        };
        let serialized_encoded = serialize_and_encode(transaction, encoding).map_err(|kind| {
            RunnerClientError::ClientError(ClientError {
                kind,
                request: None,
            })
        })?;
        self.runner_client
            .send_transaction(serialized_encoded, Some(config))
            .await
            .map(|signature_str| Signature::from_str(&signature_str).unwrap())
            .map_err(|e| {
                RunnerClientError::CatchAll(format!(
                    "Failed to send transaction to SVM engine: {e}"
                ))
            })
    }

    /// Sends a transaction to the SVM engine and returns the transaction signature.
    /// In case of SVM engine failure (not the transaction itself), it falls back to the Solana RPC client.
    pub async fn runner_send_transaction_with_fallback(
        &self,
        transaction: &impl SerializableTransaction,
    ) -> RunnerClientResult<Signature> {
        match self.runner_send_transaction(transaction).await {
            Ok(signature) => Ok(signature),
            Err(_) => self.inner.send_transaction(transaction).await.map_err(|e| {
                RunnerClientError::CatchAll(format!("Failed to send transaction to cluster: {e}"))
            }),
        }
    }

    /// Sends a transaction to the SVM engine with the specified configuration and returns the transaction signature.
    /// In case of SVM engine failure (not the transaction itself), it falls back to the Solana RPC client.
    pub async fn runner_send_transaction_with_config_with_fallback(
        &self,
        transaction: &impl SerializableTransaction,
        config: RpcSendTransactionConfig,
    ) -> RunnerClientResult<Signature> {
        match self
            .runner_send_transaction_with_config(transaction, config)
            .await
        {
            Ok(signature) => Ok(signature),
            Err(_) => self
                .inner
                .send_transaction_with_config(transaction, config)
                .await
                .map_err(|e| {
                    RunnerClientError::CatchAll(format!(
                        "Failed to send transaction to cluster: {e}"
                    ))
                }),
        }
    }

    /// Gets the signature statuses from the SVM engine.
    pub async fn runner_get_signature_statuses(
        &self,
        signatures: &[Signature],
        config: Option<RpcSignatureStatusConfig>,
    ) -> RunnerClientResult<Response<Vec<Option<TransactionStatus>>>> {
        self.runner_client
            .get_signature_statuses(
                signatures
                    .iter()
                    .map(|signature| signature.to_string())
                    .collect(),
                config,
            )
            .await
            .map_err(|e| {
                RunnerClientError::CatchAll(format!(
                    "Failed to get signature statuses from SVM engine: {e}"
                ))
            })
    }

    /// Gets the account info from the SVM engine.
    pub async fn runner_get_account_info(
        &self,
        pubkey: &Pubkey,
        config: Option<RpcAccountInfoConfig>,
    ) -> RunnerClientResult<Response<Option<UiAccount>>> {
        self.runner_client
            .get_account_info(pubkey.to_string(), config)
            .await
            .map_err(|e| {
                RunnerClientError::CatchAll(format!("Failed to get account from SVM engine: {e}"))
            })
    }

    /// Gets the balance from the SVM engine.
    pub async fn runner_get_balance(
        &self,
        pubkey: &Pubkey,
        config: Option<RpcAccountInfoConfig>,
    ) -> RunnerClientResult<Response<u64>> {
        self.runner_client
            .get_balance(pubkey.to_string(), config)
            .await
            .map_err(|e| {
                RunnerClientError::CatchAll(format!("Failed to get balance from SVM engine: {e}"))
            })
    }

    /// Sets up proxy lock for a target program with the given lock duration (in seconds).
    pub async fn lock_program(
        &self,
        proxy_program_id: &Pubkey,
        target_program_id: &Pubkey,
        lock_duration_s: u16,
        payer: &Keypair,
    ) -> RunnerClientResult<Signature> {
        let lock = find_lock_address(*proxy_program_id, *target_program_id);
        let lock_config = find_lock_config_address(*proxy_program_id, *target_program_id);
        let initialize_lock_ix = Instruction {
            accounts: proxy::accounts::Initialize {
                lock,
                lock_config,
                payer: payer.pubkey(),
                system_program: solana_sdk::system_program::id(),
                target: *target_program_id,
            }
            .to_account_metas(None),
            data: proxy::instruction::Initialize {
                lock_duration: lock_duration_s,
            }
            .data(),
            program_id: *proxy_program_id,
        };

        let transaction = Transaction::new_signed_with_payer(
            &[initialize_lock_ix],
            Some(&payer.pubkey()),
            &[payer],
            self.inner.get_latest_blockhash().await?,
        );

        Ok(self.inner.send_transaction(&transaction).await?)
    }

    /// Configures the lock for a target program with the given lock duration (in seconds).
    pub async fn configure_lock(
        &self,
        proxy_program_id: &Pubkey,
        target_program_id: &Pubkey,
        lock_duration_s: u16,
        payer: &Keypair,
    ) -> RunnerClientResult<Signature> {
        let lock_config = find_lock_config_address(*proxy_program_id, *target_program_id);
        let configure_lock_ix = Instruction {
            accounts: proxy::accounts::Configure {
                lock_config,
                payer: payer.pubkey(),
                target: *target_program_id,
            }
            .to_account_metas(None),
            data: proxy::instruction::Configure {
                lock_duration: lock_duration_s,
            }
            .data(),
            program_id: *proxy_program_id,
        };

        let transaction = Transaction::new_signed_with_payer(
            &[configure_lock_ix],
            Some(&payer.pubkey()),
            &[payer],
            self.inner.get_latest_blockhash().await?,
        );

        Ok(self.inner.send_transaction(&transaction).await?)
    }

    /// Creates a proxied instruction for the target program.
    pub async fn create_proxied_instruction(
        &self,
        proxy_program_id: &Pubkey,
        target_program_id: &Pubkey,
        instruction: Instruction,
        payer: &Pubkey,
    ) -> Instruction {
        let pda_signer = find_lock_signer_address(*proxy_program_id, *target_program_id);
        let lock = find_lock_address(*proxy_program_id, *target_program_id);
        let lock_config = find_lock_config_address(*proxy_program_id, *target_program_id);

        let mut account_metas = instruction.accounts.clone();
        account_metas.first_mut().unwrap().is_signer = false;

        let proxy_instruction = proxy::instruction::Proxy {
            instruction_data: instruction.data,
            account_metas: bincode::serialize(&account_metas).unwrap(),
        };
        let proxy_accounts = proxy::accounts::Proxy {
            lock,
            lock_config,
            pda_signer,
            payer: *payer,
            target: *target_program_id,
            system_program: system_program::id(),
        };

        Instruction {
            accounts: [proxy_accounts.to_account_metas(None), account_metas].concat(),
            data: proxy_instruction.data(),
            program_id: *proxy_program_id,
        }
    }
}

fn serialize_and_encode<T>(
    input: &T,
    encoding: UiTransactionEncoding,
) -> Result<String, ClientErrorKind>
where
    T: serde::ser::Serialize,
{
    let serialized = bincode::serialize(input)
        .map_err(|e| ClientErrorKind::Custom(format!("Serialization failed: {e}")))?;
    let encoded = match encoding {
        UiTransactionEncoding::Base58 => bs58::encode(serialized).into_string(),
        UiTransactionEncoding::Base64 => BASE64_STANDARD.encode(serialized),
        _ => {
            return Err(ClientErrorKind::Custom(format!(
                "unsupported encoding: {encoding}. Supported encodings: base58, base64"
            )));
        }
    };
    Ok(encoded)
}
