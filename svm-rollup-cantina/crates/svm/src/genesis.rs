use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use bincode::serialize;
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use solana_accounts_db::blockhash_queue::BlockhashQueue;
use solana_compute_budget::compute_budget::ComputeBudget;
use solana_program::{
    bpf_loader_upgradeable,
    bpf_loader_upgradeable::{get_program_data_address, UpgradeableLoaderState},
    clock::Clock,
    epoch_rewards::EpochRewards,
};
use solana_sdk::{
    account::{Account, AccountSharedData},
    bpf_loader,
    clock::{UnixTimestamp, DEFAULT_TICKS_PER_SLOT},
    epoch_info::EpochInfo,
    epoch_schedule::EpochSchedule,
    feature_set::FeatureSet,
    fee::FeeStructure,
    fee_calculator::{
        FeeRateGovernor, DEFAULT_TARGET_LAMPORTS_PER_SIGNATURE, DEFAULT_TARGET_SIGNATURES_PER_SLOT,
    },
    genesis_config::{ClusterType, DEFAULT_GENESIS_FILE},
    hash::{hash, Hash},
    inflation::Inflation,
    native_token::lamports_to_sol,
    poh_config::PohConfig,
    pubkey::Pubkey,
    rent::Rent,
    reserved_account_keys::ReservedAccountKeys,
    shred_version::compute_shred_version,
    sysvar::{self, clock, epoch_rewards, epoch_schedule, rent},
    timing::years_as_slots,
};
use sov_address::FromVmAddress;
use sov_modules_api::{GenesisState, Module, Spec};
use svm_types::SolanaAddress;

use crate::{
    errors::{SVMRollupResult, SVMStateError},
    wrappers, SVM,
};

pub struct SplProgram {
    pub program_id: Pubkey,
    pub owner_program: Pubkey,
    pub bytecode: &'static [u8],
}

/// List of SPL programs initialized during genesis.
/// This list must be kept in sync with any changes to the genesis configuration:
/// - When a new program is added during genesis, ensure it is included here.
/// - When a program is removed from genesis, it must also be removed from this list.
///
/// Failing to update this list accordingly can lead to incorrect program initialization in transaction processor and `SVM` state.
pub static SPL_PROGRAMS: &[SplProgram] = &[
    SplProgram {
        program_id: spl_token::id(),
        owner_program: bpf_loader::id(),
        bytecode: include_bytes!("../../test-data/spl-bytecode/spl_token.so"),
    },
    SplProgram {
        program_id: spl_associated_token_account::id(),
        owner_program: bpf_loader::id(),
        bytecode: include_bytes!("../../test-data/spl-bytecode/spl_associated_token_account.so"),
    },
    SplProgram {
        program_id: spl_token_2022::id(),
        owner_program: bpf_loader_upgradeable::id(),
        bytecode: include_bytes!("../../test-data/spl-bytecode/spl_token_2022.so"),
    },
];

// struct that is used to initialize the SVM module struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitSvmConfig {
    /// when the network (bootstrap validator) was started relative to the UNIX Epoch, i64
    pub creation_time: UnixTimestamp,
    /// initial accounts
    pub accounts: BTreeMap<svm_types::SolanaAddress, wrappers::AccountProxy>,
    /// faucet address
    pub faucet_keypair: Option<wrappers::KeypairProxy>,
    /// built-in programs
    pub native_instruction_processors: Vec<(String, svm_types::SolanaAddress)>,
    /// accounts for network rewards, these do not count towards capitalization
    pub rewards_pools: BTreeMap<Pubkey, Account>,
    pub ticks_per_slot: u64,
    /// network speed configuration
    pub poh_config: PohConfig,
    /// transaction fee config
    pub fee_rate_governor: FeeRateGovernor,
    /// rent config
    pub rent: Rent,
    /// inflation config
    pub inflation: Inflation,
    /// how slots map to epochs
    pub epoch_schedule: EpochSchedule,
    /// rewards distribution config
    pub epoch_rewards: EpochRewards,
    /// tracking the current slot, epoch, and time.
    pub clock: Clock,
    /// network (mainnet, devnet, testnet)
    pub cluster_type: ClusterType,
    /// collector ID
    pub collector_id: SolanaAddress,
}

