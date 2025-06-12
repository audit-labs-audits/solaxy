use std::{
    error::Error as StdError,
    str::FromStr,
    sync::Arc,
    task::{Context, Poll},
};

use futures::future::BoxFuture;
use http::{Request, Response};
use jsonrpsee::{
    core::RpcResult,
    proc_macros::rpc,
    server::HttpBody as Body,
    types::error::{ErrorCode, ErrorObjectOwned},
};
use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    reserved_account_keys::ReservedAccountKeys, signature::Signature, transaction::Transaction,
};
use solana_svm::{
    transaction_processing_callback::TransactionProcessingCallback,
    transaction_processor::TransactionBatchProcessor,
};
use svm_engine::{
    Engine,
    storage::{AccountsDB, SVMStorage, SvmBankForks},
    verification::{get_transaction_check_results, sanitize_and_verify_tx},
};
use tower::{Layer, Service};
use tracing::debug;

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfoResponse {
    /// The amount of lamports owned by this account.
    pub lamports: u64,
    /// The program that owns this account. If executable, the program that loads this account.
    pub owner: Pubkey,
    /// Data held in this account.
    pub data: Vec<u8>,
    /// This account's data contains a loaded program (and is now read-only).
    pub executable: bool,
    /// The epoch at which this account will next owe rent.
    pub rent_epoch: u64,
    /// The data size of the account.
    pub size: u64,
}

#[rpc(server)]
pub trait Rpc {
    #[method(name = "getAccountInfo")]
    async fn get_account_info(&self, pubkey: &str) -> RpcResult<AccountInfoResponse>;

    #[method(name = "sendTransaction")]
    async fn send_transaction(&self, tx: Transaction) -> RpcResult<Signature>;
}

pub struct Svm<'a, Storage: SVMStorage + Send + Sync> {
    engine: Arc<Engine<'a, Storage>>,
}

impl<'a, S: SVMStorage + Send + Sync> Svm<'a, S> {
    pub fn new(db: S, processor: &'a TransactionBatchProcessor<SvmBankForks>) -> Self {
        let programs = db.get_program_accounts();
        let engine = Arc::new(Engine::builder().db(db).processor(processor).build());
        engine.initialize_cache();
        engine.initialize_transaction_processor();
        engine.fill_cache(&programs);

        Self { engine }
    }
}

#[async_trait::async_trait]
impl<S: SVMStorage + Send + Sync + 'static> RpcServer for Svm<'static, S> {
    async fn get_account_info(&self, pubkey: &str) -> RpcResult<AccountInfoResponse> {
        let pubkey = Pubkey::from_str(pubkey)
            .map_err(|e| create_client_error(&format!("Invalid base58 pubkey: {e}")))?;

        let account = self
            .engine
            .get_account(&pubkey)
            .map_err(|e| create_client_error(&format!("Account not found with pubkey: {e}")))?
            .ok_or_else(|| {
                create_client_error(&format!("Account not found with pubkey: {pubkey}"))
            })?;

        Ok(AccountInfoResponse {
            lamports: account.lamports,
            owner: account.owner,
            data: account.data.clone(),
            executable: account.executable,
            rent_epoch: account.rent_epoch,
            size: account.data.len() as u64,
        })
    }

    async fn send_transaction(&self, transaction: Transaction) -> RpcResult<Signature>
    // processor expects db to be TransactionProcessingCallback but commit function expects it to be AccountsDB
    where
        AccountsDB: TransactionProcessingCallback,
    {
        // preflight checks: verify signatures and simulate transaction
        // sanitize and verify transaction string
        debug!("sanitizing tx");
        let san_tx = sanitize_and_verify_tx(&transaction, &ReservedAccountKeys::default().active)
            .map_err(|e| {
            create_client_error(&format!("Failed to sanitize and verify transaction: {e}"))
        })?;

        let check_results =
            get_transaction_check_results(&[san_tx.clone()], solana_sdk::clock::MAX_PROCESSING_AGE);

        debug!("executing and commiting tx");
        self.engine
            .load_execute_and_commit_transactions(&[san_tx.clone()], vec![check_results[0].clone()])
            .map_err(|e| {
                create_client_error(&format!(
                    "failed to load, execute, and commit transactions: {e}",
                ))
            })?;

        // according to Solana RPC documentation, return the first signature of the transaction sent
        Ok(*san_tx.signature())
    }
}

fn create_client_error(message: &str) -> ErrorObjectOwned {
    ErrorObjectOwned::owned(ErrorCode::InvalidRequest.code(), message, Some(()))
}

#[derive(Clone)]
pub struct ApiKeyLayer {
    api_key: String,
}

impl ApiKeyLayer {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

impl<S> Layer<S> for ApiKeyLayer {
    type Service = ApiKeyMiddleware<S>;

    fn layer(&self, service: S) -> Self::Service {
        ApiKeyMiddleware {
            inner: service,
            api_key: self.api_key.clone(),
        }
    }
}

#[derive(Clone)]
pub struct ApiKeyMiddleware<S> {
    inner: S,
    api_key: String,
}

impl<S> Service<Request<Body>> for ApiKeyMiddleware<S>
where
    S: Service<Request<Body>, Response = Response<Body>, Error = Box<dyn StdError + Send + Sync>>
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = Box<dyn StdError + Send + Sync>;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let api_key = self.api_key.clone();
        let is_authorized = request
            .headers()
            .get(http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .map(|auth| auth == format!("Bearer {}", api_key))
            .unwrap_or(false);

        if !is_authorized {
            let response = Response::builder()
                .status(401)
                .body(Body::from("Unauthorized"))
                .unwrap();
            return Box::pin(async move { Ok(response) });
        }

        let future = self.inner.call(request);
        Box::pin(future)
    }
}
