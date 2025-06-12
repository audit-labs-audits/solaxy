use std::str::FromStr;

use jsonrpsee::{
    core::client::ClientT,
    rpc_params,
    ws_client::{WsClient, WsClientBuilder},
};
use solana_account_decoder::UiAccount;
use solana_client::rpc_response::RpcResponseContext;
use solana_program::{hash::Hash, pubkey::Pubkey};
use solana_rpc_client_api::{
    config::RpcRequestAirdropConfig,
    response::{Response, RpcBlockhash},
};
use solana_sdk::{account::Account, signature::Signature, transaction::Transaction};
use solana_transaction_status::UiTransactionEncoding;
use svm::test_utils::serialize_and_encode;

pub(crate) struct TestClient {
    rpc: WsClient,
}

impl TestClient {
    pub(crate) async fn new(rpc_addr: std::net::SocketAddr) -> Self {
        let rpc = WsClientBuilder::default()
            .build(&format!("ws://127.0.0.1:{}/rpc", rpc_addr.port()))
            .await
            .unwrap();

        Self { rpc }
    }

    pub(crate) async fn svm_query_state(&self) -> String {
        self.rpc
            .request("queryModuleState", rpc_params![])
            .await
            .unwrap()
    }

    pub(crate) async fn svm_get_slot(&self) -> u64 {
        self.rpc.request("getSlot", rpc_params![]).await.unwrap()
    }

    pub(crate) async fn svm_get_account_info(&self, pubkey: Pubkey) -> Account {
        let res: Response<Option<UiAccount>> = self
            .rpc
            .request("getAccountInfo", rpc_params![pubkey.to_string()])
            .await
            .unwrap();
        res.value.unwrap().decode().unwrap()
    }

    pub(crate) async fn svm_get_account_balance(&self, pubkey: Pubkey) -> u64 {
        let res: Response<u64> = self
            .rpc
            .request("getBalance", rpc_params![pubkey.to_string()])
            .await
            .unwrap_or_else(|_| Response {
                context: RpcResponseContext {
                    api_version: None,
                    slot: 0,
                },
                value: 0,
            });
        res.value
    }

    pub(crate) async fn svm_get_latest_blockhash(&self) -> Hash {
        let res: Response<RpcBlockhash> = self
            .rpc
            .request("getLatestBlockhash", rpc_params![])
            .await
            .unwrap();
        Hash::from_str(&res.value.blockhash).unwrap()
    }

    pub(crate) async fn svm_send_transaction(&self, tx: &Transaction) -> Signature {
        let raw_tx = serialize_and_encode(tx, UiTransactionEncoding::Base58).unwrap();
        let signature_str: String = self
            .rpc
            .request("sendTransaction", rpc_params![raw_tx])
            .await
            .unwrap();
        Signature::from_str(&signature_str).unwrap()
    }

    pub(crate) async fn svm_request_airdrop(
        &self,
        pubkey: &Pubkey,
        lamports: u64,
        config: Option<RpcRequestAirdropConfig>,
    ) -> Result<Signature, jsonrpsee::core::client::Error> {
        let signature_str: String = self
            .rpc
            .request(
                "requestAirdrop",
                rpc_params![pubkey.to_string(), lamports, config],
            )
            .await?;
        Ok(Signature::from_str(&signature_str).unwrap())
    }

    pub(crate) async fn svm_get_minimum_balance_for_rent_exemption(&self, data_len: usize) -> u64 {
        self.rpc
            .request("getMinimumBalanceForRentExemption", rpc_params![data_len])
            .await
            .unwrap()
    }

    pub(crate) async fn svm_get_transaction_count(&self) -> u64 {
        let res: u64 = self
            .rpc
            .request("getTransactionCount", rpc_params![])
            .await
            .unwrap();
        res
    }
}
