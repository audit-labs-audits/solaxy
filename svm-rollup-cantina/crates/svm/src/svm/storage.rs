use solana_program::pubkey::Pubkey;
use solana_sdk::account::Account;
use sov_address::FromVmAddress;
use sov_modules_api::{Spec, StateReaderAndWriter};
use sov_state::User;
use svm_engine::{
    storage::{SVMStorage, SVMStorageError},
    SVMEngineResult,
};
use svm_types::SolanaAddress;

use crate::svm::SvmCallback;

#[allow(invalid_reference_casting, clippy::mut_from_ref)]
pub unsafe fn as_mut<T>(r: &T) -> &mut T {
    let const_ptr = r as *const T;
    let mut_ptr = const_ptr as *mut T;
    &mut *mut_ptr
}

impl<S: Spec, T: StateReaderAndWriter<User>> SVMStorage for SvmCallback<'_, S, T>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    fn get_account(&self, key: &Pubkey) -> SVMEngineResult<Option<Account>> {
        if let Some(account) = self.account_cache_get(key) {
            return Ok(account);
        }

        let working_set_mut = unsafe {
            as_mut(
                *self
                    .working_set
                    .write()
                    .expect("working_set poisoned, could not write"),
            )
        };

        // If the account is not in the cache, fetch it from the storage and update the cache
        // accordingly.
        let mut account = self
            .accounts
            .read()
            .expect("Account cache poisoned, could not read value")
            .get(key, working_set_mut)
            .map_err(|e| SVMStorageError::Read(format!("Failed to get account {e}")))?;

        if let Some(account) = account.as_mut() {
            debug_assert!(
                account.lamports == 0,
                "SVM balance is not zero - balance should be stored in the bank module instead"
            );

            account.lamports = self.get_bank_balance(key)?;
        }

        self.account_cache_add(key, account.as_ref());

        Ok(account)
    }

    fn set_account(&self, key: &Pubkey, account: Account) -> SVMEngineResult {
        // SVM native token balance is stored in the bank module so we set the SVM account balance
        // to 0.
        self.set_bank_balance(key, account.lamports)?;
        let account = Account {
            lamports: 0,
            ..account
        };
        let working_set_mut = unsafe {
            as_mut(
                *self
                    .working_set
                    .write()
                    .expect("working_set poisoned, could not write"),
            )
        };
        self.accounts
            .write()
            .expect("Account cache poisoned, could not write for set")
            .set(key, &account, working_set_mut)
            .map_err(|e| SVMStorageError::Write(format!("Failed to set account {e}")))?;
        self.account_cache_add(key, Some(&account));
        Ok(())
    }

    fn get_owner_index(&self, owner: &Pubkey) -> SVMEngineResult<Option<Vec<Pubkey>>> {
        let working_set_mut = unsafe {
            as_mut(
                *self
                    .working_set
                    .write()
                    .expect("working_set poisoned, could not write"),
            )
        };
        let owner_index = self
            .owner_index
            .read()
            .expect("Owner index poisoned, could not read value")
            .get(owner, working_set_mut)
            .map_err(|e| SVMStorageError::Read(format!("Failed to get owner index {e}")))?;
        Ok(owner_index)
    }

    fn set_owner_index(&self, owner: &Pubkey, key: &Pubkey) -> SVMEngineResult {
        let working_set_mut = unsafe {
            as_mut(
                *self
                    .working_set
                    .write()
                    .expect("working_set poisoned, could not write"),
            )
        };
        let mut owned_accounts = self
            .owner_index
            .read()
            .expect("Owner index poisoned, could not read value for set")
            .get(owner, working_set_mut)
            .unwrap_or(None)
            .unwrap_or_default();
        if !owned_accounts.contains(key) {
            owned_accounts.push(*key);
            self.owner_index
                .write()
                .expect("Owner index poisoned, could not write value")
                .set(owner, &owned_accounts, working_set_mut)
                .map_err(|e| SVMStorageError::Write(format!("Failed to set owner index: {e}")))?;
        }
        Ok(())
    }

    fn remove_account(&self, key: &Pubkey) -> SVMEngineResult {
        self.account_cache_remove(key);
        let working_set_mut = unsafe {
            as_mut(
                *self
                    .working_set
                    .write()
                    .expect("working_set poisoned, could not write"),
            )
        };
        self.accounts
            .read()
            .expect("Account cache poisoned, could not remove account")
            .remove(key, working_set_mut)
            .map_err(|e| {
                SVMStorageError::Write(format!("Failed to remove {key} from accounts: {e}"))
            })?;
        self.set_bank_balance(key, 0)?;
        Ok(())
    }
}
