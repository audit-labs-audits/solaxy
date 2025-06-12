use std::sync::{Arc, RwLockWriteGuard};

use solana_bpf_loader_program::syscalls::create_program_runtime_environment_v1;
use solana_program::{
    bpf_loader,
    bpf_loader_upgradeable::{self, UpgradeableLoaderState},
    clock::Clock,
    epoch_rewards::EpochRewards,
    epoch_schedule::EpochSchedule,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::{self, clock, epoch_rewards, epoch_schedule, rent},
};
use solana_program_runtime::loaded_programs::{
    LoadProgramMetrics, ProgramCache, ProgramCacheEntry,
};
use solana_sdk::account::{Account, AccountSharedData, ReadableAccount};

use crate::{
    Engine, SVMEngineResult,
    builtins::{BUILTINS, Builtin},
    storage::{SVMStorageError, SvmBankForks},
};

impl<Storage> Engine<'_, Storage>
where
    Storage: crate::storage::SVMStorage,
{
    fn initialize_sysvars(&self) -> eyre::Result<()> {
        let new_accounts = [
            // Rent
            (rent::id(), bincode::serialize(&Rent::default())?),
            // Clock
            (clock::id(), bincode::serialize(&Clock::default())?),
            // Epoch Schedule
            (
                epoch_schedule::id(),
                bincode::serialize(&EpochSchedule::default())?,
            ),
            // Epoch Rewards
            (
                epoch_rewards::id(),
                bincode::serialize(&EpochRewards::default())?,
            ),
        ];

        for (id, data) in new_accounts {
            let _ = self.set_account(
                &id,
                Account {
                    lamports: Rent::default().minimum_balance(data.len()),
                    data,
                    owner: sysvar::id(),
                    executable: false,
                    rent_epoch: 0,
                },
            );
        }

        Ok(())
    }

    pub fn fill_cache(&self, program_ids: &[Pubkey]) {
        let mut cache = self.processor.program_cache.write().unwrap();
        for program_id in program_ids {
            self.load_program_to_cache(program_id, &mut cache, true);
        }
    }

    fn create_new_program_cache_entry(
        program_id: &Pubkey,
        cache: &mut RwLockWriteGuard<'_, ProgramCache<SvmBankForks>>,
        loader_key: &Pubkey,
        elf_bytes: &[u8],
        account_size: usize,
        is_reload: bool,
    ) {
        // These two cases should be identical except for reload vs. new as the constructor,
        // which controls whether the loaded program is verified or not. Using reload on a
        // program that has been previously verified (e.g. because it is now being loaded from
        // a database) saves substantial processing time.

        let program_runtime_environment = cache.environments.program_runtime_v1.clone();

        if is_reload {
            cache.assign_program(
                *program_id,
                Arc::new(
                    unsafe {
                        ProgramCacheEntry::reload(
                            loader_key,
                            program_runtime_environment,
                            0,
                            0,
                            elf_bytes,
                            account_size,
                            &mut LoadProgramMetrics::default(),
                        )
                    }
                    .unwrap(),
                ),
            );
        } else {
            cache.assign_program(
                *program_id,
                Arc::new(
                    ProgramCacheEntry::new(
                        loader_key,
                        program_runtime_environment,
                        0,
                        0,
                        elf_bytes,
                        account_size,
                        &mut LoadProgramMetrics::default(),
                    )
                    .unwrap(),
                ),
            );
        }
    }

    fn load_bpf_loader_program_to_cache(
        &self,
        program_id: &Pubkey,
        program_account: AccountSharedData,
        cache: &mut RwLockWriteGuard<'_, ProgramCache<SvmBankForks>>,
        is_reload: bool,
    ) {
        let elf_bytes = program_account.data();

        Self::create_new_program_cache_entry(
            program_id,
            cache,
            &solana_sdk::bpf_loader::id(),
            elf_bytes,
            elf_bytes.len(),
            is_reload,
        );
    }

    fn load_bpf_loader_upgradeable_program_to_cache(
        &self,
        program_id: &Pubkey,
        program_account: AccountSharedData,
        cache: &mut RwLockWriteGuard<'_, ProgramCache<SvmBankForks>>,
        is_reload: bool,
    ) {
        let program_loader_state: UpgradeableLoaderState =
            bincode::deserialize(program_account.data()).unwrap();

        let UpgradeableLoaderState::Program {
            programdata_address,
        } = program_loader_state
        else {
            return;
        };

        let Some(programdata_account) = self.get_account_shared_data(&programdata_address) else {
            return;
        };

        let program_data = programdata_account.data();

        // Calculate the size of the metadata (ProgramData)
        let metadata_size = UpgradeableLoaderState::size_of_programdata_metadata();

        // Extract the program bytecode (skip the metadata portion)
        let program_bytecode = &program_data[metadata_size..];

        Self::create_new_program_cache_entry(
            program_id,
            cache,
            &bpf_loader_upgradeable::id(),
            program_bytecode,
            program_account
                .data()
                .len()
                .saturating_add(program_data.len()),
            is_reload,
        );
    }

    pub(crate) fn load_program_to_cache(
        &self,
        program_id: &Pubkey,
        cache: &mut RwLockWriteGuard<'_, ProgramCache<SvmBankForks>>,
        is_reload: bool,
    ) {
        let Some(program_account) = self.get_account_shared_data(program_id) else {
            return;
        };

        if bpf_loader::check_id(program_account.owner()) {
            self.load_bpf_loader_program_to_cache(program_id, program_account, cache, is_reload);
            return;
        }

        if bpf_loader_upgradeable::check_id(program_account.owner()) {
            self.load_bpf_loader_upgradeable_program_to_cache(
                program_id,
                program_account,
                cache,
                is_reload,
            );
        }
    }

    pub fn initialize_cache(&self) {
        let mut cache = self.processor.program_cache.write().unwrap();

        cache.fork_graph = Some(Arc::downgrade(&self.fork_graph.clone()));

        cache.environments.program_runtime_v1 = Arc::new(
            create_program_runtime_environment_v1(
                &self.config.feature_set,
                &self.config.compute_budget,
                false,
                false,
            )
            .unwrap(),
        );
    }

    pub fn initialize_transaction_processor(&self) {
        let _ = self.initialize_sysvars();

        // add built-in programs
        for Builtin {
            id,
            name,
            entrypoint,
        } in BUILTINS
        {
            self.processor.add_builtin(
                &self.db,
                *id,
                name,
                ProgramCacheEntry::new_builtin(0, name.len(), *entrypoint),
            );
        }

        self.processor.fill_missing_sysvar_cache_entries(&self.db);
    }

    pub fn update_slot(&self) -> SVMEngineResult {
        // Update clock slot (increment by one)
        let mut clock_account = self
            .db
            .get_account(&clock::id())
            .map_err(|e| SVMStorageError::Read(format!("Failed to retrieve clock account: {e}")))?
            .ok_or_else(|| SVMStorageError::Read("Clock account not found".to_string()))?;
        let mut clock_data: Clock = bincode::deserialize(&clock_account.data).map_err(|e| {
            SVMStorageError::Read(format!("Failed to deserialize clock account data: {e}"))
        })?;

        clock_data.slot += 1;

        // Set new clock account
        clock_account.data = bincode::serialize(&clock_data).map_err(|e| {
            SVMStorageError::Write(format!("Failed to serialize clock account data: {e}"))
        })?;
        self.set_account(&clock::id(), clock_account)?;

        // Update sysvar cache
        self.processor.reset_sysvar_cache();
        self.processor.fill_missing_sysvar_cache_entries(&self.db);

        Ok(())
    }
}
