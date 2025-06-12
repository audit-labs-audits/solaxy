use std::sync::Arc;

use anchor_lang::prelude::Pubkey;
use nitro_da_client::Priority;
use solana_rpc_client::{
    mock_sender::MockSender, nonblocking::rpc_client::RpcClient, rpc_client::RpcClientConfig,
};
use solana_rpc_client_api::{client_error, request::RpcRequest};

use crate::{
    client_error_is_serde_null_data, native::service::SolanaService, SolanaChainParams,
    SolanaServiceConfig,
};

fn test_chain_params() -> SolanaChainParams {
    SolanaChainParams {
        blober_batch: Pubkey::new_unique(),
        blober_proofs: Pubkey::new_unique(),
    }
}

fn test_service_config() -> SolanaServiceConfig {
    SolanaServiceConfig {
        indexer_url: String::new(),
        priority: Priority::default(),
        program_id: Pubkey::new_unique().into(),
    }
}

#[tokio::test]
async fn test_block_not_confirmed_rpc() {
    // According to the Solana API, null will be returned if the block is not confirmed, but
    // it's not clear how that's returned by the RPC client. Let's check.

    let client = RpcClient::new_sender(
        MockSender::new_with_mocks(
            "succeeds".to_string(),
            [(RpcRequest::GetBlock, serde_json::Value::Null)]
                .into_iter()
                .collect(),
        ),
        RpcClientConfig::default(),
    );

    let result = client.get_block(0).await;
    let e = result.unwrap_err();
    let client_error::ErrorKind::SerdeJson(e2) = e.kind() else {
        panic!("Expected a serde error, got {e:?}");
    };
    assert_eq!(e2.classify(), serde_json::error::Category::Data);
    assert_eq!(e2.column(), 0);
    assert_eq!(e2.line(), 0);
    assert!(client_error_is_serde_null_data(&e));
}

#[tokio::test]
async fn test_block_not_confirmed_retries() {
    // Ensure that we retry if the block is not yet confirmed.

    let client = Arc::new(RpcClient::new_sender(
        MockSender::new_with_mocks(
            "succeeds".to_string(),
            [
                // This will fail the first time, but succeed the second time,
                // since the MockSender removes the mock data after the first call.
                (RpcRequest::GetBlock, serde_json::Value::Null),
            ]
            .into_iter()
            .collect(),
        ),
        RpcClientConfig::default(),
    ));

    let service = SolanaService::builder()
        .solana_rpc_client(client)
        .params(test_chain_params())
        .config(test_service_config())
        .build();

    service.rpc_get_block(100).await.unwrap();
}
