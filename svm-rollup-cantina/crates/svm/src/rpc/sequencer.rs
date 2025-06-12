use std::{
    collections::HashMap,
    marker::PhantomData,
    str::FromStr,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};

use jsonrpsee::{
    core::{async_trait, RpcResult},
    proc_macros::rpc,
    types::{ErrorCode, ErrorObjectOwned},
};
use solana_rpc_client_api::config::{RpcRequestAirdropConfig, RpcSendTransactionConfig};
use solana_sdk::{
    hash::Hash,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_instruction,
    transaction::{Transaction, VersionedTransaction},
};
use solana_transaction_status::UiTransactionEncoding;
use sov_address::FromVmAddress;
use sov_modules_api::Spec;
use sov_sequencer::Sequencer;
use svm_types::SolanaAddress;

use crate::{authentication::SolanaAuthenticator, rpc::decode_and_deserialize, SVM};

/// Errors which can occur in the SVM sequencer.
#[derive(Debug, thiserror::Error)]
pub enum SVMSequencerError {
    /// Failed to serialize transactions: {0}
    #[error("Failed to serialize transactions: {0}")]
    SerializationError(#[from] std::io::Error),
    /// Da failure: {0}
    #[error("Da failure: {0}")]
    DaFailure(String),
    /// Invalid params: {0}
    #[error("Invalid params: {0}")]
    InvalidParams(String),
    /// Faucet error: {0}
    #[error("Faucet error: {0}")]
    FaucetError(String),
    /// RPC error: {0}
    #[error("RPC error: {0}")]
    RpcError(#[from] ErrorObjectOwned),
    /// Sequencer error: {0}
    #[error("Sequencer error: {0}")]
    SovereignSequencerError(String),
}

impl From<SVMSequencerError> for ErrorObjectOwned {
    fn from(err: SVMSequencerError) -> Self {
        let code = match err {
            SVMSequencerError::InvalidParams(_) => ErrorCode::InvalidParams,
            _ => ErrorCode::InternalError,
        }
        .code();
        Self::owned(code, format!("{err:?}"), None::<()>)
    }
}

/// Result type for the Sequencer.
pub type SVMSequencerResult<T = ()> = Result<T, SVMSequencerError>;

// defaults for the faucet
const AIRDROP_LAMPORTS_MAX_DEFAULT: u64 = 250_000;
const AIRDROP_RATE_LIMIT_SECS_DEFAULT: u64 = 6 * 60 * 60;

#[derive(Debug)]
pub enum FaucetConfig {
    Inactive,
    Active {
        airdrop_lamports_max: u64,
        airdrop_rate_limit_secs: u64,
        faucet_keypair: Box<Keypair>,
    },
}

impl Default for FaucetConfig {
    fn default() -> Self {
        Self::Active {
            airdrop_lamports_max: AIRDROP_LAMPORTS_MAX_DEFAULT,
            airdrop_rate_limit_secs: AIRDROP_RATE_LIMIT_SECS_DEFAULT,
            faucet_keypair: Box::new(Keypair::new()),
        }
    }
}

impl FaucetConfig {
    pub fn new(keypair: Keypair, lamports: Option<u64>, timeout: Option<u64>) -> Self {
        let airdrop_lamports_max = if let Some(lamports) = lamports {
            lamports
        } else {
            AIRDROP_LAMPORTS_MAX_DEFAULT
        };

        let airdrop_rate_limit_secs = if let Some(timeout) = timeout {
            timeout
        } else {
            AIRDROP_RATE_LIMIT_SECS_DEFAULT
        };

        Self::Active {
            airdrop_lamports_max,
            airdrop_rate_limit_secs,
            faucet_keypair: Box::new(keypair),
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active { .. })
    }
}

#[derive(Debug, Clone)]
pub struct SvmSequencerRpc<Seq, RT>
where
    Seq: Sequencer,
    <Seq::Spec as Spec>::Address: FromVmAddress<SolanaAddress>,
    RT: SolanaAuthenticator<<Seq as Sequencer>::Spec> + Default + Send + Sync + 'static,
{
    pub sequencer: Arc<Seq>,
    pub faucet_config: Arc<FaucetConfig>,
    pub airdrop_tracker: Arc<Mutex<HashMap<Pubkey, SystemTime>>>,
    _phantom: PhantomData<(Seq, RT)>,
}

impl<Seq, RT> SvmSequencerRpc<Seq, RT>
where
    Seq: Sequencer,
    <Seq::Spec as Spec>::Address: FromVmAddress<SolanaAddress>,
    RT: SolanaAuthenticator<<Seq as Sequencer>::Spec> + Default + Send + Sync + 'static,
{
    pub fn new(sequencer: Arc<Seq>, faucet_config: Arc<FaucetConfig>) -> Self {
        Self {
            sequencer,
            faucet_config,
            airdrop_tracker: Arc::new(Mutex::new(HashMap::new())),
            _phantom: PhantomData,
        }
    }

    async fn send_tx(&self, tx: Transaction) -> SVMSequencerResult<String> {
        let signature = tx.signatures[0].to_string();
        let raw_message = borsh::to_vec(&tx)?;
        let fully_baked = RT::encode_with_solana_auth(raw_message);
        let _result = self.sequencer.accept_tx(fully_baked).await.map_err(|e| {
            SVMSequencerError::SovereignSequencerError(format!(
                "Failed to accept transaction: {e:?}"
            ))
        })?;

        Ok(signature)
    }

    async fn get_latest_blockhash(&self) -> SVMSequencerResult<Hash> {
        let mut accessor = self.sequencer.api_state().default_api_state_accessor();
        let svm = SVM::<Seq::Spec>::default();
        let hash = svm.get_latest_blockhash(None, &mut accessor).map_err(|e| {
            SVMSequencerError::SovereignSequencerError(format!(
                "Failed to get recent blockhash: {e:?}"
            ))
        })?;

        Ok(Hash::from_str(&hash.value.blockhash).unwrap())
    }

    fn account_exists(&self, pubkey: &Pubkey) -> bool {
        let mut accessor = self.sequencer.api_state().default_api_state_accessor();
        let svm = SVM::<Seq::Spec>::default();
        svm.get_account_data(pubkey, &mut accessor).is_some()
    }

    fn get_minimum_balance_for_rent_exemption(&self, data_len: usize) -> SVMSequencerResult<u64> {
        let mut accessor = self.sequencer.api_state().default_api_state_accessor();
        let svm = SVM::<Seq::Spec>::default();
        Ok(svm.get_minimum_balance_for_rent_exemption(data_len, None, &mut accessor)?)
    }

    pub async fn request_airdrop(
        &self,
        pubkey: SolanaAddress,
        lamports: u64,
        config: Option<RpcRequestAirdropConfig>,
    ) -> RpcResult<String> {
        let config = config.unwrap_or_default();
        let blockhash = if let Some(blockhash) = config.recent_blockhash {
            Hash::from_str(&blockhash).map_err(|e| {
                SVMSequencerError::InvalidParams(format!(
                    "Failed to deserialize blockhash {blockhash}: {e}"
                ))
            })?
        } else {
            self.get_latest_blockhash().await?
        };

        let receiver: Pubkey = pubkey.into();

        let faucet_config = self.faucet_config.clone();
        let FaucetConfig::Active {
            airdrop_lamports_max: airdrop_lamports,
            airdrop_rate_limit_secs: airdrop_request_timeout_secs,
            faucet_keypair,
        } = faucet_config.as_ref()
        else {
            return Err(SVMSequencerError::FaucetError("Faucet isn't active".to_string()).into());
        };

        if lamports > *airdrop_lamports {
            return Err(SVMSequencerError::FaucetError(format!(
                "Requested lamports need to be less than {airdrop_lamports}"
            ))
            .into());
        }

        if let Some(last_airdrop) = self.airdrop_tracker.lock().unwrap().get(&receiver) {
            let required_timeout = Duration::from_secs(*airdrop_request_timeout_secs);
            let elapsed_time = SystemTime::now()
                .duration_since(*last_airdrop)
                .expect("Failed to compare time instances");

            if required_timeout > elapsed_time {
                let remaining_timeout = required_timeout - elapsed_time;
                return Err(SVMSequencerError::FaucetError(format!(
                    "Need to wait {remaining_timeout:?} before next airdrop"
                ))
                .into());
            }
        };

        let faucet_pubkey = faucet_keypair.pubkey();
        let mut lamports = lamports;

        // Check if the receiver account exists - if not we need to add rent exempt lamports to the
        // transfer amount
        if !self.account_exists(&receiver) {
            lamports += self.get_minimum_balance_for_rent_exemption(0)?;
        }

        let instruction = system_instruction::transfer(&faucet_pubkey, &receiver, lamports);
        let tx = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&faucet_pubkey),
            &[faucet_keypair],
            blockhash,
        );

        self.airdrop_tracker
            .lock()
            .unwrap()
            .insert(receiver, SystemTime::now());

        self.send_tx(tx).await.map_err(ErrorObjectOwned::from)
    }