impl Default for InitSvmConfig {
    fn default() -> Self {
        Self {
            creation_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as UnixTimestamp,
            accounts: BTreeMap::default(),
            faucet_keypair: None,
            native_instruction_processors: Vec::default(),
            rewards_pools: BTreeMap::default(),
            // At 160 ticks/s, 64 ticks per slot implies that leader rotation and voting will happen
            // every 400 ms. A fast voting cadence ensures faster finality and convergence
            ticks_per_slot: DEFAULT_TICKS_PER_SLOT,
            poh_config: PohConfig::default(),
            inflation: Inflation::default(),
            fee_rate_governor: FeeRateGovernor::default(),
            rent: Rent::default(),
            epoch_schedule: EpochSchedule::default(),
            epoch_rewards: EpochRewards::default(),
            clock: Clock::default(),
            cluster_type: ClusterType::Development,
            collector_id: Pubkey::default().into(),
        }
    }
}

impl InitSvmConfig {
    pub fn new(
        accounts: &[(Pubkey, AccountSharedData)],
        native_instruction_processors: &[(&str, Pubkey)],
    ) -> Self {
        let accounts = accounts
            .iter()
            .map(|(key, account)| (key.into(), account.clone().into()))
            .collect();
        let native_instruction_processors = native_instruction_processors
            .iter()
            .map(|(name, pubkey)| ((*name).to_string(), svm_types::SolanaAddress::from(pubkey)))
            .collect();
        Self {
            accounts,
            native_instruction_processors,
            ..InitSvmConfig::default()
        }
    }

    pub fn hash(&self) -> Hash {
        let serialized = serialize(&self).unwrap();
        hash(&serialized)
    }

    fn genesis_filename(ledger_path: &Path) -> PathBuf {
        Path::new(ledger_path).join(DEFAULT_GENESIS_FILE)
    }

    pub fn write(&self, ledger_path: &Path) -> anyhow::Result<(), std::io::Error> {
        let serialized = serialize(&self).map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Unable to serialize: {err:?}"),
            )
        })?;

        std::fs::create_dir_all(ledger_path)?;

        let mut file = File::create(Self::genesis_filename(ledger_path))?;
        file.write_all(&serialized)
    }

    pub fn add_account(&mut self, pubkey: Pubkey, account: AccountSharedData) {
        self.accounts.insert(pubkey.into(), account.into());
    }

    pub fn add_native_instruction_processor(&mut self, name: &str, program_id: Pubkey) {
        self.native_instruction_processors
            .push((name.to_string(), program_id.into()));
    }

    pub fn hashes_per_tick(&self) -> Option<u64> {
        self.poh_config.hashes_per_tick
    }

    pub fn ticks_per_slot(&self) -> u64 {
        self.ticks_per_slot
    }

    pub fn ns_per_slot(&self) -> u128 {
        self.poh_config
            .target_tick_duration
            .as_nanos()
            .saturating_mul(self.ticks_per_slot() as u128)
    }

    pub fn slots_per_year(&self) -> f64 {
        years_as_slots(
            1.0,
            &self.poh_config.target_tick_duration,
            self.ticks_per_slot(),
        )
    }
}

impl std::fmt::Display for InitSvmConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "\
             Creation time: {}\n\
             Cluster type: {:?}\n\
             Genesis hash: {}\n\
             Shred version: {}\n\
             Ticks per slot: {:?}\n\
             Hashes per tick: {:?}\n\
             Target tick duration: {:?}\n\
             Slots per epoch: {}\n\
             Warmup epochs: {}abled\n\
             Slots per year: {}\n\
             {:?}\n\
             {:?}\n\
             {:?}\n\
             Capitalization: {} SOL in {} accounts\n\
             Native instruction processors: {:#?}\n\
             Rewards pool: {:#?}\n\
             Collector ID: {}\n\
             ",
            Utc.timestamp_opt(self.creation_time, 0)
                .unwrap()
                .to_rfc3339(),
            self.cluster_type,
            self.hash(),
            compute_shred_version(&self.hash(), None),
            self.ticks_per_slot,
            self.poh_config.hashes_per_tick,
            self.poh_config.target_tick_duration,
            self.epoch_schedule.slots_per_epoch,
            if self.epoch_schedule.warmup {
                "en"
            } else {
                "dis"
            },
            self.slots_per_year(),
            self.inflation,
            self.rent,
            self.fee_rate_governor,
            lamports_to_sol(
                self.accounts
                    .iter()
                    .map(|(pubkey, account)| {
                        assert!(account.lamports > 0, "{:?}", (pubkey, account));
                        account.lamports
                    })
                    .sum::<u64>()
            ),
            self.accounts.len(),
            self.native_instruction_processors,
            self.rewards_pools,
            self.collector_id,
        )
    }
}

