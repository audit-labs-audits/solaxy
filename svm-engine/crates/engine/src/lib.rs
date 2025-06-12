use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock},
};

use bon::Builder;
use solana_compute_budget::compute_budget::ComputeBudget;
use solana_sdk::{
    clock::{MAX_PROCESSING_AGE, Slot},
    feature_set::FeatureSet,
    fee::FeeStructure,
    fee_calculator::FeeRateGovernor,
    hash::Hash,
    rent_collector::RentCollector,
};
use solana_svm::transaction_processor::{
    ExecutionRecordingConfig, TransactionBatchProcessor, TransactionProcessingConfig,
    TransactionProcessingEnvironment,
};
use solana_vote::vote_account::VoteAccounts;

pub mod builtins;
pub mod execution;
pub mod processor;
pub mod programs;
pub mod storage;
#[cfg(test)]
pub mod tests;
pub mod verification;

#[derive(Debug, Clone, PartialEq)]
pub struct LoadAndExecuteTransactionsConfig {
    pub blockhash: Hash,
    pub fee_rate_governor: FeeRateGovernor,
    pub fee_structure: FeeStructure,
    pub rent_collector: RentCollector,
    pub max_age: usize,
    pub vote_accounts: Option<VoteAccounts>,
    pub total_stake: Option<u64>,
    pub compute_budget: ComputeBudget,
    pub feature_set: FeatureSet,
}

impl Default for LoadAndExecuteTransactionsConfig {
    fn default() -> Self {
        Self {
            blockhash: Hash::default(),
            fee_rate_governor: FeeRateGovernor::default(),
            fee_structure: FeeStructure::default(),
            rent_collector: RentCollector::default(),
            max_age: MAX_PROCESSING_AGE,
            vote_accounts: None,
            total_stake: None,
            compute_budget: ComputeBudget::default(),
            feature_set: FeatureSet::all_enabled().clone(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SVMEngineError {
    #[error("Engine storage error: {0}")]
    Storage(#[from] storage::SVMStorageError),

    #[error("Engine execution error: {0}")]
    Execution(#[from] execution::SVMExecutionError),
}

pub type SVMEngineResult<T = ()> = Result<T, SVMEngineError>;

#[derive(Builder)]
pub struct Engine<'a, Storage>
where
    Storage: storage::SVMStorage,
{
    db: Storage,

    processor: &'a TransactionBatchProcessor<storage::SvmBankForks>,

    // A bit counterintuitive, but we need to keep a strong reference otherwise it'll get dropped
    // TBP's program cache only holds a weak reference to bank forks
    #[builder(default)]
    fork_graph: Arc<RwLock<storage::SvmBankForks>>,

    #[builder(default)]
    config: LoadAndExecuteTransactionsConfig,
}

impl<Storage> Engine<'_, Storage>
where
    Storage: storage::SVMStorage,
{
    pub fn get_storage(&self) -> &Storage {
        &self.db
    }

    pub fn slot(&self) -> Slot {
        self.fork_graph
            .read()
            .expect("Fork graph is poisoned")
            .highest_slot
    }

    pub fn transaction_config_and_environment(
        &self,
    ) -> (
        TransactionProcessingConfig,
        TransactionProcessingEnvironment,
    ) {
        let epoch_vote_accounts = self
            .config
            .vote_accounts
            .as_ref()
            .map(|vote_accounts| vote_accounts.as_ref());

        let processing_environment = TransactionProcessingEnvironment {
            epoch_total_stake: self.config.total_stake,
            epoch_vote_accounts,
            feature_set: Arc::new(self.config.feature_set.clone()),
            lamports_per_signature: self.config.fee_rate_governor.lamports_per_signature,
            rent_collector: Some(&self.config.rent_collector),
            fee_structure: Some(&self.config.fee_structure),
            blockhash: self.config.blockhash,
        };

        let processing_config = TransactionProcessingConfig {
            compute_budget: Some(self.config.compute_budget),
            recording_config: ExecutionRecordingConfig::new_single_setting(true),
            ..Default::default()
        };

        (processing_config, processing_environment)
    }
}

impl<Storage> Deref for Engine<'_, Storage>
where
    Storage: storage::SVMStorage,
{
    type Target = Storage;

    fn deref(&self) -> &Storage {
        &self.db
    }
}

impl<Storage> DerefMut for Engine<'_, Storage>
where
    Storage: storage::SVMStorage,
{
    fn deref_mut(&mut self) -> &mut Storage {
        &mut self.db
    }
}
