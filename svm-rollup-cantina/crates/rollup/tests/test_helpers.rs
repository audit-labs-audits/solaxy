use std::path::Path;

use sov_cli::wallet_state::PrivateKeyAndAddress;
use sov_mock_da::BlockProducingConfig;
use sov_modules_api::{execution_mode::Native, OperatingMode, Spec};
use sov_stf_runner::processes::RollupProverConfig;
use sov_test_utils::test_rollup::{GenesisSource, RollupBuilder, TestRollup};
use stf::genesis_config::GenesisPaths;
use svm_rollup::{
    da::SupportedDaLayer,
    mock_rollup::MockSvmRollup,
    zk::{rollup_host_args, InnerZkvm},
};

pub fn read_private_keys<S: Spec>(suffix: &str) -> PrivateKeyAndAddress<S> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    let private_keys_dir = Path::new(&manifest_dir).join("../test-data/keys");

    let data = std::fs::read_to_string(private_keys_dir.join(suffix))
        .expect("Unable to read file to string");

    let key_and_address: PrivateKeyAndAddress<S> =
        serde_json::from_str(&data).unwrap_or_else(|e| {
            panic!(
                "Unable to convert data {} to PrivateKeyAndAddress: {e:?}",
                &data
            );
        });

    assert!(
        key_and_address.is_matching_to_default(),
        "Inconsistent key data"
    );

    key_and_address
}

pub async fn start_rollup_in_background(
    rt_genesis_paths: GenesisPaths,
    rollup_prover_config: RollupProverConfig<InnerZkvm>,
    finalization_blocks: u32,
) -> TestRollup<MockSvmRollup<Native>> {
    RollupBuilder::new(
        GenesisSource::Paths(rt_genesis_paths),
        BlockProducingConfig::Periodic { block_time_ms: 150 },
        finalization_blocks,
    )
    .with_zkvm_host_args(rollup_host_args(SupportedDaLayer::Mock))
    .set_config(|c| {
        c.automatic_batch_production = true;
        c.rollup_prover_config = Some(rollup_prover_config);
        c.aggregated_proof_block_jump = 5;
        c.max_infos_in_db = 30;
        c.max_channel_size = 20;
        c.sequencer_address = "HjjEhif8MU9DtnXtZc5hkBu9XLAkAYe1qwzhDoxbcECv".to_string();
    })
    .start()
    .await
    .unwrap()
}

pub fn get_appropriate_rollup_prover_config() -> RollupProverConfig<InnerZkvm> {
    let skip_guest_build = std::env::var("SKIP_GUEST_BUILD").unwrap_or_else(|_| "0".to_string());
    if skip_guest_build == "1" {
        RollupProverConfig::Skip
    } else {
        RollupProverConfig::Execute(rollup_host_args(SupportedDaLayer::Mock))
    }
}

pub fn test_genesis_paths(operating_mode: OperatingMode) -> GenesisPaths {
    let dir: &dyn AsRef<Path> = &"../test-data/genesis/integration-tests/";

    let mut paths = GenesisPaths::from_dir(dir.as_ref());
    paths.chain_state_genesis_path = match operating_mode {
        OperatingMode::Zk => dir.as_ref().join("chain_state_zk.json"),
        OperatingMode::Optimistic => dir.as_ref().join("chain_state_op.json"),
    };

    paths
}
