//! Benchmarking utilities

use solana_sdk::{account::AccountSharedData, native_token::LAMPORTS_PER_SOL, system_program};
use sov_address::MultiAddress;
use sov_bank::Amount;
use sov_mock_da::BlockProducingConfig;
use sov_modules_api::{
    configurable_spec::ConfigurableSpec, execution_mode::Native, CryptoSpecExt, Spec, ZkVerifier,
    Zkvm,
};
use sov_test_utils::{
    runtime::genesis::zk::config::{HighLevelZkGenesisConfig, MinimalZkGenesisConfig},
    MockDaSpec, MockZkvm, TestPreferredSequencer, TestProver, TestUser,
};
use stf::runtime::GenesisConfig;
use svm::{
    test_utils::{get_test_payer, get_test_receiver},
    InitSvmConfig,
};
use svm_types::{NativeStorage, SolanaAddress};

pub const DEFAULT_BLOCK_TIME_MS: u64 = 150;
pub const DEFAULT_BLOCK_PRODUCING_CONFIG: BlockProducingConfig = BlockProducingConfig::Periodic {
    block_time_ms: DEFAULT_BLOCK_TIME_MS,
};
pub const DEFAULT_FINALIZATION_BLOCKS: u32 = 0;

/// [`ConfigurableSpec`] with [`MockDaSpec`] and a custom inner vm
pub type BenchSpec<Vm> = ConfigurableSpec<
    MockDaSpec,
    Vm,
    MockZkvm,
    <<Vm as Zkvm>::Verifier as ZkVerifier>::CryptoSpec,
    MultiAddress<SolanaAddress>,
    Native,
    NativeStorage,
>;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BenchmarkType {
    SolTransfer,
    SplSuite,
}

/// Benchmark user roles
pub struct Roles<S: Spec> {
    /// Admin of the bank module
    pub bank_admin: TestUser<S>,
    /// Default Prover
    pub prover: TestProver<S>,
    /// Initial preferred sequencer.
    pub preferred_sequencer: TestPreferredSequencer<S>,
    /// Transaction senders
    pub senders: Vec<TestUser<S>>,
}

/// Sets up benchmarks and returns the genesis config along with benchmark roles
pub fn setup<Vm: Zkvm>(
    num_senders: usize,
    inner_code_commitment: <Vm::Verifier as ZkVerifier>::CodeCommitment,
) -> (GenesisConfig<BenchSpec<Vm>>, Roles<BenchSpec<Vm>>)
where
    <Vm::Verifier as ZkVerifier>::CryptoSpec: CryptoSpecExt,
{
    let mut genesis_config =
        HighLevelZkGenesisConfig::generate_with_additional_accounts_and_code_commitments(
            3 + num_senders,
            inner_code_commitment,
            Default::default(),
        );

    genesis_config.initial_sequencer.bond = genesis_config
        .initial_sequencer
        .bond
        .checked_mul(Amount::new(num_senders as u128 * 10))
        .unwrap();

    let sequencer = TestPreferredSequencer::new(genesis_config.initial_sequencer.clone());
    let prover = genesis_config.initial_prover.clone();

    let extra_account = genesis_config.additional_accounts[2].clone();

    let senders = (0..num_senders)
        .map(|i| genesis_config.additional_accounts[i + 3].clone())
        .collect::<Vec<_>>();

    let minimal_config: MinimalZkGenesisConfig<BenchSpec<Vm>> = genesis_config.clone().into();

    // Setup: define Solana accounts that will be used in this test.
    let payer = get_test_payer();
    let receiver = get_test_receiver();
    let accounts = &[
        (
            payer,
            AccountSharedData::new(LAMPORTS_PER_SOL * 100, 0, &system_program::id()),
        ),
        (
            receiver,
            AccountSharedData::new(LAMPORTS_PER_SOL * 100, 0, &system_program::id()),
        ),
    ];

    let svm_config = InitSvmConfig::new(accounts, &[]);

    let genesis_config: GenesisConfig<BenchSpec<Vm>> = GenesisConfig {
        bank: minimal_config.bank,
        sequencer_registry: minimal_config.sequencer_registry,
        attester_incentives: minimal_config.attester_incentives,
        prover_incentives: minimal_config.prover_incentives,
        accounts: minimal_config.accounts,
        chain_state: minimal_config.chain_state,
        blob_storage: minimal_config.blob_storage,
        svm: svm_config,
    };

    (
        genesis_config,
        Roles {
            bank_admin: extra_account,
            senders,
            preferred_sequencer: sequencer,
            prover,
        },
    )
}

pub fn setup_with_genesis_and_roles<Vm: Zkvm>(
    num_senders: u64,
    inner_code_commitment: <Vm::Verifier as ZkVerifier>::CodeCommitment,
) -> (GenesisConfig<BenchSpec<Vm>>, Roles<BenchSpec<Vm>>)
where
    <Vm::Verifier as ZkVerifier>::CryptoSpec: CryptoSpecExt,
{
    let (genesis_config, roles) = setup(num_senders as usize, inner_code_commitment);

    (genesis_config, roles)
}
