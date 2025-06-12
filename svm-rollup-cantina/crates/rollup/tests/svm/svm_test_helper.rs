use std::net::SocketAddr;

use sov_modules_api::execution_mode::Native;
use sov_stf_runner::processes::RollupProverConfig;
use sov_test_utils::test_rollup::TestRollup;
use stf::genesis_config::GenesisPaths;
use svm_rollup::{mock_rollup::MockSvmRollup, zk::InnerZkvm};

use crate::{svm::test_client::TestClient, test_helpers::start_rollup_in_background};

pub const TEST_GENESIS_CONFIG_DIR: &str = "../test-data/genesis/integration-tests";

/// Starts test rollup node with MockDA.
pub(crate) async fn start_node(
    rollup_prover_config: RollupProverConfig<InnerZkvm>,
    finalization_blocks: u32,
) -> TestRollup<MockSvmRollup<Native>> {
    let rt_genesis_paths = GenesisPaths::from_dir(TEST_GENESIS_CONFIG_DIR);

    start_rollup_in_background(rt_genesis_paths, rollup_prover_config, finalization_blocks).await
}

/// Creates a test client to communicate with the rollup node.
pub(crate) async fn create_test_client(port: SocketAddr) -> TestClient {
    TestClient::new(port).await
}
