use std::{net::SocketAddr, str::FromStr};

use anyhow::anyhow;
use jsonrpsee::{
    core::client::ClientT,
    rpc_params,
    ws_client::{WsClient, WsClientBuilder},
};
use solana_account_decoder::UiAccount;
use solana_program::{hash::Hash, message::Message, pubkey::Pubkey};
use solana_rpc_client_api::response::{Response, RpcBlockhash};
use solana_sdk::{account::Account, signature::Signature, transaction::Transaction};
use solana_transaction_status::UiTransactionEncoding;
use sov_cli::NodeClient;
use sov_modules_api::{
    capabilities::config_chain_id, execution_mode::Native, transaction::PriorityFeeBips, PrivateKey,
};
use sov_rollup_apis::SimulateExecutionContainer;
use sov_sp1_adapter::{SP1CryptoSpec, SP1};
use sov_test_utils::{
    ledger_db::sov_api_spec::types::{PartialTransaction, SimulateBody},
    EncodeCall, MockDaSpec, MockZkvm, TestUser, TEST_DEFAULT_MAX_FEE,
};
use svm::{test_utils::serialize_and_encode, SVM};
use svm_types::{NativeStorage, SolanaSpec};

pub(crate) struct TestClient {
    rpc: WsClient,
    node_client: NodeClient,
}

type SvmRollupSpec = SolanaSpec<MockDaSpec, SP1, MockZkvm, SP1CryptoSpec, Native, NativeStorage>;
pub type RT = stf::runtime::Runtime<SvmRollupSpec>;

impl TestClient {
    pub(crate) async fn new(addr: String) -> Self {
        let socket: SocketAddr = addr.parse().unwrap();
        let rpc = WsClientBuilder::default()
            .build(&format!("ws://127.0.0.1:{}/rpc", socket.port()))
            .await
            .unwrap();

        let node_client = NodeClient::new_unchecked(&format!("http://127.0.0.1:{}", socket.port()));

        Self { rpc, node_client }
    }

    pub(crate) async fn svm_query_state(&self) -> anyhow::Result<String> {
        self.rpc
            .request("queryModuleState", rpc_params![])
            .await
            .map_err(|e| anyhow!("Failed to query SVM state: {e}"))
    }

    #[allow(dead_code)]
    pub(crate) async fn svm_get_slot(&self) -> anyhow::Result<u64> {
        self.rpc
            .request("getSlot", rpc_params![])
            .await
            .map_err(|e| anyhow!("Failed to get slot: {e}"))
    }

    pub(crate) async fn svm_get_account_info(&self, pubkey: Pubkey) -> anyhow::Result<Account> {
        let res: Response<Option<UiAccount>> = self
            .rpc
            .request("getAccountInfo", rpc_params![pubkey.to_string()])
            .await
            .map_err(|e| anyhow!("Failed to get account info: {e}"))?;
        res.value
            .ok_or_else(|| anyhow!("Failed to get account info"))?
            .decode()
            .ok_or_else(|| anyhow!("Failed to decode account info"))
    }

    pub(crate) async fn svm_get_account_balance(&self, pubkey: Pubkey) -> anyhow::Result<u64> {
        let res: Response<u64> = self
            .rpc
            .request("getBalance", rpc_params![pubkey.to_string()])
            .await
            .map_err(|e| anyhow!("Failed to get account balance: {e}"))?;
        Ok(res.value)
    }

    pub(crate) async fn svm_get_latest_blockhash(&self) -> anyhow::Result<Hash> {
        let res: Response<RpcBlockhash> = self
            .rpc
            .request("getLatestBlockhash", rpc_params![])
            .await
            .map_err(|e| anyhow!("Failed to get blockhash: {e}"))?;
        Ok(Hash::from_str(&res.value.blockhash).unwrap())
    }

    pub(crate) async fn svm_send_transaction(&self, tx: &Transaction) -> anyhow::Result<Signature> {
        let raw_tx = serialize_and_encode(tx, UiTransactionEncoding::Base58)?;
        let res: String = self
            .rpc
            .request("sendTransaction", rpc_params![raw_tx])
            .await
            .map_err(|e| anyhow!("Failed to send transaction: {e}"))?;
        Ok(Signature::from_str(&res).unwrap())
    }

    pub(crate) async fn get_minimum_balance_for_rent_exemption(
        &self,
        data_len: usize,
    ) -> anyhow::Result<u64> {
        self.rpc
            .request("getMinimumBalanceForRentExemption", rpc_params![data_len])
            .await
            .map_err(|e| anyhow!("Failed to get rent balance: {e}"))
    }

    pub(crate) async fn get_fee_for_message(&self, message: Message) -> anyhow::Result<u64> {
        let encoded = serialize_and_encode(&message, UiTransactionEncoding::Base64)?;
        let res: Response<Option<u64>> = self
            .rpc
            .request("getFeeForMessage", rpc_params![encoded])
            .await
            .map_err(|e| anyhow!("Failed to get fee for message: {e}"))?;
        res.value.ok_or(anyhow!("Failed to get fee for message"))
    }

    pub(crate) async fn simulate_call_message(
        &self,
        transaction: &Transaction,
    ) -> anyhow::Result<u64> {
        let raw_tx = borsh::to_vec(transaction).expect("Failed to serialize transaction");

        let user = TestUser::<SvmRollupSpec>::generate_with_default_balance();

        let partial_transaction: PartialTransaction =
            sov_rollup_apis::PartialTransaction::<SvmRollupSpec> {
                sender_pub_key: user.private_key().pub_key(),
                details: sov_modules_api::transaction::TxDetails {
                    max_priority_fee_bips: PriorityFeeBips::ZERO,
                    max_fee: TEST_DEFAULT_MAX_FEE,
                    gas_limit: None,
                    chain_id: config_chain_id(),
                },
                encoded_call_message: <RT as EncodeCall<SVM<SvmRollupSpec>>>::encode_call(raw_tx),
                gas_price: None,
                sequencer: None,
                sequencer_rollup_address: None,
                generation: 0,
            }
            .try_into()
            .unwrap();

        let simulation_result = self
            .node_client
            .client
            .simulate(&SimulateBody {
                body: partial_transaction,
            })
            .await
            .map_err(|e| anyhow!("Failed to simulate call message: {e}"))?
            .data
            .clone()
            .unwrap();

        let simulation_result_parsed: SimulateExecutionContainer<SvmRollupSpec> =
            simulation_result.try_into()?;

        let tx_consumption = simulation_result_parsed
            .apply_tx_result
            .transaction_consumption;
        let base_fee = tx_consumption.base_fee_value().0;
        let priority_fee = tx_consumption.priority_fee().0;

        Ok(base_fee
            .checked_add(priority_fee)
            .unwrap()
            .0
            .try_into()
            .expect("balance not to overflow u64"))
    }

    pub(crate) async fn get_total_fee_for_transaction(
        &self,
        transaction: &Transaction,
    ) -> anyhow::Result<u64> {
        let call_message_gas_fee_estimate = self.simulate_call_message(transaction).await?;
        let svm_gas_fee_estimate = self
            .get_fee_for_message(transaction.message.clone())
            .await?;
        Ok(call_message_gas_fee_estimate + svm_gas_fee_estimate)
    }
}
