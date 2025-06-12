#![no_main]

sp1_zkvm::entrypoint!(main);

use sov_mock_da::{MockDaSpec, MockDaVerifier};
use sov_mock_zkvm::MockZkvm;
use sov_modules_api::execution_mode::Zk;
use sov_modules_stf_blueprint::StfBlueprint;
use sov_sp1_adapter::{guest::SP1Guest, SP1CryptoSpec, SP1};
use sov_state::{DefaultStorageSpec, ZkStorage};
use stf::{runtime::Runtime, StfVerifier};
use svm_types::SolanaSpec;

type Storage =
    ZkStorage<DefaultStorageSpec<<SP1CryptoSpec as sov_modules_api::CryptoSpec>::Hasher>>;

#[allow(unexpected_cfgs)]
#[cfg_attr(feature = "bench", sov_modules_api::cycle_tracker)]
pub fn main() {
    let guest = SP1Guest::new();
    let storage = ZkStorage::new();
    let stf: StfBlueprint<
        SolanaSpec<MockDaSpec, SP1, MockZkvm, SP1CryptoSpec, Zk, Storage>,
        Runtime<_>,
    > = StfBlueprint::new();

    let stf_verifier = StfVerifier::<_, _, _, _, _>::new(stf, MockDaVerifier {});

    stf_verifier
        .run_block(guest, storage)
        .expect("Prover must be honest");
}
