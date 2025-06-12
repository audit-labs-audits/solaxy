use solana_sdk::{account::Account, pubkey::Pubkey};
use sov_address::FromVmAddress;
use sov_bank::Amount;
use sov_modules_api::{Spec, StateReaderAndWriter};
use sov_state::User;
use svm_types::{to_rollup_address, SolanaAddress};

use crate::{
    errors::{SVMRollupResult, SVMStateError},
    SVM,
};

impl<S: Spec> SVM<S>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    pub fn insert_account<T: StateReaderAndWriter<User>>(
        &mut self,
        pubkey: &Pubkey,
        account: &Account,
        working_set: &mut T,
    ) -> SVMRollupResult {
        self.set_bank_balance(pubkey, account.lamports, working_set)?;
        self.accounts
            .set(
                pubkey,
                &Account {
                    lamports: 0,
                    ..account.clone()
                },
                working_set,
            )
            .map_err(|e| SVMStateError::Write(format!("Failed to insert account {pubkey}: {e}")))?;
        Ok(())
    }

    pub(crate) fn update_owner_index(
        &mut self,
        owner: &Pubkey,
        accounts: &[Pubkey],
        working_set: &mut impl StateReaderAndWriter<User>,
    ) -> SVMRollupResult {
        let mut owned_accounts = self
            .owner_index
            .get(owner, working_set)
            .unwrap_or(None)
            .unwrap_or_default();
        for a in accounts {
            if !owned_accounts.contains(a) {
                owned_accounts.push(*a);
            }
        }
        self.owner_index
            .set(owner, &owned_accounts, working_set)
            .map_err(|e| {
                SVMStateError::Write(format!(
                    "Failed to update owner index map for owner with pubkey {owner}: {e}"
                ))
            })?;
        Ok(())
    }

    pub(crate) fn get_bank_balance(
        &self,
        pubkey: &Pubkey,
        working_set: &mut impl StateReaderAndWriter<User>,
    ) -> SVMRollupResult<u64> {
        let Amount(balance) = self
            .bank_module
            .get_balance_of(
                &to_rollup_address::<S>(*pubkey),
                sov_bank::config_gas_token_id(),
                working_set,
            )
            .map_err(|e| {
                SVMStateError::Read(format!("Failed to get balance of {pubkey} from bank: {e}"))
            })?
            .unwrap_or_default();

        Ok(balance.try_into().expect("balance to not overflow u64"))
    }

    pub(crate) fn set_bank_balance(
        &mut self,
        pubkey: &Pubkey,
        balance: u64,
        working_set: &mut impl StateReaderAndWriter<User>,
    ) -> SVMRollupResult {
        self.bank_module
            .override_gas_balance(
                balance.into(),
                &to_rollup_address::<S>(*pubkey),
                working_set,
            )
            .map_err(|e| {
                SVMStateError::Write(format!(
                    "Failed to set bank balance for {pubkey} to {balance}: {e}"
                ))
            })?;
        Ok(())
    }
}
