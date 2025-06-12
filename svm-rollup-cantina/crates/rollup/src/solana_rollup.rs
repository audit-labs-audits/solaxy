use std::sync::Arc;

use async_trait::async_trait;
use solana_da::SolanaDASpec;
use sov_address::FromVmAddress;
use sov_db::{ledger_db::LedgerDb, storage_manager::NativeStorageManager};
use sov_mock_zkvm::MockCodeCommitment;
use sov_modules_api::{
    execution_mode::{Native, WitnessGeneration},
    rest::StateUpdateReceiver,
    CryptoSpec, NodeEndpoints, Spec, SyncStatus, ZkVerifier,
};
use sov_modules_rollup_blueprint::{
    pluggable_traits::PluggableSpec, proof_sender::SovApiProofSender, FullNodeBlueprint,
    RollupBlueprint, SequencerCreationReceipt, WalletBlueprint,
};
use sov_rollup_interface::zk::aggregated_proof::CodeCommitment;
use sov_sequencer::{ProofBlobSender, Sequencer};
use sov_state::{DefaultStorageSpec, ProverStorage, Storage};
use sov_stf_runner::{
    processes::{ParallelProverService, ProverService, RollupProverConfig},
    RollupConfig,
};
use stf::{genesis_config::GenesisPaths, runtime::Runtime};
use svm::sequencer::{FaucetConfig, SvmSequencerRpc, SvmSequencerServer};
use svm_types::{NativeStorage, SolanaAddress, SolanaSpec};
use tokio::sync::watch;

use crate::{
    da::solana::{new_da_service, new_verifier, DaService, DaSpec},
    zk::{get_outer_vm, InnerCryptoSpec, InnerZkvm, InnerZkvmHost, OuterZkvm},
};

/// Rollup with SolanaDa
pub struct SolanaRollup<M> {
    genesis_paths: GenesisPaths,
    phantom: std::marker::PhantomData<M>,
}

impl<M> SolanaRollup<M> {
    pub fn new(genesis_paths: GenesisPaths) -> Self {
        Self {
            genesis_paths,
            phantom: std::marker::PhantomData,
        }
    }
}

type SvmRollupSpec<M> = SolanaSpec<DaSpec, InnerZkvm, OuterZkvm, InnerCryptoSpec, M, NativeStorage>;

impl RollupBlueprint<Native> for SolanaRollup<Native>
where
    SvmRollupSpec<Native>: PluggableSpec,
    <SvmRollupSpec<Native> as Spec>::Address: FromVmAddress<SolanaAddress>,
{
    type Spec = SvmRollupSpec<Native>;
    type Runtime = Runtime<Self::Spec>;
}

impl RollupBlueprint<WitnessGeneration> for SolanaRollup<WitnessGeneration>
where
    SvmRollupSpec<WitnessGeneration>: PluggableSpec,
    <SvmRollupSpec<WitnessGeneration> as Spec>::Address: FromVmAddress<SolanaAddress>,
{
    type Spec = SvmRollupSpec<WitnessGeneration>;
    type Runtime = Runtime<Self::Spec>;
}

#[async_trait]
impl FullNodeBlueprint<Native> for SolanaRollup<Native> {
    type DaService = DaService;
    /// Manager for the native storage lifecycle.
    type StorageManager = NativeStorageManager<
        SolanaDASpec,
        ProverStorage<DefaultStorageSpec<<<Self::Spec as Spec>::CryptoSpec as CryptoSpec>::Hasher>>,
    >;
    /// Prover service.
    type ProverService = ParallelProverService<
        <Self::Spec as Spec>::Address,
        <<Self::Spec as Spec>::Storage as Storage>::Root,
        <<Self::Spec as Spec>::Storage as Storage>::Witness,
        Self::DaService,
        <Self::Spec as Spec>::InnerZkvm,
        <Self::Spec as Spec>::OuterZkvm,
    >;

    type ProofSender = SovApiProofSender<Self::Spec>;

