use std::{
    collections::{HashMap, HashSet},
    panic::catch_unwind,
    str::FromStr,
};

use itertools::Itertools;
use jsonrpsee::{
    core::RpcResult,
    types::error::{ErrorCode, ErrorObjectOwned},
};
use solana_account_decoder::{
    parse_account_data::{AccountAdditionalDataV2, SplTokenAdditionalData},
    parse_token::{
        get_token_account_mint, is_known_spl_token_id, token_amount_to_ui_amount_v2, UiTokenAmount,
    },
    UiAccount, UiAccountEncoding, MAX_BASE58_BYTES,
};
use solana_compute_budget::compute_budget_processor::process_compute_budget_instructions;
use solana_program::{clock::UnixTimestamp, rent::Rent, sysvar::rent};
use solana_program_runtime::timings::ExecuteTimings;
use solana_rpc_client_api::{
    config::{
        RpcAccountInfoConfig, RpcBlockConfig, RpcBlocksConfigWrapper, RpcContextConfig,
        RpcEncodingConfigWrapper, RpcProgramAccountsConfig, RpcSignatureStatusConfig,
        RpcSignaturesForAddressConfig, RpcSimulateTransactionConfig, RpcTokenAccountsFilter,
        RpcTransactionConfig,
    },
    filter::RpcFilterType,
    request::{MAX_GET_CONFIRMED_BLOCKS_RANGE, MAX_GET_CONFIRMED_SIGNATURES_FOR_ADDRESS2_LIMIT},
    response::{
        Response, RpcBlockhash, RpcConfirmedTransactionStatusWithSignature, RpcContactInfo,
        RpcKeyedAccount, RpcPrioritizationFee, RpcSimulateTransactionResult, RpcVersionInfo,
    },
};
use solana_sdk::{
    account::{Account, ReadableAccount},
    clock::{Slot, MAX_PROCESSING_AGE},
    commitment_config::{CommitmentConfig, CommitmentLevel},
    epoch_info::EpochInfo,
    feature_set::{
        include_loaded_accounts_data_size_in_fee_calculation, remove_rounding_in_fee_calculation,
        FEATURE_NAMES,
    },
    hash::{Hash, Hasher},
    message::{Message, SanitizedMessage},
    pubkey::Pubkey,
    signature::Signature,
    transaction::{SanitizedTransaction, TransactionError, VersionedTransaction},
    vote::state::MAX_LOCKOUT_HISTORY,
};
use solana_svm::{
    transaction_processor::{LoadAndExecuteSanitizedTransactionsOutput, TransactionBatchProcessor},
    transaction_results::TransactionExecutionResult,
};
use solana_transaction_status::{
    map_inner_instructions, BlockEncodingOptions, ConfirmedBlock,
    ConfirmedTransactionStatusWithSignature, ConfirmedTransactionWithStatusMeta,
    EncodedConfirmedTransactionWithStatusMeta, TransactionBinaryEncoding,
    TransactionConfirmationStatus, TransactionStatus, TransactionWithStatusMeta, UiConfirmedBlock,
    UiInnerInstructions, UiTransactionEncoding,
};
use sov_address::FromVmAddress;
use sov_modules_api::{
    macros::rpc_gen, prelude::UnwrapInfallible, ApiStateAccessor, Spec, StateAccessor,
};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token::{solana_program::program_pack::Pack, state::Account as TokenAccount};
use spl_token_2022::{
    extension::{
        interest_bearing_mint::InterestBearingConfig, AccountType::Account as ACCOUNTTYPE_ACCOUNT,
        BaseStateWithExtensions, StateWithExtensions,
    },
    state::Account as Token2022Account,
};
use svm_engine::verification::sanitize_tx;
use svm_types::SolanaAddress;

use crate::{rpc::utils::decode_and_deserialize, wrappers::ExecutedTransactionData, SVM};

pub mod sequencer;
pub mod utils;

/// Used like in [https://github.com/nitro-svm/agave/blob/86d2adfd1e59280a63f375af45488b5684fa8165/inline-spl/src/token.rs#L24]
/// but imported directly from the [`spl_token_2022::state::Account`] struct.
const SPL_TOKEN_ACCOUNT_LENGTH: usize = Token2022Account::LEN;

