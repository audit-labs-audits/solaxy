//! Sovereign Module call implementation for Module SVM.

use std::{
    collections::HashSet,
    default::Default,
    mem::take,
    sync::{Arc, Mutex, RwLock},
};

use anyhow::anyhow;
use solana_program::{
    clock::{Epoch, Slot, MAX_PROCESSING_AGE},
    fee_calculator::FeeRateGovernor,
};
use solana_sdk::{
    pubkey::Pubkey,
    rent_collector::RentCollector,
    transaction::{self, SanitizedTransaction, Transaction, TransactionError},
};
use solana_svm::{
    account_loader::{CheckedTransactionDetails, TransactionCheckResult},
    transaction_processor::TransactionBatchProcessor,
};
use sov_address::FromVmAddress;
use sov_modules_api::{
    BorshSerializedSize, Context, Spec, StateReaderAndWriter, TxState, UnmeteredStateWrapper,
};
use sov_state::User;
use svm_engine::{
    storage::SvmBankForks, verification::sanitize_and_verify_tx, LoadAndExecuteTransactionsConfig,
};
use svm_types::SolanaAddress;

use crate::{
    errors::{SVMRollupResult, SVMStateError},
    svm::SvmCallback,
    wrappers::ExecutedTransactionData,
    SvmEngine, SVM,
};

#[derive(Default)]
struct CachedEngineData {
    tbp: TransactionBatchProcessor<SvmBankForks>,
    fg: Arc<RwLock<SvmBankForks>>,
}

struct MutableCachedEngineData(Mutex<Option<CachedEngineData>>);

// This implements a required trait for a  future feature by Sovereign that evicts things from the
// cache if it gets too full. It's a required trait, but also there's no plausible way for us to
// have a real number there (because we're caching Agave structures which certainly don't implement
// BorshSerializedSize). We can't compute the actual size easily without actually serializing.
//
// The value of 2_000_000 was taken from having approximately 1MB of programs in the initial cache
// at the time of implementation, with the assumption being that this would be vaguely correct
// even after a few more programs are added to the rollup. If and when this is actually used
// by Sovereign, we can attempt a better realtime estimate by totalling the sizes of all loaded
// programs, but because the internal data structures are controlled by Agave, we can't get an
// exact value.
impl BorshSerializedSize for MutableCachedEngineData {
    fn serialized_size(&self) -> usize {
        2_000_000 // Somewhat arbitrary value (see comment, above)
    }
}

