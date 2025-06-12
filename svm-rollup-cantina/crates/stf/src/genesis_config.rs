//! While the `GenesisConfig` type for `Rollup` is generated from the underlying runtime through a macro,
//! specific module configurations are obtained from files. This code is responsible for the logic
//! that transforms module genesis data into Rollup genesis data.

use std::{
    convert::AsRef,
    path::{Path, PathBuf},
};

use serde::de::DeserializeOwned;
pub use sov_accounts::{AccountConfig, AccountData};
use sov_address::FromVmAddress;
use sov_attester_incentives::AttesterIncentivesConfig;
use sov_bank::BankConfig;
use sov_chain_state::ChainStateConfig;
use sov_modules_api::prelude::*;
use sov_modules_stf_blueprint::Runtime as RuntimeTrait;
use sov_prover_incentives::ProverIncentivesConfig;
use sov_sequencer_registry::SequencerConfig;
pub use svm::InitSvmConfig;
use svm_types::SolanaAddress;

/// Creates config for a rollup with some default settings, the config is used in demos and tests.
use crate::runtime::{GenesisConfig, Runtime};

/// Paths pointing to genesis files.
#[derive(Clone, Debug)]
pub struct GenesisPaths {
    /// Bank genesis path.
    pub bank_genesis_path: PathBuf,
    /// Sequencer Registry genesis path.
    pub sequencer_genesis_path: PathBuf,
    /// Accounts genesis path.
    pub accounts_genesis_path: PathBuf,
    /// Attester Incentives genesis path.
    pub attester_incentives_genesis_path: PathBuf,
    /// Prover Incentives genesis path.
    pub prover_incentives_genesis_path: PathBuf,
    /// Chain State genesis path.
    pub chain_state_genesis_path: PathBuf,
    /// SVM genesis path.
    pub svm_genesis_path: PathBuf,
}

impl GenesisPaths {
    /// Creates a new [`GenesisPaths`] from the files contained in the given
    /// directory.
    ///
    /// Take a look at the contents of the `test-data` directory to see the
    /// expected files.
    pub fn from_dir(dir: impl AsRef<Path>) -> Self {
        Self {
            bank_genesis_path: dir.as_ref().join("bank.json"),
            sequencer_genesis_path: dir.as_ref().join("sequencer_registry.json"),
            accounts_genesis_path: dir.as_ref().join("accounts.json"),
            prover_incentives_genesis_path: dir.as_ref().join("prover_incentives.json"),
            attester_incentives_genesis_path: dir.as_ref().join("attester_incentives.json"),
            chain_state_genesis_path: dir.as_ref().join("chain_state_zk.json"),
            svm_genesis_path: dir.as_ref().join("svm.json"),
        }
    }
}

/// Creates a new [`RuntimeTrait::GenesisConfig`] from the files contained in
/// the given directory.
pub fn create_genesis_config<S: Spec>(
    genesis_paths: &GenesisPaths,
) -> anyhow::Result<<Runtime<S> as RuntimeTrait<S>>::GenesisConfig>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    let bank_config: BankConfig<S> = read_genesis_json(&genesis_paths.bank_genesis_path)?;

    let sequencer_registry_config: SequencerConfig<S> =
        read_genesis_json(&genesis_paths.sequencer_genesis_path)?;

    let attester_incentives_config: AttesterIncentivesConfig<S> =
        read_genesis_json(&genesis_paths.attester_incentives_genesis_path)?;

    let prover_incentives_config: ProverIncentivesConfig<S> =
        read_genesis_json(&genesis_paths.prover_incentives_genesis_path)?;

    let accounts_config: AccountConfig<S> =
        read_genesis_json(&genesis_paths.accounts_genesis_path)?;

    let chain_state_config: ChainStateConfig<S> =
        read_genesis_json(&genesis_paths.chain_state_genesis_path)?;

    let blob_storage_config = ();

    let svm_config: InitSvmConfig = read_genesis_json(&genesis_paths.svm_genesis_path)?;

    Ok(GenesisConfig::new(
        bank_config,
        sequencer_registry_config,
        attester_incentives_config,
        prover_incentives_config,
        accounts_config,
        chain_state_config,
        blob_storage_config,
        svm_config,
    ))
}

fn read_genesis_json<T: DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
    let contents = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}
