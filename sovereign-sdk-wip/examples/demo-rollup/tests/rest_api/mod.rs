use std::str::FromStr;
use std::time::Duration;

use anyhow::Context;
use demo_stf::runtime::{Runtime, RuntimeCall};
use demo_stf_json_client::types::{RuntimeAnyJsonValue, RuntimeErrorContainer};
use demo_stf_json_client::Error;
use futures::StreamExt;
use serde::Deserialize;
use sov_bank::config_gas_token_id;
use sov_cli::wallet_state::PrivateKeyAndAddress;
use sov_demo_rollup::{mock_da_risc0_host_args, MockDemoRollup};
use sov_modules_api::execution_mode::Native;
use sov_modules_api::rest::utils::ResponseObject;
use sov_modules_api::OperatingMode;
use sov_test_utils::test_rollup::{read_private_key, RollupBuilder};
use sov_test_utils::{
    default_test_signed_transaction, TEST_DEFAULT_MOCK_DA_ON_SUBMIT,
    TEST_DEFAULT_MOCK_DA_PERIODIC_PRODUCING,
};

use crate::test_helpers::{test_genesis_source, DemoRollupSpec, CHAIN_HASH};

type TestSpec = DemoRollupSpec;

#[derive(Debug, Deserialize)]
struct ValueResponse {
    value: [u64; 2],
}

#[tokio::test(flavor = "multi_thread")]
async fn trailing_slashes_handled() -> anyhow::Result<()> {
    let test_rollup = RollupBuilder::<MockDemoRollup<Native>>::new(
        test_genesis_source(OperatingMode::Zk),
        TEST_DEFAULT_MOCK_DA_ON_SUBMIT,
        0,
    )
    .with_zkvm_host_args(mock_da_risc0_host_args())
    .with_standard_sequencer()
    .start()
    .await?;

    let response = test_rollup
        .client
        .query_rest_endpoint::<ResponseObject<ValueResponse>>(
            "/modules/attester-incentives/state/minimum-challenger-bond",
        )
        .await?;

    let bond = response.data.unwrap().value;

    let response = test_rollup
        .client
        .query_rest_endpoint::<ResponseObject<ValueResponse>>(
            "/modules/attester-incentives/state/minimum-challenger-bond/",
        )
        .await?;

    assert_eq!(Some(bond), response.data.map(|d| d.value));

    let swagger_ui_url_1 = test_rollup.client.http_get("/swagger-ui").await?;
    let swagger_ui_url_2 = test_rollup.client.http_get("/swagger-ui/").await?;

    assert_eq!(swagger_ui_url_1, swagger_ui_url_2);

    Ok(())
}

async fn setup() -> anyhow::Result<demo_stf_json_client::Client> {
    let test_rollup = RollupBuilder::<MockDemoRollup<Native>>::new(
        test_genesis_source(OperatingMode::Zk),
        TEST_DEFAULT_MOCK_DA_PERIODIC_PRODUCING,
        0,
    )
    .with_zkvm_host_args(mock_da_risc0_host_args())
    .start()
    .await?;

    test_rollup
        .da_service
        .produce_n_blocks_now(5)
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Based on an assumption that this key is admin in sov-value-setter
    let key_and_address = read_private_key::<TestSpec>("tx_signer_private_key.json");
    let msg =
        RuntimeCall::<TestSpec>::ValueSetter(sov_value_setter::CallMessage::SetManyValues(vec![
            1, 2, 3, 4, 5, 6, 7, 8,
        ]));

    let tx = default_test_signed_transaction::<Runtime<TestSpec>, TestSpec>(
        &key_and_address.private_key,
        &msg,
        0,
        &CHAIN_HASH,
    );
    let mut slot_subscription = test_rollup.client.client.subscribe_slots().await?;
    test_rollup
        .client
        .client
        .send_txs_to_sequencer(&[tx])
        .await?;
    slot_subscription.next().await;

    test_rollup
        .da_service
        .produce_n_blocks_now(3)
        .await
        .unwrap();

    Ok(demo_stf_json_client::Client::new(
        &test_rollup.client.base_url,
    ))
}

#[tokio::test(flavor = "multi_thread")]
async fn flaky_test_runtime_spec_with_gen_client() -> anyhow::Result<()> {
    let runtime_client = setup().await?;

    check_base_runtime_info(&runtime_client)
        .await
        .context("Base runtime check")?;
    check_state_value(&runtime_client)
        .await
        .context("StateValue")?;
    check_state_map(&runtime_client).await.context("StateMap")?;
    check_state_vec(&runtime_client).await.context("StateVec")?;
    check_custom_endpoints(&runtime_client)
        .await
        .context("Custom endpoints")?;
    check_historical_data(&runtime_client)
        .await
        .context("Historical data")?;

    Ok(())
}

