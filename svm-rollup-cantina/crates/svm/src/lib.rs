use std::{
    collections::BTreeSet,
    sync::{Arc, RwLock},
};

use anyhow::anyhow;
use solana_accounts_db::blockhash_queue::BlockhashQueue;
use solana_compute_budget::compute_budget::ComputeBudget;
use solana_sdk::{
    account::Account,
    clock::{Slot, UnixTimestamp},
    epoch_info::EpochInfo,
    feature_set::FeatureSet,
    fee::FeeStructure,
    pubkey::Pubkey,
    signature::Signature,
};
use solana_svm::transaction_processor::TransactionBatchProcessor;
use sov_address::FromVmAddress;
use sov_modules_api::{
    AccessoryStateMap, AccessoryStateValue, AccessoryStateVec, Context, DaSpec, Error,
    EventEmitter, GenesisState, Module, ModuleId, ModuleInfo, Spec, StateAccessor, StateMap,
    StateValue, StateVec, TxState,
};
use sov_state::BcsCodec;
use svm_engine::{
    storage::{SVMStorage, SvmBankForks},
    Engine, LoadAndExecuteTransactionsConfig,
};
use svm_types::SolanaAddress;
use wrappers::ExecutedTransactionData;

pub mod authentication;
pub mod call;
pub mod chain_hash;
pub mod errors;
pub mod event;
mod genesis;
mod hooks;
#[cfg(feature = "native")]
mod rpc;
pub mod state_helpers;
pub mod svm;
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;
pub mod wrappers;

#[cfg(feature = "native")]
pub use crate::rpc::*;
pub use crate::{
    event::Event,
    genesis::InitSvmConfig,
    wrappers::{ConfirmedBlockDetails, ConfirmedTransactionDetails},
};

/// A new module:
/// - Must derive `ModuleInfo`
/// - Must contain `[address]` field
///
/// The SVM module allows the Sovereign SDK to process SVM transaction batches
#[derive(ModuleInfo, Clone)]
pub struct SVM<S: Spec>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    /// Id of the module.
    #[id]
    pub id: ModuleId,

    /// All accounts stored in the module
    #[state]
    pub(crate) accounts: StateMap<Pubkey, Account, BcsCodec>,

    /// A reference to the Bank module.
    #[module]
    pub(crate) bank_module: sov_bank::Bank<S>,

    /// Mapping from program accounts to accounts, and user account to token accounts.
    #[state]
    pub owner_index: StateMap<Pubkey, Vec<Pubkey>, BcsCodec>,

    #[state]
    pub creation_time: StateValue<UnixTimestamp, BcsCodec>,

    /// Queue for blockhash items
    #[state]
    pub blockhash_queue: StateValue<BlockhashQueue, BcsCodec>,

    /// The compute budget to use for transaction execution.
    #[state]
    pub compute_budget: StateValue<ComputeBudget, BcsCodec>,

    /// Information about the current epoch
    #[state]
    pub epoch_info: StateValue<EpochInfo, BcsCodec>,

    /// Track cluster signature throughput and adjust fee rate
    #[state]
    pub fee_rate_governor: StateValue<wrappers::FeeRateGovernorWrapper, BcsCodec>,

    /// Fee structure to use for assessing transaction fees.
    #[state]
    pub fee_structure: StateValue<FeeStructure, BcsCodec>,

    /// Track set of currently active features
    #[state]
    pub feature_set: StateValue<FeatureSet, BcsCodec>,

    /// Set of reserved account keys that cannot be write locked
    #[state]
    pub reserved_account_keys: StateValue<BTreeSet<Pubkey>, BcsCodec>,

    /// The Pubkey to which to send the collected fees.
    #[state]
    pub collector_id: StateValue<Pubkey, BcsCodec>,

    /// A vector of processed transactions which is appended after each succesfull SVM
    /// transaction execution.
    #[state]
    pub processed_transactions: StateVec<ExecutedTransactionData, BcsCodec>,

    /// Processed transactions used for RPC only.
    #[state]
    pub confirmed_transactions: AccessoryStateMap<Signature, ConfirmedTransactionDetails, BcsCodec>,

    /// Count of executed transactions for RPC only.
    #[state]
    pub transactions_count: AccessoryStateValue<u64, BcsCodec>,

    /// Processed blocks used for RPC only.
    #[state]
    pub confirmed_blocks: AccessoryStateMap<Slot, ConfirmedBlockDetails, BcsCodec>,

    /// Last confirmed block for RPC only.
    #[state]
    pub confirmed_block_slots: AccessoryStateVec<Slot, BcsCodec>,

    /// Genesis hash.
    #[state]
    pub genesis_hash: AccessoryStateValue<String, BcsCodec>,

    #[phantom]
    phantom: core::marker::PhantomData<S>,
}

impl<S: Spec> Module for SVM<S>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    type Spec = S;

    type Config = InitSvmConfig;

    type CallMessage = Vec<u8>;

    type Event = Event;

    // The genesis function gets called once at the initialization of the rollup
    fn genesis(
        &mut self,
        _genesis_rollup_header: &<<S as Spec>::Da as DaSpec>::BlockHeader,
        config: &Self::Config,
        state: &mut impl GenesisState<S>,
    ) -> Result<(), Error> {
        self.init_module(config, state)
            .map_err(|e| anyhow!("Failed to initialize module in genesis: {e}"))?;
        Ok(())
    }

    fn call(
        &mut self,
        msg: Self::CallMessage,
        context: &Context<Self::Spec>,
        state: &mut impl TxState<S>,
    ) -> Result<(), Error> {
        let mut unmetered_state = state.to_unmetered();

        self.execute_call(msg, context, &mut unmetered_state)?;

        self.emit_event(state, Event::ExecuteTransactions);

        Ok(())
    }
}

pub(crate) struct SvmEngine<'a, Storage: SVMStorage> {
    pub(crate) inner: Engine<'a, Storage>,
}

impl<'a, S: SVMStorage> SvmEngine<'a, S> {
    pub fn new(
        db: S,
        processor: &'a TransactionBatchProcessor<SvmBankForks>,
        config: LoadAndExecuteTransactionsConfig,
        fork_graph: Arc<RwLock<SvmBankForks>>,
    ) -> Self {
        let programs = if processor
            .builtin_program_ids
            .read()
            .expect("Program ids for processor are poisoned")
            .is_empty()
        {
            db.get_program_accounts() // processor is newly created and needs programs
        } else {
            Vec::new() // processor is from the Sovereign cache
        };

        let engine = Engine::builder()
            .db(db)
            .processor(processor)
            .config(config)
            .fork_graph(fork_graph)
            .build();

        engine.initialize_cache();
        engine.initialize_transaction_processor();
        engine.fill_cache(&programs);

        Self { inner: engine }
    }
}