impl<S: Spec> SVM<S>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    fn take_engine_data_from_cache(
        &self,
        unmetered: &mut UnmeteredStateWrapper<'_, impl TxState<S>>,
        slot: Slot,
        epoch: Epoch,
    ) -> CachedEngineData {
        // Get the TransactionBatchProcessor from the Sovereign cache, if it's present (which it will
        // not be at the start of a new block, or if Sovereign decides the cache is too full and flushes
        // it). Otherwise, create a new one.
        let maybe_tbp = unmetered.inner().get_cached::<MutableCachedEngineData>();
        let tbp = if let Some(tbp) = maybe_tbp {
            tbp
        } else {
            unmetered
                .inner_mut()
                .put_cached(MutableCachedEngineData(Mutex::new(Some(
                    CachedEngineData {
                        tbp: TransactionBatchProcessor::<SvmBankForks>::new(
                            slot,
                            epoch,
                            HashSet::default(),
                        ),
                        fg: Arc::new(RwLock::new(SvmBankForks::default())),
                    },
                ))));
            unmetered
                .inner()
                .get_cached::<MutableCachedEngineData>()
                .unwrap()
        };

        // Take ownership of the internal value of the TBP structure in the Sovereign cache, so
        // we can pass a reference to the engine. It does look like cheating to take a value out
        // of the cached structure like this, but this usage comes from Preston at Sovereign.
        take(
            tbp.0
                .lock()
                .expect("Could not lock cached TBP")
                .as_mut()
                .expect("Sovereign cache was storing an empty CachedEngineData Option"),
        )
    }

    // Restore the value we took from within the cache
    fn return_engine_data_to_cache(
        &self,
        unmetered: &mut UnmeteredStateWrapper<'_, impl TxState<S>>,
        cached_engine_data: CachedEngineData,
    ) {
        let tbp = unmetered
            .inner()
            .get_cached::<MutableCachedEngineData>()
            .expect("It should already be in cache");
        let _ = tbp
            .0
            .lock()
            .expect("Could not lock cached TBP")
            .insert(cached_engine_data);
    }

    pub(crate) fn execute_call(
        &mut self,
        msg: Vec<u8>,
        _context: &Context<S>,
        unmetered: &mut UnmeteredStateWrapper<'_, impl TxState<S>>,
    ) -> Result<(), sov_modules_api::Error> {
        let reserved_account_keys: HashSet<Pubkey> = self
            .reserved_account_keys
            .get(unmetered)
            .map_err(|e| anyhow!("Failed to retrieve reserved account keys: {e}"))?
            .ok_or_else(|| anyhow!("Reserved account keys were not initialized"))?
            .into_iter()
            .collect();

        let transaction: Transaction = borsh::from_slice(&msg)
            .expect("transactions to have been checked to be deserializable");

        let sanitized_transaction: SanitizedTransaction =
            sanitize_and_verify_tx(&transaction, &reserved_account_keys)
                .map_err(|e| anyhow!("Failed to sanitize and verify transaction: {e:?}"))?;

        let sanitized_transactions = [sanitized_transaction];

        let (slot, epoch) = self
            .epoch_info
            .get(unmetered)
            .map_err(|e| anyhow!("Failed to retrieve epoch info: {e}"))?
            .ok_or_else(|| anyhow!("Epoch info was not initialized"))
            .map(|info| (info.absolute_slot, info.epoch))?;

        let config = self
            .get_load_and_execute_transactions_config(unmetered)
            .map_err(|e| anyhow!("Failed to retrieve load and execute transactions config: {e}"))?;

        let check_results =
            self.get_transaction_check_results(&sanitized_transactions, config.max_age, unmetered);

        let cached_engine_data = self.take_engine_data_from_cache(unmetered, slot, epoch);

        let engine = self.initialize_engine(
            config,
            &cached_engine_data.tbp,
            cached_engine_data.fg.clone(),
            unmetered,
        );

        let execution_details = engine
            .inner
            .load_execute_and_commit_transactions(&sanitized_transactions, check_results)
            .map_err(|e| anyhow!("Failed to load execute and commit transactions: {e}"))?;

        let transaction_execution_result = execution_details
            .first()
            .ok_or_else(|| anyhow!("Failed to retrieve transaction execution result"))?;

        self.processed_transactions
            .push(
                &ExecutedTransactionData::new(transaction, transaction_execution_result.clone()),
                unmetered,
            )
            .map_err(|e| anyhow!("Failed to push transaction to processed transactions: {e}"))?;

        self.return_engine_data_to_cache(unmetered, cached_engine_data);

        Ok(())
    }

    pub(crate) fn initialize_engine<'a, T>(
        &'a mut self,
        config: LoadAndExecuteTransactionsConfig,
        transaction_processor: &'a TransactionBatchProcessor<SvmBankForks>,
        fork_graph: Arc<RwLock<SvmBankForks>>,
        working_set: &'a T,
    ) -> SvmEngine<'a, SvmCallback<'a, S, T>>
    where
        T: StateReaderAndWriter<User>,
    {
        let working_set = Arc::new(RwLock::new(working_set));
        let callback = SvmCallback::new(self, working_set.clone());

        SvmEngine::new(callback, transaction_processor, config, fork_graph)
    }

    #[cfg(feature = "native")]
    pub(crate) fn initialize_engine_for_simulation<'a, T>(
        clone: &'a mut Self,
        config: LoadAndExecuteTransactionsConfig,
        transaction_processor: &'a TransactionBatchProcessor<SvmBankForks>,
        working_set: &'a T,
    ) -> SvmEngine<'a, SvmCallback<'a, S, T>>
    where
        T: StateReaderAndWriter<User>,
    {
        let working_set = Arc::new(RwLock::new(working_set));
        let callback = SvmCallback::new(clone, working_set.clone());
        let fork_graph = Arc::new(RwLock::new(SvmBankForks::default()));

        SvmEngine::new(callback, transaction_processor, config, fork_graph)
    }

    pub(crate) fn get_load_and_execute_transactions_config(
        &self,
        working_set: &mut impl StateReaderAndWriter<User>,
    ) -> SVMRollupResult<LoadAndExecuteTransactionsConfig> {
        let feature_set = self
            .feature_set
            .get(working_set)
            .map_err(|e| SVMStateError::Read(format!("Failed to retrieve feature set: {e}")))?
            .ok_or_else(|| SVMStateError::Read("Feature set was not initialized".to_string()))?;

        let blockhash = self
            .blockhash_queue
            .get(working_set)
            .map_err(|e| SVMStateError::Read(format!("Failed to retrieve blockhash queue: {e}")))?
            .ok_or_else(|| SVMStateError::Read("Blockhash queue was not initialized".to_string()))?
            .last_hash();

        let fee_structure = self
            .fee_structure
            .get(working_set)
            .map_err(|e| SVMStateError::Read(format!("Failed to retrieve fee structure: {e}")))?
            .ok_or_else(|| SVMStateError::Read("Fee structure was not initialized".to_string()))?;

        let fee_rate_governor: FeeRateGovernor = self
            .fee_rate_governor
            .get(working_set)
            .map_err(|e| SVMStateError::Read(format!("Failed to retrieve fee rate governor: {e}")))?
            .ok_or_else(|| {
                SVMStateError::Read("Fee rate governor was not initialized".to_string())
            })?
            .into();

        let rent_sysvar = self
            .accounts
            .get(&solana_sdk::sysvar::rent::id(), working_set)
            .map_err(|e| {
                SVMStateError::Read(format!("Failed to retrieve rent sysvar account: {e}"))
            })?
            .ok_or_else(|| {
                SVMStateError::Read("Rent sysvar account was not initialized".to_string())
            })?;
        let rent = bincode::deserialize(rent_sysvar.data.as_slice()).map_err(|e| {
            SVMStateError::Read(format!("Failed to retrieve rent system variable: {e}"))
        })?;
        let rent_collector = RentCollector {
            rent,
            ..RentCollector::default()
        };

        let compute_budget = self
            .compute_budget
            .get(working_set)
            .map_err(|e| SVMStateError::Read(format!("Failed to retrieve compute budget: {e}")))?
            .ok_or_else(|| SVMStateError::Read("Compute budget was not initialized".to_string()))?;

        Ok(LoadAndExecuteTransactionsConfig {
            blockhash,
            fee_rate_governor,
            fee_structure,
            rent_collector,
            max_age: MAX_PROCESSING_AGE,
            vote_accounts: None,
            total_stake: None,
            compute_budget,
            feature_set,
        })
    }

    pub(crate) fn get_transaction_check_results(
        &self,
        sanitized_tx: &[SanitizedTransaction],
        max_age: usize,
        working_set: &mut impl StateReaderAndWriter<User>,
    ) -> Vec<transaction::Result<CheckedTransactionDetails>> {
        sanitized_tx
            .iter()
            .map(|tx| self.check_age(tx, max_age, working_set))
            .collect()
    }

    fn check_age(
        &self,
        sanitized_tx: &SanitizedTransaction,
        max_age: usize,
        working_set: &mut impl StateReaderAndWriter<User>,
    ) -> TransactionCheckResult {
        let blockhash = sanitized_tx.message().recent_blockhash();
        let Ok(Some(hash_queue)) = self.blockhash_queue.get(working_set) else {
            return Err(TransactionError::BlockhashNotFound);
        };

        let Some(lamports_per_signature) = hash_queue.get_lamports_per_signature(blockhash) else {
            return Err(TransactionError::BlockhashNotFound);
        };

        if hash_queue.is_hash_valid_for_age(blockhash, max_age) {
            Ok(CheckedTransactionDetails {
                nonce: None,
                lamports_per_signature,
            })
        } else {
            Err(TransactionError::BlockhashNotFound)
        }
    }
}