async fn check_base_runtime_info(client: &demo_stf_json_client::Client) -> anyhow::Result<()> {
    //Get the list of modules
    let modules = client.get_modules().await.context("Modules list")?;
    let modules = modules.data.clone().unwrap().modules.unwrap().0;

    // There are some modules, but to make test break less we have loose check that some data has been put in the response.
    assert!(modules.len() > 5);

    let bank = modules.get("bank").unwrap().clone();
    assert!(bank.id.starts_with("module_1"));

    let bank_module_info = client
        .get_module("bank")
        .await
        .context("Bank module info")?;
    let bank_module_info = bank_module_info.data.clone().unwrap();
    let module_name = bank_module_info.name.unwrap();
    assert_eq!("Bank", module_name);
    assert!(bank_module_info.id.unwrap().starts_with("module_1"));
    assert!(bank_module_info.prefix.unwrap().starts_with("0x"));
    assert!(!bank_module_info.description.unwrap().is_empty());

    let state_items = bank_module_info.state_items;

    let mut state_item_keys: Vec<String> = state_items.keys().cloned().collect();
    state_item_keys.sort();
    assert_eq!(vec!["balances", "tokens"], state_item_keys);

    let value = state_items.get("tokens").unwrap();

    assert_eq!(
        Some(demo_stf_json_client::types::Namespace::User),
        value.namespace
    );

    assert_eq!(Some("state_map"), value.type_.as_deref());
    assert!(value.prefix.clone().unwrap().starts_with("0x"));
    assert!(!value.description.clone().unwrap().is_empty());

    // French bank:
    let unknown_module_info = client.get_module("le-banquoe").await.unwrap_err();

    assert_eq!(
        Some(reqwest::StatusCode::NOT_FOUND),
        unknown_module_info.status()
    );
    check_not_found_error(
        unknown_module_info,
        "Not Found",
        "url",
        "/modules/le-banquoe",
    );
    Ok(())
}

async fn check_state_value(client: &demo_stf_json_client::Client) -> anyhow::Result<()> {
    // Known value: u64
    let finality_period = client
        .attester_incentives_rollup_finality_period_get_state_value(None, None)
        .await?;
    let finality_period = if let RuntimeAnyJsonValue::Object(inner) = &finality_period.data {
        let value = inner
            .get("value")
            .cloned()
            .expect("finality period must be set");
        value.as_u64().expect("finality period must be a number")
    } else {
        panic!(
            "Unexpected type of finality period response: {:?}",
            &finality_period.data
        );
    };
    assert!(finality_period >= 1);

    // Empty value
    let empty_value = client
        .value_setter_value_get_state_value(None, None)
        .await?;
    match &empty_value.data {
        RuntimeAnyJsonValue::Object(inner) => {
            let value = inner.get("value");
            assert_eq!(Some(&serde_json::Value::Null), value);
        }
        _ => panic!("Unexpected type for non set StateValue"),
    }

    Ok(())
}

async fn check_state_map(client: &demo_stf_json_client::Client) -> anyhow::Result<()> {
    // State Map meta info
    let meta_info = client
        .sequencer_registry_known_sequencers_get_state_map_info(None, None)
        .await?;

    let meta_info = meta_info.data.clone();

    assert!(!meta_info.description.unwrap().is_empty());
    assert!(meta_info.prefix.unwrap().starts_with("0x"));
    assert_eq!(Some("state_map"), meta_info.type_.as_deref());
    assert_eq!(
        Some(demo_stf_json_client::types::Namespace::Kernel),
        meta_info.namespace
    );
    Ok(())
}

