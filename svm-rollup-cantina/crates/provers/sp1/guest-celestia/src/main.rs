#![no_main]
sp1_zkvm::entrypoint!(main);

use const_rollup_config::{ROLLUP_BATCH_NAMESPACE_RAW, ROLLUP_PROOF_NAMESPACE_RAW};
use sov_celestia_adapter::{
    types::Namespace,
    verifier::{CelestiaSpec, CelestiaVerifier, RollupParams},
};
use sov_mock_zkvm::MockZkvm;
use sov_modules_api::execution_mode::Zk;
use sov_modules_stf_blueprint::StfBlueprint;
use sov_rollup_interface::da::DaVerifier;
use sov_sp1_adapter::{guest::SP1Guest, SP1CryptoSpec, SP1};
use sov_state::{DefaultStorageSpec, ZkStorage};
use stf::{runtime::Runtime, StfVerifier};
use svm_types::SolanaSpec;

type Storage =
    ZkStorage<DefaultStorageSpec<<SP1CryptoSpec as sov_modules_api::CryptoSpec>::Hasher>>;

// The rollup stores its data in the namespace b"sov-test" on Celestia
const ROLLUP_BATCH_NAMESPACE: Namespace = Namespace::const_v0(ROLLUP_BATCH_NAMESPACE_RAW);
const ROLLUP_PROOF_NAMESPACE: Namespace = Namespace::const_v0(ROLLUP_PROOF_NAMESPACE_RAW);

pub fn main() {
    let guest = SP1Guest::new();
    let storage = ZkStorage::new();
    let stf: StfBlueprint<
        SolanaSpec<CelestiaSpec, SP1, MockZkvm, SP1CryptoSpec, Zk, Storage>,
        Runtime<_>,
    > = StfBlueprint::new();

    let stf_verifier = StfVerifier::<_, _, _, _, _>::new(
        stf,
        CelestiaVerifier::new(RollupParams {
            rollup_batch_namespace: ROLLUP_BATCH_NAMESPACE,
            rollup_proof_namespace: ROLLUP_PROOF_NAMESPACE,
        }),
    );
    stf_verifier
        .run_block(guest, storage)
        .expect("Prover must be honest");
}
