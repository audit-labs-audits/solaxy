#![no_main]
sp1_zkvm::entrypoint!(main);

use std::str::FromStr;

use const_rollup_config::{SOLANA_BLOBER_BATCH_ADDRESS, SOLANA_BLOBER_PROOFS_ADDRESS};
use solana_da::{SolanaChainParams, SolanaDASpec, SolanaVerifier};
use solana_sdk::pubkey::Pubkey;
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

pub fn main() {
    let guest = SP1Guest::new();
    let storage = ZkStorage::new();
    let stf: StfBlueprint<
        SolanaSpec<SolanaDASpec, SP1, MockZkvm, SP1CryptoSpec, Zk, Storage>,
        Runtime<_>,
    > = StfBlueprint::new();

    let stf_verifier = StfVerifier::<_, _, _, _, _>::new(
        stf,
        SolanaVerifier::new(SolanaChainParams {
            blober_batch: Pubkey::from_str(SOLANA_BLOBER_BATCH_ADDRESS).unwrap(),
            blober_proofs: Pubkey::from_str(SOLANA_BLOBER_PROOFS_ADDRESS).unwrap(),
        }),
    );
    stf_verifier
        .run_block(guest, storage)
        .expect("Failed to verify proof");
}