fn create_account(data: Vec<u8>, owner: Pubkey, executable: bool, rent: &Rent) -> Account {
    Account {
        lamports: rent.minimum_balance(data.len()),
        data,
        owner,
        executable,
        rent_epoch: 0,
    }
}

impl<S: Spec> SVM<S>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    pub(crate) fn init_module(
        &mut self,
        config: &<Self as Module>::Config,
        working_set: &mut impl GenesisState<S>,
    ) -> SVMRollupResult {
        self.initialize_genesis_hash(config, working_set)?;

        self.initialize_sysvars(config, working_set)?;
        self.initialize_spl_programs(config, working_set)?;
        self.initialize_solana_accounts(config, working_set)?;
        self.initialize_creation_time(config, working_set)?;
        self.initialize_collector_id(config, working_set)?;

        self.initialize_epoch_info(working_set)?;
        self.initialize_fee_rate_governor(working_set)?;
        self.initialize_fee_structure(working_set)?;
        self.initialize_blockhash(config, working_set)?;
        self.initialize_compute_budget(working_set)?;
        self.initialize_feature_set(working_set)?;
        self.initialize_reserved_account_keys(working_set)?;
        Ok(())
    }

    /// Initializes the genesis hash
    fn initialize_genesis_hash(
        &mut self,
        config: &InitSvmConfig,
        working_set: &mut impl GenesisState<S>,
    ) -> SVMRollupResult {
        self.genesis_hash
            .set(&config.hash().to_string(), working_set)
            .map_err(|e| {
                SVMStateError::Write(format!(
                    "Failed to initialize genesis hash during genesis: {e}"
                ))
            })?;
        Ok(())
    }

    /// Initializes configured Solana accounts
    fn initialize_solana_accounts(
        &mut self,
        config: &InitSvmConfig,
        working_set: &mut impl GenesisState<S>,
    ) -> SVMRollupResult {
        for (address, proxy) in config.accounts.iter() {
            let pubkey = address.into();
            let account: Account = proxy.into();

            let bank_balance = self.get_bank_balance(&pubkey, working_set)?;

            let svm_balance_unset = account.lamports == 0;
            let bank_balance_unset = bank_balance == 0;
            debug_assert!(svm_balance_unset || bank_balance_unset,
                "SVM account balance can only be set from one genesis config to avoid conflicts. Choose either the bank or the SVM module genesis config."
            );

            self.insert_account(&pubkey, &account, working_set)?;

            self.update_owner_index(&account.owner, &[address.into()], working_set)?;
        }

        Ok(())
    }

    fn initialize_spl_programs(
        &mut self,
        config: &InitSvmConfig,
        working_set: &mut impl GenesisState<S>,
    ) -> SVMRollupResult {
        let mut bpf_programs = Vec::new();
        let mut bpf_upgradeable_programs = Vec::new();

        for spl_program in SPL_PROGRAMS.iter() {
            let bytecode = spl_program.bytecode;

            let program_id = spl_program.program_id;

            if bpf_loader::check_id(&spl_program.owner_program) {
                let account =
                    create_account(bytecode.to_vec(), bpf_loader::id(), true, &config.rent);

                self.insert_account(&program_id, &account, working_set)?;

                bpf_programs.push(program_id);
            } else if bpf_loader_upgradeable::check_id(&spl_program.owner_program) {
                let accounts =
                    bpf_loader_upgradeable_program_accounts(&program_id, bytecode, &config.rent);

                for (address, account) in accounts {
                    self.insert_account(&address, &account, working_set)?;

                    bpf_upgradeable_programs.push(address);
                }
            }
        }

        // Update the owner index with the collected program IDs
        if !bpf_programs.is_empty() {
            self.update_owner_index(&bpf_loader::id(), &bpf_programs, working_set)?;
        }
        if !bpf_upgradeable_programs.is_empty() {
            self.update_owner_index(
                &bpf_loader_upgradeable::id(),
                &bpf_upgradeable_programs,
                working_set,
            )?;
        }

        Ok(())
    }

    /// Initializes Solana system accounts to support basic chain features.
    fn initialize_sysvars(
        &mut self,
        config: &InitSvmConfig,
        working_set: &mut impl GenesisState<S>,
    ) -> SVMRollupResult {
        // https://docs.solanalabs.com/runtime/sysvars
        let sysvars = [
            // Rent
            (
                rent::id(),
                serialize(&config.rent).map_err(|e| {
                    SVMStateError::Write(format!(
                        "Failed to serialize rent into bytes during genesis: {e}"
                    ))
                })?,
            ),
            // Clock
            (
                clock::id(),
                serialize(&config.clock).map_err(|e| {
                    SVMStateError::Write(format!(
                        "Failed to serialize clock into bytes during genesis: {e}"
                    ))
                })?,
            ),
            // Epoch Schedule
            (
                epoch_schedule::id(),
                serialize(&config.epoch_schedule).map_err(|e| {
                    SVMStateError::Write(format!(
                        "Failed to serialize epoch schedule into bytes during genesis: {e}"
                    ))
                })?,
            ),
            // Epoch Rewards
            (
                epoch_rewards::id(),
                serialize(&config.epoch_rewards).map_err(|e| {
                    SVMStateError::Write(format!(
                        "Failed to serialize epoch rewards into bytes during genesis: {e}"
                    ))
                })?,
            ),
        ];

        for (id, data) in sysvars.iter() {
            let account = create_account(data.clone(), sysvar::id(), false, &config.rent);
            self.insert_account(id, &account, working_set)?;
        }

        self.update_owner_index(
            &sysvar::id(),
            &sysvars
                .into_iter()
                .map(|(id, _)| id)
                .collect::<Vec<Pubkey>>(),
            working_set,
        )?;

        Ok(())
    }

    /// Initializes the creation time of the rollup
    fn initialize_creation_time(
        &mut self,
        config: &InitSvmConfig,
        working_set: &mut impl GenesisState<S>,
    ) -> SVMRollupResult {
        self.creation_time
            .set(&config.creation_time, working_set)
            .map_err(|e| {
                SVMStateError::Write(format!(
                    "Failed to initialize creation time during genesis: {e}"
                ))
            })?;
        Ok(())
    }

    /// Initializes `EpochInfo` with all fields set to zero.
    fn initialize_epoch_info(&mut self, working_set: &mut impl GenesisState<S>) -> SVMRollupResult {
        let epoch_info = EpochInfo {
            epoch: 0,
            slot_index: 0,
            slots_in_epoch: 0,
            absolute_slot: 0,
            block_height: 0,
            transaction_count: None,
        };
        self.epoch_info.set(&epoch_info, working_set).map_err(|e| {
            SVMStateError::Write(format!(
                "Failed to initialize epoch info during genesis: {e}"
            ))
        })?;
        Ok(())
    }

    /// Initializes the blockhash queue with the genesis hash.
    fn initialize_blockhash(
        &mut self,
        config: &<Self as Module>::Config,
        working_set: &mut impl GenesisState<S>,
    ) -> SVMRollupResult {
        let lamports_per_signature = {
            let fee_structure = self
                .fee_structure
                .get(working_set)
                .map_err(|e| SVMStateError::Read(format!("Failed to retrieve fee structure: {e}")))?
                .ok_or_else(|| {
                    SVMStateError::Read("Fee structure is not initialized".to_string())
                })?;
            fee_structure.lamports_per_signature
        };

        let mut blockhash_queue = BlockhashQueue::default();
        blockhash_queue.genesis_hash(&config.hash(), lamports_per_signature);

        self.blockhash_queue
            .set(&blockhash_queue, working_set)
            .map_err(|e| {
                SVMStateError::Write(format!(
                    "Failed to initialize blockhash queue during genesis: {e}"
                ))
            })?;
        Ok(())
    }

    /// Initialize a default `FeeRateGovernor`.
    fn initialize_fee_rate_governor(
        &mut self,
        working_set: &mut impl GenesisState<S>,
    ) -> SVMRollupResult {
        // It doesn't matter what we set for the values of fee rate governor
        // as long as the `lamports_per_signature` field is nonzero,
        // otherwise the fee structure's transaction fees won't be respected.
        // https://github.com/nitro-svm/agave/blob/50f12b00402f2539691d83d522886c43c38148d8/sdk/src/fee.rs#L157-L161
        self.fee_rate_governor
            .set(
                &FeeRateGovernor::new(
                    DEFAULT_TARGET_LAMPORTS_PER_SIGNATURE,
                    DEFAULT_TARGET_SIGNATURES_PER_SLOT,
                )
                .into(),
                working_set,
            )
            .map_err(|e| {
                SVMStateError::Write(format!(
                    "Failed to initialize fee rate governor during genesis: {e}"
                ))
            })?;
        Ok(())
    }

    /// Initialize a default `ComputeBudget`.
    fn initialize_compute_budget(
        &mut self,
        working_set: &mut impl GenesisState<S>,
    ) -> SVMRollupResult {
        self.compute_budget
            .set(&ComputeBudget::default(), working_set)
            .map_err(|e| {
                SVMStateError::Write(format!(
                    "Failed to initialize compute budget during genesis: {e}"
                ))
            })?;
        Ok(())
    }

    /// Initialize a default `FeeStructure`.
    fn initialize_fee_structure(
        &mut self,
        working_set: &mut impl GenesisState<S>,
    ) -> SVMRollupResult {
        self.fee_structure
            .set(&FeeStructure::default(), working_set)
            .map_err(|e| {
                SVMStateError::Write(format!(
                    "Failed to initialize fee structure during genesis: {e}"
                ))
            })?;
        Ok(())
    }

    /// Initializes the set of activated features
    fn initialize_feature_set(
        &mut self,
        working_set: &mut impl GenesisState<S>,
    ) -> SVMRollupResult {
        self.feature_set
            .set(&FeatureSet::all_enabled(), working_set)
            .map_err(|e| {
                SVMStateError::Write(format!(
                    "Failed to initialize feature set during genesis: {e}"
                ))
            })?;
        Ok(())
    }

    fn initialize_reserved_account_keys(
        &mut self,
        working_set: &mut impl GenesisState<S>,
    ) -> SVMRollupResult {
        let feature_set = self
            .feature_set
            .get(working_set)
            .map_err(|e| SVMStateError::Read(format!("Failed to retrieve feature set: {e}")))?
            .ok_or_else(|| SVMStateError::Read("Feature set is not initialized".to_string()))?;

        let mut reserved_keys = ReservedAccountKeys::default();
        reserved_keys.update_active_set(&feature_set);

        let reserved_account_keys: BTreeSet<Pubkey> = reserved_keys.active.into_iter().collect();
        self.reserved_account_keys
            .set(&reserved_account_keys, working_set)
            .map_err(|e| {
                SVMStateError::Write(format!(
                    "Failed to initialize reserved account keys during genesis: {e}"
                ))
            })?;
        Ok(())
    }

    fn initialize_collector_id(
        &mut self,
        config: &InitSvmConfig,
        working_set: &mut impl GenesisState<S>,
    ) -> SVMRollupResult {
        self.collector_id
            .set(&config.collector_id.into(), working_set)
            .map_err(|e| {
                SVMStateError::Write(format!(
                    "Failed to initialize collector ID during genesis: {e}"
                ))
            })?;
        Ok(())
    }
}

