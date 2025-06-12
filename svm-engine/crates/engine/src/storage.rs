use std::{
    cmp::Ordering,
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use solana_program::{
    clock::{Epoch, INITIAL_RENT_EPOCH, Slot},
    pubkey::Pubkey,
};
use solana_program_runtime::loaded_programs::{BlockRelation, ForkGraph};
use solana_sdk::{
    account::{Account, AccountSharedData},
    bpf_loader, bpf_loader_upgradeable, native_loader,
};
use solana_svm::transaction_processing_callback::TransactionProcessingCallback;

use crate::SVMEngineResult;

#[derive(Debug, thiserror::Error)]
pub enum SVMStorageError {
    #[error("Failed to read from storage: {0}")]
    /// Failed to read from storage: {0}
    Read(String),

    #[error("Failed to write to storage: {0}")]
    /// Failed to write to storage: {0}
    Write(String),

    #[error("Unknown storage error: {0}")]
    /// Unknown storage error: {0}
    CatchAll(#[from] eyre::Error),
}

/// SVMStorage is a trait that allows the SVM engine to use various storage methods to hold account data
pub trait SVMStorage: TransactionProcessingCallback {
    /// Retrieve account data from storage
    fn get_account(&self, key: &Pubkey) -> SVMEngineResult<Option<Account>>;

    /// Persist a new account to storage
    fn set_account(&self, key: &Pubkey, account: Account) -> SVMEngineResult;

    /// Remove an account from storage
    fn remove_account(&self, key: &Pubkey) -> SVMEngineResult;

    /// Retrieve an owner index from storage
    fn get_owner_index(&self, owner: &Pubkey) -> SVMEngineResult<Option<Vec<Pubkey>>>;

    /// Update owner index in storage
    fn set_owner_index(&self, owner: &Pubkey, key: &Pubkey) -> SVMEngineResult;

    /// Retrieve all program accounts from storage
    fn get_program_accounts(&self) -> Vec<Pubkey> {
        let bpf_loader_accounts = self
            .get_owner_index(&bpf_loader::id())
            .unwrap_or_default()
            .unwrap_or_default();
        let bpf_loader_upgradeable_accounts = self
            .get_owner_index(&bpf_loader_upgradeable::id())
            .unwrap_or_default()
            .unwrap_or_default();

        bpf_loader_accounts
            .iter()
            .chain(bpf_loader_upgradeable_accounts.iter())
            .filter(|key| {
                if let Ok(Some(account)) = self.get_account(key) {
                    account.executable
                } else {
                    false
                }
            })
            .copied()
            .collect()
    }

    /// Create a `account_matches_owners` implementation to be used inside of the `TransactionProcessingCallback` trait.
    fn is_account_owned_by(&self, pubkey: &Pubkey, owners: &[Pubkey]) -> Option<usize> {
        let Ok(Some(account)) = self.get_account(pubkey) else {
            return None;
        };

        owners.iter().position(|entry| account.owner == *entry)
    }

    /// Get the account data for a given pubkey flattening into an option.
    fn account_shared_data(&self, pubkey: &Pubkey) -> Option<AccountSharedData> {
        self.get_account(pubkey)
            .ok()
            .flatten()
            .map(AccountSharedData::from)
    }

    /// Add a builtin program to the storage.
    /// Implementation is based on the [Agave bank](https://github.com/nitro-svm/agave/blob/d3b8cb0ce41269d7c2869b2589fa75d898e5575c/runtime/src/bank.rs#L1830-L1846) work.
    fn add_builtin_program(&self, name: &str, program_id: &Pubkey) -> SVMEngineResult {
        const RENT_UNADJUSTED_INITIAL_BALANCE: u64 = 1;

        if self.account_shared_data(program_id).is_some() {
            // The existing account is sufficient
            return Ok(());
        }

        // Add a bogus executable builtin account, which will be loaded and ignored.
        let account = native_loader::create_loadable_account_with_fields(
            name,
            (RENT_UNADJUSTED_INITIAL_BALANCE, INITIAL_RENT_EPOCH),
        );

        self.set_account(program_id, account.into())
    }
}

#[derive(Default)]
pub struct SvmBankForks {
    pub root: Slot,
    pub highest_slot: Slot,
}

pub fn from_ordering(ordering: Ordering) -> BlockRelation {
    match ordering {
        Ordering::Less => BlockRelation::Ancestor,
        Ordering::Equal => BlockRelation::Equal,
        Ordering::Greater => BlockRelation::Descendant,
    }
}

impl ForkGraph for SvmBankForks {
    fn relationship(&self, a: Slot, b: Slot) -> BlockRelation {
        let known_slot_range = self.root..=self.highest_slot;

        match (known_slot_range.contains(&a), known_slot_range.contains(&b)) {
            (true, true) => from_ordering(a.cmp(&b)),
            _ => BlockRelation::Unknown,
        }
    }

    fn slot_epoch(&self, _slot: Slot) -> Option<Epoch> {
        None
    }
}

pub struct AccountsDB {
    pub(crate) accounts: Arc<RwLock<BTreeMap<Pubkey, Account>>>,
    pub(crate) owner_index: Arc<RwLock<BTreeMap<Pubkey, Vec<Pubkey>>>>,
}

impl AccountsDB {
    pub fn new(accounts: BTreeMap<Pubkey, Account>) -> Self {
        // Initialize the owner index
        let mut owner_index: BTreeMap<Pubkey, Vec<Pubkey>> = BTreeMap::new();
        for (account_pubkey, account) in &accounts {
            owner_index
                .entry(account.owner)
                .or_default()
                .push(*account_pubkey);
        }

        let accounts = Arc::new(RwLock::new(accounts));
        let owner_index = Arc::new(RwLock::new(owner_index));

        Self {
            accounts,
            owner_index,
        }
    }

    pub fn get_accounts(&self) -> Arc<RwLock<BTreeMap<Pubkey, Account>>> {
        Arc::clone(&self.accounts)
    }
}

impl SVMStorage for AccountsDB {
    fn get_account(&self, key: &Pubkey) -> SVMEngineResult<Option<Account>> {
        let accounts = self.accounts.read().map_err(|e| {
            SVMStorageError::Read(format!("Failed to acquire accounts handle: {e}"))
        })?;

        Ok(accounts.get(key).cloned())
    }

    fn set_account(&self, key: &Pubkey, account: Account) -> SVMEngineResult {
        let mut accounts = self.accounts.write().map_err(|e| {
            SVMStorageError::Write(format!("Failed to acquire accounts handle: {e}"))
        })?;
        accounts.insert(*key, account);

        Ok(())
    }

    fn remove_account(&self, key: &Pubkey) -> SVMEngineResult {
        let mut accounts = self.accounts.write().map_err(|e| {
            SVMStorageError::Write(format!("Failed to acquire accounts handle: {e}"))
        })?;
        accounts.remove(key);

        Ok(())
    }

    fn get_owner_index(&self, owner: &Pubkey) -> SVMEngineResult<Option<Vec<Pubkey>>> {
        let owner_index = self.owner_index.read().map_err(|e| {
            SVMStorageError::Read(format!("Failed to acquire owner index handle {e}"))
        })?;

        Ok(owner_index.get(owner).cloned())
    }

    fn set_owner_index(&self, owner: &Pubkey, key: &Pubkey) -> SVMEngineResult {
        let mut owner_index = self.owner_index.write().map_err(|e| {
            SVMStorageError::Write(format!("Failed to acquire owner index handle {e}"))
        })?;
        let owned = owner_index.entry(*owner).or_default();
        if !owned.contains(key) {
            owned.push(*key);
        }
        Ok(())
    }
}

impl TransactionProcessingCallback for AccountsDB {
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