#[rpc_gen(client, server)]
impl<S: Spec> SVM<S>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    // ------ MINIMAL RPC ENDPOINTS ------

    /// Queries account balance referred by public key
    #[rpc_method(name = "getBalance")]
    pub fn get_balance(
        &self,
        pubkey: SolanaAddress,
        _config: Option<RpcContextConfig>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Response<u64>> {
        self.to_response(
            self.get_bank_balance(&pubkey.into(), state)
                .unwrap_or_default(),
            state,
        )
    }

    /// Queries the current epoch information
    #[rpc_method(name = "getEpochInfo")]
    pub fn get_epoch_info(
        &self,
        _config: Option<RpcContextConfig>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<EpochInfo> {
        let epoch_info = self
            .epoch_info
            .get(state)
            .unwrap_infallible()
            .ok_or_else(|| create_server_error("Epoch info not found"))?;
        Ok(epoch_info)
    }

    /// Get the genesis hash
    #[rpc_method(name = "getGenesisHash")]
    pub fn get_genesis_hash(&self, state: &mut ApiStateAccessor<S>) -> RpcResult<String> {
        let genesis_hash = self
            .genesis_hash
            .get(state)
            .unwrap_infallible()
            .ok_or_else(|| create_server_error("Genesis hash not found"))?;
        Ok(genesis_hash)
    }

    /// Queries SVM module current health
    #[rpc_method(name = "getHealth")]
    pub fn get_health(&self) -> RpcResult<String> {
        Ok("ok".to_string())
    }

    /// Queries current slot
    #[rpc_method(name = "getSlot")]
    pub fn get_slot(
        &self,
        config: Option<RpcContextConfig>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Slot> {
        if config.and_then(|c| c.commitment).is_some() {
            let slot = self
                .confirmed_block_slots
                .last(state)
                .unwrap_infallible()
                .ok_or_else(|| create_server_error("Epoch info not found"))?;
            return Ok(slot);
        }

        let epoch_info = self
            .epoch_info
            .get(state)
            .unwrap_infallible()
            .ok_or_else(|| create_server_error("Epoch info not found"))?;
        Ok(epoch_info.absolute_slot)
    }

    /// Queries the current block height
    #[rpc_method(name = "getBlockHeight")]
    pub fn get_block_height(
        &self,
        _config: Option<RpcContextConfig>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<u64> {
        let epoch_info = self
            .epoch_info
            .get(state)
            .unwrap_infallible()
            .ok_or_else(|| create_server_error("Epoch info not found"))?;
        // The block height we store is the height of the next block
        let block_height = epoch_info.block_height - 1;
        Ok(block_height)
    }

    /// Get the accumulated executed transaction count
    #[rpc_method(name = "getTransactionCount")]
    pub fn get_transaction_count(
        &self,
        _config: Option<RpcContextConfig>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<u64> {
        let transaction_count = self
            .transactions_count
            .get(state)
            .unwrap_infallible()
            .unwrap_or_default();

        Ok(transaction_count)
    }

    /// Get the validator version information
    #[rpc_method(name = "getVersion")]
    pub fn get_version(&self) -> RpcResult<RpcVersionInfo> {
        // Copied from https://github.com/nitro-svm/agave/blob/f0d7e4f71cc1f0ed2d1f12614b4f254fb13f2303/sdk/feature-set/src/lib.rs#L1104-L1113
        let feature_set_hash = {
            let mut hasher = Hasher::default();
            let mut feature_ids = FEATURE_NAMES.keys().collect::<Vec<_>>();
            feature_ids.sort();
            for feature in feature_ids {
                hasher.hash(feature.as_ref());
            }
            hasher.result()
        };
        let feature_set = u32::from_le_bytes(feature_set_hash.as_ref()[..4].try_into().unwrap());
        Ok(RpcVersionInfo {
            solana_core: "2.0.5".to_string(),
            feature_set: Some(feature_set),
        })
    }

    // ------ BANK ENDPOINTS ------

    #[rpc_method(name = "getMinimumBalanceForRentExemption")]
    pub fn get_minimum_balance_for_rent_exemption(
        &self,
        data_len: usize,
        _commitment: Option<CommitmentConfig>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<u64> {
        let rent_account = self
            .accounts
            .get(&rent::id(), state)
            .unwrap_infallible()
            .ok_or_else(|| {
                create_client_error(&format!(
                    "Rent account not found with pubkey: {}",
                    rent::id()
                ))
            })?;
        let serialized_rent = rent_account.data();
        let rent: Rent = bincode::deserialize(serialized_rent)
            .map_err(|e| create_server_error(&format!("Could not deserialize rent: {e}")))?;

        Ok(rent.minimum_balance(data_len))
    }

    #[rpc_method(name = "getSlotLeader")]
    pub fn get_slot_leader(
        &self,
        _config: Option<RpcContextConfig>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<String> {
        Ok(self
            .collector_id
            .get(state)
            .unwrap_infallible()
            .ok_or_else(|| create_server_error("Slot leader not found"))?
            .to_string())
    }

    #[rpc_method(name = "getSlotLeaders")]
    pub fn get_slot_leaders(
        &self,
        _start_slot: Slot,
        limit: u64,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Vec<String>> {
        let leader = self
            .collector_id
            .get(state)
            .unwrap_infallible()
            .ok_or_else(|| create_server_error("Slot leaders not found"))?;
        Ok(vec![leader.to_string(); limit as usize])
    }

    // ------ ACCOUNTS DATA ENDPOINTS ------

    // #[rpc_method(name = "getBlockCommitment")]
    // fn get_block_commitment(
    //     &self,
    //     block: Slot,
    // ) -> Result<RpcBlockCommitment<BlockCommitmentArray>>;

    // SPL Token-specific RPC endpoints
    // See https://github.com/solana-labs/solana-program-library/releases/tag/token-v2.0.0 for
    // program details

    // #[rpc(meta, name = "getTokenSupply")]
    // fn get_token_supply(
    //     &self,
    //     meta: Self::Metadata,
    //     mint_str: String,
    //     commitment: Option<CommitmentConfig>,
    // ) -> Result<RpcResponse<UiTokenAmount>>;

    /// Queries account information referred by public key
    #[rpc_method(name = "getAccountInfo")]
    pub fn get_account_info(
        &self,
        pubkey: SolanaAddress,
        config: Option<RpcAccountInfoConfig>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Response<Option<UiAccount>>> {
        let Some(account) = self.get_account_data(&pubkey.into(), state) else {
            return self.to_response(None, state);
        };

        let (encoding, data_slice) = match config {
            Some(RpcAccountInfoConfig {
                encoding,
                data_slice,
                ..
            }) => (
                encoding.unwrap_or(UiAccountEncoding::Base64Zstd),
                data_slice,
            ),
            None => (UiAccountEncoding::Base64Zstd, None),
        };

        self.to_response(
            Some(UiAccount::encode(
                &pubkey.into(),
                &account,
                encoding,
                None,
                data_slice,
            )),
            state,
        )
    }

    /// Queries multiple accounts information referred by list of public keys
    #[rpc_method(name = "getMultipleAccounts")]
    pub fn get_multiple_accounts(
        &self,
        pubkeys: Vec<SolanaAddress>,
        config: Option<RpcAccountInfoConfig>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Response<Vec<Option<UiAccount>>>> {
        pubkeys
            .into_iter()
            .map(|pubkey| {
                self.get_account_info(pubkey, config.clone(), state)
                    .map_err(|e| {
                        create_client_error(&format!("Account with pubkey {pubkey} not found: {e}"))
                    })
                    .map(|res| res.value)
            })
            .collect::<RpcResult<Vec<Option<UiAccount>>>>()
            .and_then(|r| self.to_response(r, state))
    }

    #[rpc_method(name = "getTokenAccountBalance")]
    pub fn get_token_account_balance(
        &self,
        pubkey: SolanaAddress,
        _commitment: Option<CommitmentConfig>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Response<UiTokenAmount>> {
        let token_account_info = self
            .get_account_info(pubkey, None, state)
            .map_err(|_| create_client_error("Invalid param: could not find account"))?
            .value
            .ok_or_else(|| create_client_error("Invalid param: could not find account"))?;

        let token_slice: &[u8] = &token_account_info
            .data
            .decode()
            .ok_or_else(|| create_client_error("Invalid param: could not decode account data"))?;

        let token_account = TokenAccount::unpack(token_slice).map_err(|e| {
            create_client_error(&format!(
                "Failed to unpack spl token account from data: {}",
                e
            ))
        })?;

        let mint_pubkey = token_account.mint;
        let mint_account_info = self
            .get_account_info(mint_pubkey.into(), None, state)
            .map_err(|_| create_client_error("Invalid param: could not find account"))?
            .value
            .ok_or_else(|| create_client_error("Invalid param: could not find account"))?;
        let mint_slice: &[u8] = &mint_account_info.data.decode().ok_or_else(|| {
            create_client_error("Invalid param: could not decode mint account data")
        })?;

        let mint_account = spl_token_2022::state::Mint::unpack(mint_slice).map_err(|e| {
            create_server_error(&format!("Failed to unpack mint account from data: {e}"))
        })?;

        self.to_response(
            token_amount_to_ui_amount_v2(
                token_account.amount,
                &SplTokenAdditionalData::with_decimals(mint_account.decimals),
            ),
            state,
        )
    }

    // ------ ACCOUNTS SCAN ENDPOINTS ------

    #[rpc_method(name = "getProgramAccounts")]
    pub fn get_program_accounts(
        &self,
        pubkey: SolanaAddress,
        config: Option<RpcProgramAccountsConfig>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Vec<RpcKeyedAccount>> {
        let account_info = self
            .get_account_info(
                pubkey,
                config.clone().map(|config| config.account_config),
                state,
            )
            .map_err(|e| {
                create_client_error(&format!(
                    "Failed to retrieve account with pubkey {pubkey}: {e}"
                ))
            })?
            .value
            .ok_or_else(|| {
                create_client_error(&format!("Account with pubkey {pubkey} not found"))
            })?;
        if !account_info.executable {
            return Err(create_client_error(&format!(
                "Account with pubkey {pubkey} is not executable"
            )));
        }

        let owned_accounts = self.get_accounts_for_owner(&pubkey.into(), state)?;
        let mut owned_accounts_data = self.get_accounts_data(owned_accounts, state)?;

        let (account_config, filters) = config
            .map(|config| (config.account_config, config.filters))
            .unwrap_or_default();

        if let Some(mut filters) = filters {
            // Based on agave implementation at https://github.com/nitro-svm/agave/blob/86d2adfd1e59280a63f375af45488b5684fa8165/rpc/src/rpc.rs#L2235
            filters.iter_mut().for_each(|filter_type| {
                if let RpcFilterType::Memcmp(compare) = filter_type {
                    if let Err(err) = compare.convert_to_raw_bytes() {
                        // All filters should have been previously verified
                        tracing::warn!("Invalid filter: bytes could not be decoded, {err}");
                    }
                }
            });

            // Composed from the underlaying implementation in agave at https://github.com/nitro-svm/agave/blob/86d2adfd1e59280a63f375af45488b5684fa8165/rpc/src/rpc.rs#L2038-L2042
            let filter_closure = |account: &Account| {
                let account_data = account.data();
                tracing::error!("Account data: {account_data:?}");
                filters.iter().all(|filter_type| match filter_type {
                    RpcFilterType::DataSize(size) => account_data.len() as u64 == *size,
                    RpcFilterType::Memcmp(compare) => compare.bytes_match(account_data),
                    RpcFilterType::TokenAccountState => {
                        SPL_TOKEN_ACCOUNT_LENGTH == account_data.len()
                            || &(ACCOUNTTYPE_ACCOUNT as u8)
                                == account_data.get(SPL_TOKEN_ACCOUNT_LENGTH).unwrap_or(&0)
                    }
                })
            };

            owned_accounts_data.retain(|(_, account)| filter_closure(account));
        }

        Ok(self.encode_accounts(owned_accounts_data, Some(account_config)))
    }

    #[rpc_method(name = "getTokenAccountsByOwner")]
    pub fn get_token_accounts_by_owner(
        &self,
        owner: SolanaAddress,
        token_account_filter: RpcTokenAccountsFilter,
        config: Option<RpcAccountInfoConfig>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Response<Vec<RpcKeyedAccount>>> {
        let atas = match token_account_filter {
            // caller only wants an ATA for a specific token mint
            // e.g. alice only wants to check the balance for her bonk tokens
            RpcTokenAccountsFilter::Mint(mint_str) => {
                self.get_ata_for_single_mint(&owner.into(), &mint_str, state)?
            }
            // caller wants all ATAs that are either SPL or SPL 2022
            // e.g. alice wants to see all her bonk, doge, pepe
            RpcTokenAccountsFilter::ProgramId(program_str) => {
                self.get_atas_for_spl_program(&owner.into(), &program_str, state)?
            }
        };

        self.to_response(self.encode_accounts(atas, config), state)
    }

    // ------ FULL RPC ENDPOINTS ------

    #[rpc_method(name = "getClusterNodes")]
    pub fn get_cluster_nodes(
        &self,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Vec<RpcContactInfo>> {
        let pubkey = self
            .collector_id
            .get(state)
            .unwrap_infallible()
            .ok_or_else(|| create_server_error("Collector ID not found"))?
            .to_string();

        let version = self.get_version()?;

        Ok(vec![RpcContactInfo {
            pubkey,
            version: Some(version.solana_core),
            feature_set: version.feature_set,
            rpc: None,
            shred_version: None,
            gossip: None,
            tvu: None,
            tpu: None,
            tpu_quic: None,
            tpu_forwards: None,
            tpu_forwards_quic: None,
            tpu_vote: None,
            serve_repair: None,
            pubsub: None,
        }])
    }

    #[rpc_method(name = "getSignatureStatuses")]
    pub fn get_signature_statuses(
        &self,
        signature_strs: Vec<String>,
        _config: Option<RpcSignatureStatusConfig>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Response<Vec<Option<TransactionStatus>>>> {
        let processed_txs: Vec<ExecutedTransactionData> =
            self.processed_transactions.collect_infallible(state);

        let current_slot = self
            .epoch_info
            .get(state)
            .unwrap_infallible()
            .ok_or_else(|| create_server_error("Epoch info not found"))?
            .slot_index;

        let statuses = signature_strs
            .iter()
            .map(|signature_str| -> RpcResult<Option<TransactionStatus>> {
                let signature = Signature::from_str(signature_str).map_err(|_| {
                    create_client_error(&format!("Invalid signature: {signature_str}"))
                })?;

                if let Some(confirmed_transaction) = self
                    .confirmed_transactions
                    .get(&signature, state)
                    .unwrap_infallible()
                {
                    let slots = (current_slot - confirmed_transaction.slot) as usize;
                    let (confirmations, confirmation_status) = if slots <= MAX_LOCKOUT_HISTORY + 1 {
                        (Some(slots), Some(TransactionConfirmationStatus::Confirmed))
                    } else {
                        (None, Some(TransactionConfirmationStatus::Finalized))
                    };
                    let status = TransactionStatus {
                        slot: confirmed_transaction.slot,
                        confirmations,
                        status: Ok(()),
                        err: None,
                        confirmation_status,
                    };
                    return Ok(Some(status));
                }

                let is_processed_tx = processed_txs
                    .iter()
                    .any(|tx| tx.transaction.signatures[0] == signature);

                Ok(is_processed_tx.then_some(TransactionStatus {
                    slot: current_slot,
                    confirmations: None,
                    status: Ok(()),
                    err: None,
                    confirmation_status: Some(TransactionConfirmationStatus::Processed),
                }))
            })
            .collect::<RpcResult<Vec<Option<_>>>>()?;

        self.to_response(statuses, state)
    }

    #[rpc_method(name = "simulateTransaction")]
    pub fn simulate_transaction(
        &self,
        data: String,
        config: Option<RpcSimulateTransactionConfig>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Response<RpcSimulateTransactionResult>> {
        let mut unmetered = state.to_unmetered();

        let reserved_account_keys: HashSet<Pubkey> = self
            .reserved_account_keys
            .get(&mut unmetered)
            .unwrap_infallible()
            .ok_or_else(|| create_server_error("Failed to retrieve reserved account keys"))?
            .into_iter()
            .collect();

        let encoding = config.as_ref().and_then(|config| config.encoding);
        let encoding = encoding.unwrap_or(UiTransactionEncoding::Base58);
        let encoding = encoding.into_binary_encoding().ok_or_else(|| {
            create_client_error(&format!(
                "Unsupported encoding: {encoding}. Supported encodings: base58, base64"
            ))
        })?;

        let (_, mut transaction) = decode_and_deserialize::<VersionedTransaction>(data, encoding)?;

        if config
            .as_ref()
            .map(|config| config.replace_recent_blockhash)
            == Some(true)
        {
            let blockhash = self
                .blockhash_queue
                .get(&mut unmetered)
                .unwrap_infallible()
                .ok_or_else(|| create_server_error("Failed to retrieve blockhash"))?;
            transaction
                .message
                .set_recent_blockhash(blockhash.last_hash());
        }

        // sanitize and verify transaction string
        let sanitized_tx = sanitize_tx(
            &transaction.into_legacy_transaction().unwrap(),
            &reserved_account_keys,
        )
        .map_err(|e| create_client_error(&format!("Failed to sanitize transaction: {e}")))?;

        let (slot, epoch) = {
            let info = self
                .epoch_info
                .get(&mut unmetered)
                .unwrap_infallible()
                .ok_or_else(|| create_client_error("Could not fetch feature set"))?;
            (info.absolute_slot, info.epoch)
        };

        let tx_config = self
            .get_load_and_execute_transactions_config(&mut unmetered)
            .map_err(|e| {
                create_server_error(&format!(
                    "Failed to retrieve config for transaction processor: {e}",
                ))
            })?;

        let check_results = self.get_transaction_check_results(
            &[sanitized_tx.clone()],
            tx_config.max_age,
            &mut unmetered,
        );

        let transaction_processor = TransactionBatchProcessor::new(slot, epoch, HashSet::default());

        let mut clone = self.clone();
        let engine = Self::initialize_engine_for_simulation(
            &mut clone,
            tx_config,
            &transaction_processor,
            &unmetered,
        );

        let simulation_output = engine
            .inner
            .load_and_execute_transaction(sanitized_tx.clone(), check_results[0].clone());

        // Extract simulation outputs as TransactionSimulationResult
        let output = self
            .get_simulation_output(simulation_output, config, sanitized_tx, state)
            .map_err(|e| {
                create_server_error(&format!("Failed to extract simulation results output: {e}",))
            })?;

        self.to_response(output, state)
    }

    #[rpc_method(name = "getBlock")]
    pub fn get_block(
        &self,
        slot: u64,
        config: Option<RpcEncodingConfigWrapper<RpcBlockConfig>>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Option<UiConfirmedBlock>> {
        let config = config
            .map(|config| config.convert_to_current())
            .unwrap_or_default();
        let encoding = config.encoding.unwrap_or(UiTransactionEncoding::Json);
        let encoding_options = BlockEncodingOptions {
            transaction_details: config.transaction_details.unwrap_or_default(),
            show_rewards: config.rewards.unwrap_or(true),
            max_supported_transaction_version: config.max_supported_transaction_version,
        };

        let Some(confirmed_block) = self.confirmed_blocks.get(&slot, state).unwrap_infallible()
        else {
            return Ok(None);
        };

        let confirmed_block: ConfirmedBlock = confirmed_block.into();

        confirmed_block
            .encode_with_options(encoding, encoding_options)
            .map(Some)
            .map_err(|e| create_server_error(&format!("Failed to encode confirmed block: {e}")))
    }

    // #[rpc_method(name = "minimumLedgerSlot")]
    // fn minimum_ledger_slot(&self, state: &mut ApiStateAccessor<S>) -> RpcResult<Slot>;

    #[rpc_method(name = "getBlockTime")]
    pub fn get_block_time(
        &self,
        slot: Slot,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Option<UnixTimestamp>> {
        if slot == 0 {
            return Ok(self.creation_time.get(state).unwrap_infallible());
        }

        self.confirmed_blocks
            .get(&slot, state)
            .unwrap_infallible()
            .map(|block| {
                block.block_time.ok_or_else(|| {
                    create_server_error(&format!("The block time is unavailable for slot: {slot}"))
                })
            })
            .transpose()
    }

    #[rpc_method(name = "getBlocks")]
    pub fn get_blocks(
        &self,
        start_slot: Slot,
        wrapper: Option<RpcBlocksConfigWrapper>,
        _config: Option<RpcContextConfig>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Vec<Slot>> {
        let (end_slot, _maybe_config) = wrapper.map(|wrapper| wrapper.unzip()).unwrap_or_default();
        let end_slot =
            end_slot.unwrap_or_else(|| start_slot.saturating_add(MAX_GET_CONFIRMED_BLOCKS_RANGE));

        if end_slot < start_slot {
            return Ok(Vec::new());
        }

        if end_slot - start_slot > MAX_GET_CONFIRMED_BLOCKS_RANGE {
            return Err(create_client_error(&format!(
                "Slot range too large; max {MAX_GET_CONFIRMED_BLOCKS_RANGE}"
            )));
        }

        let all_slots: Vec<Slot> = self.confirmed_block_slots.collect_infallible(state);

        Ok(all_slots
            .into_iter()
            .filter(|&slot| (start_slot..=end_slot).contains(&slot))
            .collect())
    }

    #[rpc_method(name = "getBlocksWithLimit")]
    pub fn get_blocks_with_limit(
        &self,
        start_slot: Slot,
        limit: usize,
        _config: Option<RpcContextConfig>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Vec<Slot>> {
        if limit > (MAX_GET_CONFIRMED_BLOCKS_RANGE as usize) {
            return Err(create_client_error(&format!(
                "Limit too large; max {MAX_GET_CONFIRMED_BLOCKS_RANGE}"
            )));
        }

        let end_slot = start_slot.saturating_add(limit as u64);

        let all_slots: Vec<Slot> = self.confirmed_block_slots.collect_infallible(state);

        Ok(all_slots
            .into_iter()
            .filter(|&slot| (start_slot..end_slot).contains(&slot))
            .collect())
    }

    #[rpc_method(name = "getTransaction")]
    pub fn get_transaction(
        &self,
        signature_str: String,
        config: Option<RpcEncodingConfigWrapper<RpcTransactionConfig>>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Option<EncodedConfirmedTransactionWithStatusMeta>> {
        let signature = Signature::from_str(&signature_str)
            .map_err(|_| create_client_error(&format!("Invalid signature: {signature_str}")))?;
        let config = config
            .map(|config| config.convert_to_current())
            .unwrap_or_default();
        let commitment = config.commitment.unwrap_or_default().commitment;

        let Some(transaction) = self.get_transaction_with_commitment(signature, commitment, state)
        else {
            return Ok(None);
        };

        let encoding = config.encoding.unwrap_or(UiTransactionEncoding::Json);
        let max_supported_transaction_version = config.max_supported_transaction_version;

        transaction
            .encode(encoding, max_supported_transaction_version)
            .map(Some)
            .map_err(|e| {
                create_server_error(&format!(
                    "Failed to encode confirmed transaction with status meta: {e}"
                ))
            })
    }

    #[rpc_method(name = "getSignaturesForAddress")]
    pub fn get_signatures_for_address(
        &self,
        address: SolanaAddress,
        config: Option<RpcSignaturesForAddressConfig>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Vec<RpcConfirmedTransactionStatusWithSignature>> {
        let RpcSignaturesForAddressConfig {
            before,
            until,
            limit,
            commitment,
            ..
        } = config.unwrap_or_default();
        let (before, until, limit) =
            verify_and_parse_signatures_for_address_params(before, until, limit)?;
        let commitment = commitment.unwrap_or_default();
        if !commitment.is_at_least_confirmed() {
            return Err(create_client_error("Invalid commitment level"));
        }
        let commitment_level = commitment.commitment;

        let slots: Vec<Slot> = self.confirmed_block_slots.collect_infallible(state);
        let highest_slot = slots.last().copied().unwrap_or_default();

        let (slot, mut before_excluded_signatures) = match before {
            None => (highest_slot, None),
            Some(before) => {
                match self.get_transaction_with_commitment(before, commitment_level, state) {
                    None => return Ok(Vec::new()),
                    Some(ConfirmedTransactionWithStatusMeta { slot, .. }) => {
                        let block = self
                            .confirmed_blocks
                            .get(&slot, state)
                            .unwrap_infallible()
                            .ok_or_else(|| create_client_error("Block not found"))?;
                        let mut slot_signatures = block
                            .transactions
                            .into_iter()
                            .filter_map(|tx| tx.transaction.signatures.first().copied())
                            .collect::<Vec<_>>();

                        if let Some(pos) = slot_signatures.iter().position(|&x| x == before) {
                            slot_signatures.truncate(pos + 1);
                        }

                        (
                            slot,
                            Some(slot_signatures.into_iter().collect::<HashSet<_>>()),
                        )
                    }
                }
            }
        };

        let first_available_block = self
            .confirmed_block_slots
            .get(0, state)
            .unwrap_infallible()
            .expect("Failed to get first available block");

        let (lowest_slot, until_excluded_signatures) = match until {
            None => (first_available_block, HashSet::new()),
            Some(until) => {
                match self.get_transaction_with_commitment(until, commitment_level, state) {
                    None => (first_available_block, HashSet::new()),
                    Some(ConfirmedTransactionWithStatusMeta { slot, .. }) => {
                        let block = self
                            .confirmed_blocks
                            .get(&slot, state)
                            .unwrap_infallible()
                            .ok_or_else(|| create_client_error("Block not found"))?;
                        let mut slot_signatures = block
                            .transactions
                            .into_iter()
                            .filter_map(|tx| tx.transaction.signatures.first().copied())
                            .collect::<Vec<Signature>>();

                        if let Some(pos) = slot_signatures.iter().position(|&x| x == until) {
                            slot_signatures = slot_signatures.split_off(pos);
                        }

                        (slot, slot_signatures.into_iter().collect::<HashSet<_>>())
                    }
                }
            }
        };

        // Fetch the list of signatures that affect the given address
        let mut address_signatures = Vec::<(Slot, Signature)>::new();
        let mut signatures = if slot < first_available_block {
            Vec::new()
        } else {
            let block = self
                .confirmed_blocks
                .get(&slot, state)
                .unwrap_infallible()
                .ok_or_else(|| create_client_error("Block not found"))?;

            block
                .transactions
                .into_iter()
                .filter_map(|tx| {
                    tx.transaction
                        .message
                        .account_keys
                        .contains(&address.into())
                        .then_some((slot, tx.transaction.signatures.first().copied().unwrap()))
                })
                .rev()
                .collect()
        };

        if let Some(excluded_signatures) = before_excluded_signatures.take() {
            address_signatures.extend(
                signatures
                    .into_iter()
                    .filter(|(_, signature)| !excluded_signatures.contains(signature)),
            );
        } else {
            address_signatures.append(&mut signatures);
        }

        let mut iterator = (lowest_slot..=slot).rev().flat_map(|slot| {
            let Some(block) = self.confirmed_blocks.get(&slot, state).unwrap_infallible() else {
                return Vec::new();
            };
            block
                .transactions
                .into_iter()
                .filter_map(|tx| {
                    tx.transaction
                        .message
                        .account_keys
                        .contains(&address.into())
                        .then_some((slot, tx.transaction.signatures.first().copied().unwrap()))
                })
                .rev()
                .collect::<Vec<_>>()
        });

        while address_signatures.len() < limit {
            let Some((slot, signature)) = iterator.next() else {
                break;
            };

            address_signatures.push((slot, signature));
        }

        let infos = address_signatures
            .into_iter()
            .filter(|(_, signature)| !until_excluded_signatures.contains(signature))
            .take(limit)
            .unique()
            .map(|(slot, signature)| {
                let transaction_status =
                    self.get_transaction_with_commitment(signature, commitment_level, state);
                let err = transaction_status.and_then(|status| {
                    status
                        .tx_with_meta
                        .get_status_meta()
                        .and_then(|meta| meta.status.err())
                });
                let block_time = self
                    .get_block_time(slot, state)
                    .expect("Failed to get block time");
                ConfirmedTransactionStatusWithSignature {
                    signature,
                    slot,
                    err,
                    block_time,
                    memo: None,
                }
                .into()
            })
            .collect();

        Ok(infos)
    }

    #[rpc_method(name = "getFirstAvailableBlock")]
    pub fn get_first_available_block(&self, state: &mut ApiStateAccessor<S>) -> RpcResult<Slot> {
        self.confirmed_block_slots
            .get(0, state)
            .unwrap_infallible()
            .ok_or_else(|| create_server_error("First available block not found"))
    }

    #[rpc_method(name = "getLatestBlockhash")]
    pub fn get_latest_blockhash(
        &self,
        _config: Option<RpcContextConfig>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Response<RpcBlockhash>> {
        let queue = self
            .blockhash_queue
            .get(state)
            .unwrap_infallible()
            .ok_or_else(|| create_server_error("Failed to get blockhash_queue"))?;

        // `BlockhashQueue::last_hash()` can panic if no block hash is set.
        // Since there's no method to check its size, we need to handle any
        // potential panic when it occurs.
        let blockhash = catch_unwind(|| queue.last_hash())
            .map_err(|_| create_server_error("blockhash_queue is empty"))?;

        let current_block_height = self
            .epoch_info
            .get(state)
            .unwrap_infallible()
            .ok_or_else(|| create_server_error("Epoch info not found"))?
            .block_height;

        let Some(age) = queue.get_hash_age(&blockhash) else {
            return Err(create_server_error("Failed to get blockhash age"));
        };

        self.to_response(
            RpcBlockhash {
                blockhash: blockhash.to_string(),
                last_valid_block_height: current_block_height + MAX_PROCESSING_AGE as u64 - age,
            },
            state,
        )
    }

    #[rpc_method(name = "isBlockhashValid")]
    pub fn is_blockhash_valid(
        &self,
        blockhash: String,
        _config: Option<RpcContextConfig>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Response<bool>> {
        let hash =
            Hash::from_str(&blockhash).map_err(|_| create_client_error("Invalid blockhash"))?;

        let queue = self
            .blockhash_queue
            .get(state)
            .unwrap_infallible()
            .ok_or_else(|| create_server_error("Failed to get blockhash_queue"))?;

        self.to_response(
            queue.is_hash_valid_for_age(&hash, MAX_PROCESSING_AGE),
            state,
        )
    }

    // Agave get_fee_for_message reference:
    // https://github.com/anza-xyz/agave/blob/ec9bd798492c3b15d62942f2d9b5923b99042350/rpc/src/rpc.rs#L4033
    #[rpc_method(name = "getFeeForMessage")]
    pub fn get_fee_for_message(
        &self,
        data: String,
        _config: Option<RpcContextConfig>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Response<Option<u64>>> {
        let reserved_account_keys: HashSet<Pubkey> = self
            .reserved_account_keys
            .get(state)
            .unwrap_infallible()
            .ok_or_else(|| create_server_error("Reserved account keys not found"))?
            .into_iter()
            .collect();

        let (_, message) =
            decode_and_deserialize::<Message>(data, TransactionBinaryEncoding::Base64)?;

        let sanitized_message =
            SanitizedMessage::try_from_legacy_message(message, &reserved_account_keys).map_err(
                |e| {
                    create_client_error(&format!(
                        "Failed to convert message into sanitized message: {e}"
                    ))
                },
            )?;

        let lamports_per_signature = {
            let blockhash_queue = self
                .blockhash_queue
                .get(state)
                .unwrap_infallible()
                .ok_or_else(|| create_server_error("Blockhash queue is not found"))?;
            blockhash_queue
                .get_lamports_per_signature(sanitized_message.recent_blockhash())
                .ok_or_else(|| {
                    create_server_error(
                        "Failed to get lamports per signature value from blockhash queue",
                    )
                })?
        };

        let feature_set = self
            .feature_set
            .get(state)
            .unwrap_infallible()
            .ok_or_else(|| create_server_error("Feature set is not found"))?;

        let fee_struct = self
            .fee_structure
            .get(state)
            .unwrap_infallible()
            .ok_or_else(|| create_server_error("Failed to get fee_structure"))?;

        self.to_response(
            Some(
                fee_struct.calculate_fee(
                    &sanitized_message,
                    lamports_per_signature,
                    &process_compute_budget_instructions(
                        sanitized_message.program_instructions_iter(),
                    )
                    .unwrap_or_default()
                    .into(),
                    feature_set
                        .is_active(&include_loaded_accounts_data_size_in_fee_calculation::id()),
                    feature_set.is_active(&remove_rounding_in_fee_calculation::id()),
                ),
            ),
            state,
        )
    }

    #[rpc_method(name = "getRecentPrioritizationFees")]
    pub fn get_recent_prioritization_fees(
        &self,
        _pubkeys: Option<Vec<SolanaAddress>>,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Vec<RpcPrioritizationFee>> {
        // TODO: We might want to track this in the future, for now it will suffice to return an
        // empty vec since we only have one sequencer/node running anyway, so transactions are
        // served in a FIFO manner.
        let slot = self.get_slot(None, state)?;
        Ok(vec![RpcPrioritizationFee {
            slot,
            prioritization_fee: 0,
        }])
    }

    // ------ SVM ENDPOINTS ------

    #[rpc_method(name = "net_version")]
    pub fn net_version(&self, state: &mut ApiStateAccessor<S>) -> RpcResult<String> {
        // TODO @ygao: why are we using chain ID as the "net version"?
        // Network ID is the same as chain ID for most networks
        let chain_id = self
            .creation_time
            .get(state)
            .unwrap_infallible()
            .expect("creation time must be set at genesis");

        Ok(chain_id.to_string())
    }

    /// Queries the state of the module.
    #[rpc_method(name = "queryModuleState")]
    pub fn query_module_state(&self, _state: &mut ApiStateAccessor<S>) -> RpcResult<String> {
        Ok(String::from("Hello"))
    }

    // #[rpc_method(name = "updateFeatureSet")]
    // pub fn update_feature_set(
    //     &self,
    //     active_features: HashMap<Pubkey, Slot>,
    //     inactive_features: HashSet<Pubkey>,
    //     state: &mut ApiStateAccessor<S>,
    // ) -> RpcResult<()> {
    //     // TODO: this RPC endpoint should be permissioned
    //
    //     if active_features
    //         .keys()
    //         .any(|k| inactive_features.contains(k))
    //     {
    //         return Err(create_client_error(
    //             "Active features and inactive features should not overlap",
    //         ));
    //     }
    //
    //     let mut feature_set = self.feature_set.borrow_mut(state).unwrap_infallible();
    //
    //     feature_set.as_mut().map(|feature_set| {
    //         for (feature_id, slot) in active_features {
    //             feature_set.activate(&feature_id, slot);
    //         }
    //         for feature_id in inactive_features {
    //             feature_set.deactivate(&feature_id);
    //         }
    //     });
    //
    //     Ok(())
    // }
}

// Helper functions
impl<S: Spec> SVM<S>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    fn encode_accounts<I>(
        &self,
        accounts: I,
        config: Option<RpcAccountInfoConfig>,
    ) -> Vec<RpcKeyedAccount>
    where
        I: IntoIterator<Item = (Pubkey, Account)>,
    {
        let (encoding, data_slice) = match config {
            Some(RpcAccountInfoConfig {
                encoding,
                data_slice,
                ..
            }) => (
                encoding.unwrap_or(UiAccountEncoding::Base64Zstd),
                data_slice,
            ),
            None => (UiAccountEncoding::Base64Zstd, None),
        };

        let accounts_info: Vec<RpcKeyedAccount> = accounts
            .into_iter()
            .map(|(pubkey, account)| RpcKeyedAccount {
                pubkey: pubkey.to_string(),
                account: UiAccount::encode(&pubkey, &account, encoding, None, data_slice),
            })
            .collect();

        accounts_info
    }

    /// Retrieves all accounts owned by specific public key from `SVM` state
    fn get_accounts_for_owner(
        &self,
        owner: &Pubkey,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Vec<Pubkey>> {
        let pubkeys = self
            .owner_index
            .get(owner, state)
            .unwrap_infallible()
            .unwrap_or_default();

        Ok(pubkeys.into_iter().collect())
    }

    pub fn get_account_data(
        &self,
        pubkey: &Pubkey,
        state: &mut ApiStateAccessor<S>,
    ) -> Option<Account> {
        self.accounts
            .get(pubkey, state)
            .unwrap_infallible()
            .map(|mut account| {
                account.lamports = self.get_bank_balance(pubkey, state).unwrap_or_default();
                account
            })
    }

    fn get_accounts_data<I>(
        &self,
        pubkeys: I,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Vec<(Pubkey, Account)>>
    where
        I: IntoIterator<Item = Pubkey>,
    {
        Ok(pubkeys
            .into_iter()
            .filter_map(|key| self.get_account_data(&key, state).map(|acc| (key, acc)))
            .collect())
    }

    fn get_ata_for_single_mint(
        &self,
        owner: &Pubkey,
        mint_str: &str,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Vec<(Pubkey, Account)>> {
        let mint_addr = Pubkey::from_str(mint_str)
            .map_err(|e| create_client_error(&format!("Invalid pubkey: {e}")))?;
        let mint_acc = self.get_account_data(&mint_addr, state).ok_or_else(|| {
            create_client_error(&format!("Unable to find account with address {mint_addr}"))
        })?;

        let program_id = &mint_acc.owner;
        if !is_mint_account(program_id, &mint_acc) {
            return Err(create_client_error(
                "Invalid mint account with address {mint_addr}",
            ));
        }

        let ata = get_associated_token_address_with_program_id(owner, &mint_addr, program_id);
        let ata_acc = self.get_account_data(&ata, state).ok_or_else(|| {
            create_client_error(&format!("Unable to find account with address {mint_addr}"))
        })?;
        Ok(vec![(ata, ata_acc)])
    }

    fn get_atas_for_spl_program(
        &self,
        owner: &Pubkey,
        program_str: &str,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Vec<(Pubkey, Account)>> {
        let program_id = Pubkey::from_str(program_str)
            .map_err(|e| create_client_error(&format!("Invalid pubkey: {e}")))?;

        if !is_known_spl_token_id(&program_id) {
            return Err(create_client_error(
                "Program ID needs to be SPL token or SPL token 2022",
            ));
        }

        let owned_accounts = self.get_accounts_for_owner(&program_id, state)?;

        let atas: Vec<Pubkey> = owned_accounts
            .into_iter()
            .filter_map(|addr| {
                let acc = self.accounts.get(&addr, state).unwrap_infallible()?;
                // SPL (and SPL 2022) can own different types of accounts
                // if it's a mint account, check if the user has an ATA for the mint
                if is_mint_account(&program_id, &acc) {
                    Some(get_associated_token_address_with_program_id(
                        owner,
                        &addr,
                        &program_id,
                    ))
                } else {
                    None
                }
            })
            .collect();

        self.get_accounts_data(atas, state)
    }

    /// Extracts outputs from sanitized execution results
    // The reference for basis of this functions logic is taken from Agave
    // https://github.com/nitro-svm/agave/blob/72528b5e2445fc87e9e0c651a2ad2134fe41d71c/runtime/src/bank.rs#L3345
    // and from https://github.com/nitro-svm/agave/blob/ee7e23fe7c16c50293c7b893cddb7c05754ad767/rpc/src/rpc.rs#L3957
    fn get_simulation_output(
        &self,
        simulation_output: LoadAndExecuteSanitizedTransactionsOutput,
        config: Option<RpcSimulateTransactionConfig>,
        sanitized_tx: SanitizedTransaction,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<RpcSimulateTransactionResult> {
        let number_of_accounts = sanitized_tx.message().account_keys().len();

        // Check if result vectors have expected length
        if simulation_output.loaded_transactions.len() > 1 {
            return Err(create_server_error(&format!(
                "Loaded transactions length is expected to be 1 but it is {}",
                simulation_output.loaded_transactions.len()
            )));
        } else if simulation_output.execution_results.len() > 1 {
            return Err(create_server_error(&format!(
                "Execution results length is expected to be 1 but it is {}",
                simulation_output.execution_results.len()
            )));
        }

        // Here is the same, according to design only one execution result is expected, as we pass a single transaction into simulate_transaction
        let execution_result = simulation_output
            .execution_results
            .into_iter()
            .next()
            .unwrap_or(TransactionExecutionResult::NotExecuted(
                TransactionError::InvalidProgramForExecution,
            ));
        let flattened_result = execution_result.flattened_result();

        // Get post simulation accounts information from sanitized output
        // In simulation output by design we expect vector of loaded transactions contain a single transaction, so we call `.next()`
        let Some(Ok(loaded_tx)) = simulation_output.loaded_transactions.first() else {
            return Err(create_server_error(&format!(
                "Failed to get loaded transaction: {:?} - provided accounts: {:?}",
                simulation_output.loaded_transactions,
                sanitized_tx.message().account_keys()
            )));
        };

        let post_simulation_accounts: Vec<(Pubkey, Account)> = loaded_tx
            .accounts
            .clone()
            .into_iter()
            .take(number_of_accounts)
            .map(|(pubkey, account)| (pubkey, account.into()))
            .collect::<Vec<_>>();

        let accounts = if let Some(config_accounts) = &config.and_then(|config| config.accounts) {
            let accounts_encoding = config_accounts
                .encoding
                .unwrap_or(UiAccountEncoding::Base64);

            if accounts_encoding == UiAccountEncoding::Binary
                || accounts_encoding == UiAccountEncoding::Base58
            {
                return Err(create_client_error("base58 encoding not supported"));
            }

            if config_accounts.addresses.len() > number_of_accounts {
                return Err(create_client_error(&format!(
                    "Too many accounts provided; max {number_of_accounts}"
                )));
            }

            if flattened_result.is_err() {
                Some(vec![None; config_accounts.addresses.len()])
            } else {
                let post_simulation_accounts_map =
                    HashMap::<Pubkey, Account>::from_iter(post_simulation_accounts.iter().cloned());

                Some(
                    config_accounts
                        .addresses
                        .iter()
                        .map(|address_str| {
                            let pubkey = Pubkey::from_str(address_str).map_err(|e| {
                                create_client_error(&format!("Invalid pubkey: {e}"))
                            })?;
                            let account = post_simulation_accounts_map.get(&pubkey).cloned().or_else(|| {
                                self.accounts
                                    .get(&pubkey, state)
                                    .unwrap_infallible()
                            });

                            let Some(account) = account else {
                                return Ok(None);
                            };

                            let encoded = if is_known_spl_token_id(&account.owner) {
                                let additional_data = get_token_account_mint(account.data())
                                    .and_then(|mint_pubkey| {
                                        post_simulation_accounts_map.get(&mint_pubkey).cloned().or_else(
                                            || {
                                                self.accounts
                                                    .get(&mint_pubkey, state)
                                                    .unwrap_infallible()
                                            },
                                        )
                                    })
                                    .and_then(|mint_account| {
                                        get_additional_mint_data(0, mint_account.data()).ok()
                                    })
                                    .map(|data| AccountAdditionalDataV2 {
                                        spl_token_additional_data: Some(data),
                                    });

                                Ok(UiAccount::encode(
                                    &pubkey,
                                    &account,
                                    UiAccountEncoding::JsonParsed,
                                    additional_data,
                                    None,
                                ))
                            } else if (accounts_encoding == UiAccountEncoding::Binary || accounts_encoding == UiAccountEncoding::Base58)
                                && account.data().len() > MAX_BASE58_BYTES
                            {
                                Err(create_client_error(&format!("Encoded binary (base 58) data should be less than {MAX_BASE58_BYTES} bytes, please use Base64 encoding.")))
                            } else {
                                Ok(UiAccount::encode(
                                    &pubkey, &account, accounts_encoding, None, None,
                                ))
                            }?;

                            Ok(Some(encoded))
                        })
                        .collect::<RpcResult<Vec<_>>>()?,
                )
            }
        } else {
            None
        };

        // get details of transaction execution from execution result
        let (logs, return_data, inner_instructions) = match execution_result {
            TransactionExecutionResult::Executed { details, .. } => (
                details.log_messages,
                details.return_data,
                details.inner_instructions,
            ),
            TransactionExecutionResult::NotExecuted(_) => (None, None, None),
        };
        let logs = logs.unwrap_or_default();
        let mut timings = ExecuteTimings::default();
        timings.accumulate(&simulation_output.execute_timings);

        let units_consumed =
            timings
                .details
                .per_program_timings
                .iter()
                .fold(0, |acc: u64, (_, program_timing)| {
                    acc.saturating_add(program_timing.accumulated_units)
                        .saturating_add(program_timing.total_errored_units)
                });

        let account_keys = sanitized_tx.message().account_keys();
        let inner_instructions = inner_instructions.map(|info| {
            map_inner_instructions(info)
                .map(|converted| UiInnerInstructions::parse(converted, &account_keys))
                .collect()
        });

        Ok(RpcSimulateTransactionResult {
            logs: Some(logs),
            accounts,
            units_consumed: Some(units_consumed),
            return_data: return_data.map(|return_data| return_data.into()),
            inner_instructions,
            err: flattened_result.err(),
            replacement_blockhash: None,
        })
    }

    fn get_transaction_with_commitment(
        &self,
        signature: Signature,
        commitment: CommitmentLevel,
        state: &mut ApiStateAccessor<S>,
    ) -> Option<ConfirmedTransactionWithStatusMeta> {
        if commitment == CommitmentLevel::Processed {
            let processed_txs: Vec<ExecutedTransactionData> =
                self.processed_transactions.collect_infallible(state);

            if let Some(found) = processed_txs
                .iter()
                .find(|tx| tx.transaction.signatures[0] == signature)
            {
                let slot = self.get_slot(None, state).expect("Failed to get slot");
                return Some(ConfirmedTransactionWithStatusMeta {
                    slot,
                    block_time: None,
                    tx_with_meta: TransactionWithStatusMeta::Complete(found.clone().into()),
                });
            }
        }

        self.confirmed_transactions
            .get(&signature, state)
            .unwrap_infallible()
            .map(|confirmed| ConfirmedTransactionWithStatusMeta {
                slot: confirmed.slot,
                block_time: confirmed.block_time,
                tx_with_meta: TransactionWithStatusMeta::Complete(confirmed.transaction.into()),
            })
    }
}

pub fn create_server_error(message: &str) -> ErrorObjectOwned {
    ErrorObjectOwned::owned(ErrorCode::InternalError.code(), message, Some(()))
}

pub fn create_client_error(message: &str) -> ErrorObjectOwned {
    ErrorObjectOwned::owned(ErrorCode::InvalidRequest.code(), message, Some(()))
}

fn get_additional_mint_data(timestamp: i64, data: &[u8]) -> RpcResult<SplTokenAdditionalData> {
    StateWithExtensions::<spl_token_2022::state::Mint>::unpack(data)
        .map_err(|_| create_client_error("Invalid param: Token mint could not be unpacked"))
        .map(|mint| {
            let interest_bearing_config = mint
                .get_extension::<InterestBearingConfig>()
                .map(|x| (*x, timestamp))
                .ok();
            SplTokenAdditionalData {
                decimals: mint.base.decimals,
                interest_bearing_config,
            }
        })
}

fn is_mint_account(program_id: &Pubkey, account: &Account) -> bool {
    let expected_size = if program_id == &spl_token::id() {
        spl_token::state::Mint::LEN
    } else if program_id == &spl_token_2022::id() {
        spl_token_2022::state::Mint::LEN
    } else {
        return false;
    };

    expected_size == account.data.len()
}

fn verify_signature(input: &str) -> RpcResult<Signature> {
    input
        .parse()
        .map_err(|e| create_client_error(&format!("Invalid param: {e:?}")))
}

fn verify_and_parse_signatures_for_address_params(
    before: Option<String>,
    until: Option<String>,
    limit: Option<usize>,
) -> RpcResult<(Option<Signature>, Option<Signature>, usize)> {
    let before = before
        .map(|ref before| verify_signature(before))
        .transpose()?;
    let until = until.map(|ref until| verify_signature(until)).transpose()?;
    let limit = limit.unwrap_or(MAX_GET_CONFIRMED_SIGNATURES_FOR_ADDRESS2_LIMIT);

    if limit == 0 || limit > MAX_GET_CONFIRMED_SIGNATURES_FOR_ADDRESS2_LIMIT {
        return Err(create_client_error(&format!(
            "Invalid limit; max {MAX_GET_CONFIRMED_SIGNATURES_FOR_ADDRESS2_LIMIT}"
        )));
    }
    Ok((before, until, limit))
}