async fn check_state_vec(client: &demo_stf_json_client::Client) -> anyhow::Result<()> {
    let state_vec_info = client
        .value_setter_many_values_get_state_vec_info(None, None)
        .await
        .context("vector info")?;
    let info = state_vec_info.data.clone().length.unwrap();
    assert_eq!(8, info);

    let state_vec_element_0 = client
        .value_setter_many_values_get_state_vec_element(0, None, None)
        .await
        .context("first element")?;

    let state_vec_element_1 = client
        .value_setter_many_values_get_state_vec_element(1, None, None)
        .await?;
    let state_vec_element_last = client
        .value_setter_many_values_get_state_vec_element(7, None, None)
        .await
        .context("last element")?;

    let value_0_json = state_vec_element_0.data.clone().value;
    let value_1_json = state_vec_element_1.data.clone().value;
    let value_last_json = state_vec_element_last.data.clone().value;

    match (value_0_json, value_1_json, value_last_json) {
        (
            RuntimeAnyJsonValue::Number(value_0),
            RuntimeAnyJsonValue::Number(value_1),
            RuntimeAnyJsonValue::Number(value_last),
        ) => {
            assert_eq!(1.0, value_0);
            assert_eq!(2.0, value_1);
            assert_eq!(8.0, value_last);
        }
        (_, _, _) => panic!("Incorrect type returned in vector"),
    }

    let state_vec_out_of_bounds = client
        .value_setter_many_values_get_state_vec_element(u16::MAX as u64, None, None)
        .await
        .unwrap_err();

    assert_eq!(
        Some(reqwest::StatusCode::NOT_FOUND),
        state_vec_out_of_bounds.status()
    );
    check_not_found_error(
        state_vec_out_of_bounds,
        "many_values '65535' not found",
        // TODO: Should it be index. Offloading it to item of better id handling.
        "id",
        "65535",
    );
    Ok(())
}

async fn check_custom_endpoints(client: &demo_stf_json_client::Client) -> anyhow::Result<()> {
    let gas_token_id =
        demo_stf_json_client::types::TokenId::from_str(&config_gas_token_id().to_string())?;
    let total_gas_supply = client
        .bank_custom_token_get_total_supply(&gas_token_id)
        .await?;
    let coins = total_gas_supply.data.clone().unwrap();
    let amount = coins.amount.unwrap().parse::<u128>().unwrap();
    assert!(amount > 100);

    assert_eq!(gas_token_id, coins.token_id.unwrap());

    // Unknown user
    let unknown = PrivateKeyAndAddress::<TestSpec>::generate();
    let address = demo_stf_json_client::types::Address(unknown.address.to_string());
    let balance_error = client
        .bank_custom_token_get_user_balance(&gas_token_id, &address)
        .await
        .unwrap_err();

    assert_eq!(Some(reqwest::StatusCode::NOT_FOUND), balance_error.status());
    let expected_title = format!("Balance '{}' not found", unknown.address);
    check_not_found_error(
        balance_error,
        &expected_title,
        "id",
        &unknown.address.to_string(),
    );

    Ok(())
}

async fn check_historical_data(client: &demo_stf_json_client::Client) -> anyhow::Result<()> {
    let state_vec_info = client
        .value_setter_many_values_get_state_vec_info(Some(0), None)
        .await?;
    let info = state_vec_info.data.clone().length.unwrap();
    assert_eq!(0, info);

    // StateVec info is zero in the future
    let state_vec_err = client
        .value_setter_many_values_get_state_vec_info(Some(u32::MAX as u64), None)
        .await
        .unwrap_err();

    match &state_vec_err {
        Error::UnexpectedResponse(response) => {
            assert_eq!(response.status(), 404);
        }
        _ => panic!("Should have gotten an unexpected response"),
    }

    let state_vec_element_response = client
        .value_setter_many_values_get_state_vec_element(1, Some(u32::MAX as u64), None)
        .await
        .unwrap_err();
    check_not_found_error(
        state_vec_element_response,
        "invalid rollup height",
        "message",
        "Impossible to get the rollup state at the specified height. Please ensure you have queried the correct height."
    );
    Ok(())
}

fn check_not_found_error(
    credential_id_response: demo_stf_json_client::Error<RuntimeErrorContainer>,
    expected_title: &str,
    expected_details_key: &str,
    expected_key: &str,
) {
    match credential_id_response {
        demo_stf_json_client::Error::ErrorResponse(inner_err) => {
            assert_eq!(1, inner_err.errors.len());
            let error = inner_err.errors.first().unwrap();

            assert_eq!(expected_title, error.title);
            assert_eq!(404, error.status);
            match &error.details {
                RuntimeAnyJsonValue::Object(details) => {
                    assert_eq!(1, details.len());
                    assert!(details.contains_key(expected_details_key));
                    assert_eq!(
                        Some(serde_json::Value::String(expected_key.to_string())),
                        details.get(expected_details_key).cloned()
                    );
                }
                _ => panic!("unexpected details type: {:?}", error.details),
            }
        }
        _ => {
            panic!("Unexpected error response: {:?}", credential_id_response)
        }
    };
}