    fn create_outer_code_commitment(
        &self,
    ) -> <<Self::ProverService as ProverService>::Verifier as ZkVerifier>::CodeCommitment {
        MockCodeCommitment::default()
    }

    async fn create_endpoints(
        &self,
        state_update_receiver: StateUpdateReceiver<<Self::Spec as Spec>::Storage>,
        sync_status_receiver: watch::Receiver<SyncStatus>,
        shutdown_receiver: tokio::sync::watch::Receiver<()>,
        ledger_db: &LedgerDb,
        sequencer: &SequencerCreationReceipt<Self::Spec>,
        _da_service: &Self::DaService,
        rollup_config: &RollupConfig<<Self::Spec as Spec>::Address, Self::DaService>,
    ) -> anyhow::Result<NodeEndpoints> {
        sov_modules_rollup_blueprint::register_endpoints::<Self, Native>(
            state_update_receiver.clone(),
            sync_status_receiver,
            shutdown_receiver,
            ledger_db,
            sequencer,
            rollup_config,
        )
        .await
    }

    async fn sequencer_additional_apis<Seq>(
        &self,
        sequencer: Arc<Seq>,
        rollup_config: &RollupConfig<<Self::Spec as Spec>::Address, Self::DaService>,
    ) -> anyhow::Result<NodeEndpoints>
    where
        Seq: Sequencer<Spec = Self::Spec, Rt = Self::Runtime, Da = Self::DaService>,
    {
        let genesis_params = self.create_genesis_config(&self.genesis_paths, rollup_config)?;
        let faucet_config = Arc::new(
            genesis_params
                .runtime
                .svm
                .faucet_keypair
                .map(|keypair| FaucetConfig::new(keypair.into(), None, None))
                .unwrap_or(FaucetConfig::Inactive),
        );
        Ok(NodeEndpoints {
            jsonrpsee_module: SvmSequencerServer::into_rpc(
                SvmSequencerRpc::<Seq, Self::Runtime>::new(sequencer, faucet_config),
            )
            .remove_context(),
            ..Default::default()
        })
    }

    async fn create_da_service(
        &self,
        rollup_config: &RollupConfig<<Self::Spec as Spec>::Address, Self::DaService>,
        shutdown_receiver: tokio::sync::watch::Receiver<()>,
    ) -> Self::DaService {
        new_da_service::<Self::Spec>(rollup_config, shutdown_receiver).await
    }

    async fn create_prover_service(
        &self,
        prover_config: RollupProverConfig<<Self::Spec as Spec>::InnerZkvm>,
        rollup_config: &RollupConfig<<Self::Spec as Spec>::Address, Self::DaService>,
        _da_service: &Self::DaService,
    ) -> Self::ProverService {
        let (elf, prover_config_disc) = prover_config.split();
        let inner_vm = InnerZkvmHost::new(*elf);
        let outer_vm = get_outer_vm();
        let da_verifier = new_verifier(
            rollup_config.da.program_id.into(),
            rollup_config.sequencer.da_address.into(),
        );

        ParallelProverService::new_with_default_workers(
            inner_vm,
            outer_vm,
            da_verifier,
            prover_config_disc,
            CodeCommitment::default(),
            rollup_config.proof_manager.prover_address,
        )
    }

    fn create_storage_manager(
        &self,
        rollup_config: &RollupConfig<<Self::Spec as Spec>::Address, Self::DaService>,
    ) -> anyhow::Result<Self::StorageManager> {
        NativeStorageManager::new(&rollup_config.storage.path)
    }

    fn create_proof_sender(
        &self,
        _rollup_config: &RollupConfig<<Self::Spec as Spec>::Address, Self::DaService>,
        sequence_number_provider: Arc<dyn ProofBlobSender>,
    ) -> anyhow::Result<Self::ProofSender> {
        Ok(Self::ProofSender::new(sequence_number_provider))
    }
}

impl WalletBlueprint<Native> for SolanaRollup<Native> {}
