use std::sync::{atomic::AtomicU64, Arc};

use sov_db::{schema::SchemaBatch, storage_manager::NativeStorageManager};
use sov_mock_da::{MockAddress, MockBlock, MockDaService, MockDaSpec};
use sov_mock_zkvm::MockZkvm;
use sov_modules_api::{execution_mode::WitnessGeneration, OperatingMode, SlotData};
use sov_modules_stf_blueprint::{GenesisParams, StfBlueprint};
use sov_rollup_interface::{
    da::BlockHeaderTrait,
    node::da::DaService,
    stf::{ExecutionContext, StateTransitionFunction},
    storage::HierarchicalStorageManager,
    zk::{StateTransitionWitness, StateTransitionWitnessWithAddress, ZkvmHost},
};
use sov_sp1_adapter::{host::SP1Host, SP1CryptoSpec, SP1};
use sov_state::ProverStorage;
use sov_test_utils::{generators::BlobBuildingCtx, TestStorageSpec};
use sp1::SP1_GUEST_MOCK_ELF;
use stf::{genesis_config::create_genesis_config, runtime::Runtime};
use svm_types::{MultiAddressSvm, NativeStorage};
use tempfile::TempDir;

use crate::{prover::datagen::get_blocks_from_da, test_helpers::test_genesis_paths};

type DefaultSpec = sov_modules_api::configurable_spec::ConfigurableSpec<
    MockDaSpec,
    sov_sp1_adapter::SP1,
    sov_mock_zkvm::MockZkvm,
    SP1CryptoSpec,
    MultiAddressSvm,
    WitnessGeneration,
    NativeStorage,
>;

mod datagen;

type TestSTF<'a> = StfBlueprint<DefaultSpec, Runtime<DefaultSpec>>;

/// This test reproduces the proof generation process for the rollup used in benchmarks.
#[tokio::test]
#[ignore]
async fn test_proof_generation() {
    let temp_dir = TempDir::new().expect("Unable to create temporary directory");
    tracing::info!("Creating temp dir at {}", temp_dir.path().display());
    let da_service = MockDaService::new(MockAddress::default());
    let sequencer_mode = BlobBuildingCtx::Preferred {
        curr_sequence_number: Arc::new(AtomicU64::new(0)),
    };

    let mut storage_manager =
        NativeStorageManager::<MockDaSpec, ProverStorage<TestStorageSpec>>::new(temp_dir.path())
            .expect("NativeStorageManager initialization has failed");
    let stf = TestSTF::new();

    let genesis_config = {
        let rt_params =
            create_genesis_config::<DefaultSpec>(&test_genesis_paths(OperatingMode::Zk)).unwrap();
        GenesisParams { runtime: rt_params }
    };

    tracing::info!("Starting from empty storage, initialization chain");
    let genesis_block = MockBlock::default();
    let (stf_state, _) = storage_manager
        .create_state_for(genesis_block.header())
        .unwrap();
    let (mut prev_state_root, stf_state) =
        stf.init_chain(&Default::default(), stf_state, genesis_config);
    storage_manager
        .save_change_set(genesis_block.header(), stf_state, SchemaBatch::new())
        .unwrap();
    // Write it to the database immediately!
    storage_manager.finalize(&genesis_block.header).unwrap();

    // TODO: Fix this with genesis logic.
    let mut blocks = get_blocks_from_da(sequencer_mode)
        .await
        .expect("Failed to get DA blocks");

    let elf = &SP1_GUEST_MOCK_ELF;

    let mut host = SP1Host::new(elf);

    for filtered_block in &mut blocks[..3] {
        let height = filtered_block.header().height();
        tracing::info!(
            "Requesting data for height {} and prev_state_root 0x{}",
            height,
            hex::encode(prev_state_root.root_hash())
        );
        let (mut relevant_blobs, relevant_proofs) = da_service
            .extract_relevant_blobs_with_proof(filtered_block)
            .await;

        let (stf_state, _) = storage_manager
            .create_state_for(filtered_block.header())
            .unwrap();

        let result = stf.apply_slot(
            &prev_state_root,
            stf_state,
            Default::default(),
            filtered_block.header(),
            relevant_blobs.as_iters(),
            ExecutionContext::Node,
        );

        let data = StateTransitionWitness::<
            <TestSTF as StateTransitionFunction<SP1, MockZkvm, MockDaSpec>>::StateRoot,
            <TestSTF as StateTransitionFunction<SP1, MockZkvm, MockDaSpec>>::Witness,
            MockDaSpec,
        > {
            initial_state_root: prev_state_root,
            da_block_header: filtered_block.header().clone(),
            relevant_proofs,
            witness: result.witness,
            relevant_blobs,
            final_state_root: result.state_root,
        };

        let data = StateTransitionWitnessWithAddress {
            stf_witness: data,
            prover_address: MockAddress::default(),
        };

        host.add_hint(data);

        tracing::info!("Run prover without generating a proof for block {height}\n");
        let _receipt = host.run(false).expect("Prover should run successfully");
        tracing::info!("==================================================\n");

        prev_state_root = result.state_root;
        storage_manager
            .save_change_set(
                filtered_block.header(),
                result.change_set,
                SchemaBatch::new(),
            )
            .unwrap();
    }
}
