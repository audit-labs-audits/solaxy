use solana_sdk::{account::AccountSharedData, pubkey::Pubkey};
use solana_svm::transaction_processing_callback::TransactionProcessingCallback;
use sov_address::FromVmAddress;
use sov_modules_api::{Spec, StateReaderAndWriter};
use sov_state::User;
use svm_engine::storage::SVMStorage;
use svm_types::SolanaAddress;

use super::SvmCallback;

impl<S: Spec, T: StateReaderAndWriter<User>> TransactionProcessingCallback for SvmCallback<'_, S, T>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    fn account_matches_owners(&self, account: &Pubkey, owners: &[Pubkey]) -> Option<usize> {
        self.is_account_owned_by(account, owners)
    }

    fn get_account_shared_data(&self, pubkey: &Pubkey) -> Option<AccountSharedData> {
        self.account_shared_data(pubkey)
    }

    fn add_builtin_account(&self, name: &str, program_id: &Pubkey) {
        self.add_builtin_program(name, program_id)
            .unwrap_or_else(|e| {
                panic!("Failed to add builtin program {name} with ID {program_id}: {e}")
            });
    }
}