    pub async fn send_transaction(
        &self,
        tx_bytes: String,
        config: Option<RpcSendTransactionConfig>,
    ) -> RpcResult<String> {
        let config = config.unwrap_or_default();
        let tx_encoding = config.encoding.unwrap_or(UiTransactionEncoding::Base58);
        let binary_encoding = tx_encoding.into_binary_encoding().ok_or_else(|| {
            SVMSequencerError::InvalidParams(format!(
                "unsupported encoding: {tx_encoding}. Supported encodings: base58, base64"
            ))
        })?;
        let (_, solana_tx) =
            decode_and_deserialize::<VersionedTransaction>(tx_bytes, binary_encoding).map_err(
                |e| {
                    SVMSequencerError::InvalidParams(format!(
                        "Failed decoding and deserializing transaction: {e:?}"
                    ))
                },
            )?;

        // Verify transaction signatures without hashing the message
        if !solana_tx.verify_with_results().iter().all(|res| *res) {
            return Err(SVMSequencerError::InvalidParams(
                "Failed to verify transaction signatures".to_string(),
            )
            .into());
        }

        let solana_tx = solana_tx.into_legacy_transaction().ok_or_else(|| {
            SVMSequencerError::InvalidParams(
                "Failed to convert transaction to legacy transaction".to_string(),
            )
        })?;

        self.send_tx(solana_tx)
            .await
            .map_err(ErrorObjectOwned::from)
    }
}