// Adapted from Agave: https://github.com/anza-xyz/agave/blob/a66b023f2a1c0250c4d23ab15793eee1c46f1832/program-test/src/programs.rs#L65-L69
//
/// Returns the key accounts for the `bpf_loader_upgradeable` program on Solana:
/// 1. **Program Account**: A small account that serves as the public-facing handle for the program and is marked executable.
/// 2. **Program Data Account**: Stores the executable bytecode and metadata, including the upgrade authority.
///
/// The function sets up both accounts, allowing the program to be upgraded by modifying the program data account
/// without changing the program account's public key or its associated relationships.
fn bpf_loader_upgradeable_program_accounts(
    program_id: &Pubkey,
    elf: &[u8],
    rent: &Rent,
) -> [(Pubkey, Account); 2] {
    let programdata_address = get_program_data_address(program_id);
    let program_account = {
        let space = UpgradeableLoaderState::size_of_program();
        let lamports = rent.minimum_balance(space);
        let data = bincode::serialize(&UpgradeableLoaderState::Program {
            programdata_address,
        })
        .unwrap();
        Account {
            lamports,
            data,
            owner: bpf_loader_upgradeable::id(),
            executable: true,
            rent_epoch: 0,
        }
    };
    let programdata_account = {
        let space = UpgradeableLoaderState::size_of_programdata_metadata() + elf.len();
        let lamports = rent.minimum_balance(space);
        let mut data = bincode::serialize(&UpgradeableLoaderState::ProgramData {
            slot: 0,
            upgrade_authority_address: Some(Pubkey::default()),
        })
        .unwrap();
        data.extend_from_slice(elf);
        Account {
            lamports,
            data,
            owner: bpf_loader_upgradeable::id(),
            executable: false,
            rent_epoch: 0,
        }
    };
    [
        (*program_id, program_account),
        (programdata_address, programdata_account),
    ]
}
