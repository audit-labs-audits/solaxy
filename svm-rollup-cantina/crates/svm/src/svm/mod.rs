mod storage;
pub(crate) mod transaction_processing_callback;

use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use solana_sdk::{account::Account, pubkey::Pubkey};
use sov_address::FromVmAddress;
use sov_bank::Amount;
use sov_modules_api::{Spec, StateMap, StateReaderAndWriter};
use sov_state::{BcsCodec, User};
use svm_engine::{storage::SVMStorageError, SVMEngineResult};
use svm_types::{to_rollup_address, SolanaAddress};

use crate::SVM;

pub struct SvmCallback<'a, S: Spec, T: StateReaderAndWriter<User>>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    accounts: Arc<RwLock<&'a mut StateMap<Pubkey, Account, BcsCodec>>>,
    owner_index: Arc<RwLock<&'a mut StateMap<Pubkey, Vec<Pubkey>, BcsCodec>>>,
    working_set: Arc<RwLock<&'a T>>,
    bank_module: Arc<RwLock<&'a mut sov_bank::Bank<S>>>,
    account_cache: Arc<RwLock<BTreeMap<Pubkey, Option<Account>>>>,
    phantom: core::marker::PhantomData<S>,
}

impl<'a, S: Spec, T: StateReaderAndWriter<User>> SvmCallback<'a, S, T>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    pub fn new(svm: &'a mut SVM<S>, working_set: Arc<RwLock<&'a T>>) -> Self {
        SvmCallback {
            accounts: Arc::new(RwLock::new(&mut svm.accounts)),
            owner_index: Arc::new(RwLock::new(&mut svm.owner_index)),
            working_set,
            bank_module: Arc::new(RwLock::new(&mut svm.bank_module)),
            account_cache: Arc::new(RwLock::new(Default::default())),
            phantom: Default::default(),
        }
    }

    /// Cache account data to avoid expensive calls to Sovereign code. Currently, this is reset per
    /// tx, but it's still helpful. If and when Sovereign adds the ability to store an SvmEngine
    /// for the length of a block, this cache will have much larger impact. This is used as a
    /// write-through cache, so the Sovereign database is always up to date.
    fn account_cache_add(&self, key: &Pubkey, account: Option<&Account>) {
        self.account_cache
            .write()
            .expect("Account cache poisoned; could not add element")
            .insert(*key, account.cloned());
    }

    fn account_cache_remove(&self, key: &Pubkey) {
        let _ = self
            .account_cache
            .write()
            .expect("Account cache poisoned; could not remove")
            .remove(key);
    }

    /// None = This was a cache miss.
    /// Some(None) = This was a cache hit, and the backing database doesn't have an entry at all.
    /// Some(Some(Account)) = This was a cache hit, and the database has Account in it.
    ///
    /// Returning Account (rather than &Account) can take a significant amount of time when there's data.
    fn account_cache_get(&self, key: &Pubkey) -> Option<Option<Account>> {
        self.account_cache
            .read()
            .expect("Account cache poisoned; could not get element")
            .get(key)
            .cloned()
    }

    /// This is a helper function to get the balance of an account from the bank module.
    fn get_bank_balance(&self, pubkey: &Pubkey) -> SVMEngineResult<u64> {
        let working_set_mut = unsafe {
            crate::svm::storage::as_mut(
                *self
                    .working_set
                    .write()
                    .expect("working_set poisoned, could not write"),
            )
        };
        let Amount(balance) = self
            .bank_module
            .read()
            .expect("Bank module poisoned; could not get balance")
            .get_balance_of(
                &to_rollup_address::<S>(*pubkey),
                sov_bank::config_gas_token_id(),
                working_set_mut,
            )
            .map_err(|e| {
                SVMStorageError::Read(format!("Failed to get balance of {pubkey} from bank: {e}"))
            })?
            .unwrap_or_default();

        Ok(balance.try_into().expect("balance to not overflow u64"))
    }

    /// This is a helper function to set the balance of an account in the bank module.
    fn set_bank_balance(&self, pubkey: &Pubkey, balance: u64) -> SVMEngineResult {
        let working_set_mut = unsafe {
            crate::svm::storage::as_mut(
                *self
                    .working_set
                    .write()
                    .expect("working_set poisoned, could not write"),
            )
        };
        self.bank_module
            .write()
            .expect("Bank module poisoned; could not set balance")
            .override_gas_balance(
                balance.into(),
                &to_rollup_address::<S>(*pubkey),
                working_set_mut,
            )
            .map_err(|e| {
                SVMStorageError::Write(format!(
                    "Failed to set gas balance of {pubkey} in bank: {e}"
                ))
            })?;

        Ok(())
    }
}