#[rpc(server)]
pub trait SvmSequencer {
    #[method(name = "sendTransaction")]
    async fn send_transaction(
        &self,
        tx_bytes: String,
        config: Option<RpcSendTransactionConfig>,
    ) -> RpcResult<String>;

    #[method(name = "requestAirdrop")]
    async fn request_airdrop(
        &self,
        receiver: SolanaAddress,
        lamports: u64,
        config: Option<RpcRequestAirdropConfig>,
    ) -> RpcResult<String>;
}

#[async_trait]
impl<Seq, RT> SvmSequencerServer for SvmSequencerRpc<Seq, RT>
where
    Seq: Sequencer,
    <Seq::Spec as Spec>::Address: FromVmAddress<SolanaAddress>,
    RT: SolanaAuthenticator<Seq::Spec> + Default + Send + Sync + 'static,
{
    async fn send_transaction(
        &self,
        tx_bytes: String,
        config: Option<RpcSendTransactionConfig>,
    ) -> RpcResult<String> {
        self.send_transaction(tx_bytes, config).await
    }

    async fn request_airdrop(
        &self,
        receiver: SolanaAddress,
        lamports: u64,
        config: Option<RpcRequestAirdropConfig>,
    ) -> RpcResult<String> {
        self.request_airdrop(receiver, lamports, config).await
    }
}
