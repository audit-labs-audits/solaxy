use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use solana_program::fee_calculator::FeeRateGovernor;
use solana_sdk::{
    account::{Account, AccountSharedData},
    clock::{Slot, UnixTimestamp},
    pubkey::Pubkey,
    signature::Keypair,
    transaction::{Transaction, TransactionError},
};
use solana_svm::transaction_results::{TransactionExecutionDetails, TransactionExecutionResult};
use svm_types::SolanaAddress;

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccountProxy {
    pub lamports: u64,
    pub data: Vec<u8>,
    pub owner: SolanaAddress,
    pub executable: bool,
    pub rent_epoch: u64,
}

impl From<AccountProxy> for Account {
    fn from(proxy: AccountProxy) -> Self {
        let AccountProxy {
            lamports,
            data,
            owner,
            executable,
            rent_epoch,
        } = proxy;
        Account {
            lamports,
            data,
            owner: owner.into(),
            executable,
            rent_epoch,
        }
    }
}

impl From<&AccountProxy> for Account {
    fn from(proxy: &AccountProxy) -> Self {
        proxy.clone().into()
    }
}

impl From<Account> for AccountProxy {
    fn from(account: Account) -> Self {
        let Account {
            lamports,
            data,
            owner,
            executable,
            rent_epoch,
        } = account;
        AccountProxy {
            lamports,
            data,
            owner: owner.into(),
            executable,
            rent_epoch,
        }
    }
}

impl From<AccountSharedData> for AccountProxy {
    fn from(account_shared_data: AccountSharedData) -> Self {
        let account: Account = account_shared_data.into();
        account.into()
    }
}

/// Duplicate implementation of original `FeeRateGovernor` in Agave because
/// the original code marks a few parameters as skipped, so they're not included
/// while serializing or deserializing.
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FeeRateGovernorWrapper {
    lamports_per_signature: u64,
    inner: FeeRateGovernor,
}

impl From<FeeRateGovernor> for FeeRateGovernorWrapper {
    fn from(fee_rate_governor: FeeRateGovernor) -> Self {
        FeeRateGovernorWrapper {
            lamports_per_signature: fee_rate_governor.lamports_per_signature,
            inner: fee_rate_governor,
        }
    }
}

impl From<&FeeRateGovernor> for FeeRateGovernorWrapper {
    fn from(fee_rate_governor: &FeeRateGovernor) -> Self {
        FeeRateGovernorWrapper {
            lamports_per_signature: fee_rate_governor.lamports_per_signature,
            inner: fee_rate_governor.clone(),
        }
    }
}

impl From<FeeRateGovernorWrapper> for FeeRateGovernor {
    fn from(wrapper: FeeRateGovernorWrapper) -> Self {
        FeeRateGovernor {
            lamports_per_signature: wrapper.lamports_per_signature,
            ..wrapper.inner
        }
    }
}

impl From<&FeeRateGovernorWrapper> for FeeRateGovernor {
    fn from(wrapper: &FeeRateGovernorWrapper) -> Self {
        FeeRateGovernor {
            lamports_per_signature: wrapper.lamports_per_signature,
            ..wrapper.clone().inner
        }
    }
}

/// Create a proxy since Agave's `Keypair` doesn't implement Clone or serde
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeypairProxy(#[serde(with = "BigArray")] [u8; 64]);

impl KeypairProxy {
    pub fn new(bytes: [u8; 64]) -> Self {
        KeypairProxy(bytes)
    }
}

impl From<Keypair> for KeypairProxy {
    fn from(keypair: Keypair) -> Self {
        KeypairProxy(keypair.to_bytes())
    }
}

impl From<KeypairProxy> for Keypair {
    fn from(keypair: KeypairProxy) -> Self {
        Keypair::from_bytes(&keypair.0).expect("Failed to parse bytes for keypair")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfirmedTransactionDetails {
    pub slot: Slot,
    pub transaction: ExecutedTransactionData,
    pub block_time: Option<UnixTimestamp>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfirmedBlockDetails {
    pub previous_blockhash: String,
    pub blockhash: String,
    pub parent_slot: Slot,
    pub transactions: Vec<ExecutedTransactionData>,
    pub block_time: Option<UnixTimestamp>,
    pub block_height: Option<u64>,
}

#[cfg(feature = "native")]
impl From<ConfirmedBlockDetails> for solana_transaction_status::ConfirmedBlock {
    fn from(confirmed_block: ConfirmedBlockDetails) -> Self {
        use solana_transaction_status::TransactionWithStatusMeta;

        let transactions_with_meta = confirmed_block
            .transactions
            .into_iter()
            .map(|t| TransactionWithStatusMeta::Complete(t.into()))
            .collect::<Vec<_>>();

        Self {
            previous_blockhash: confirmed_block.previous_blockhash,
            blockhash: confirmed_block.blockhash,
            parent_slot: confirmed_block.parent_slot,
            transactions: transactions_with_meta,
            rewards: Default::default(),
            num_partitions: None,
            block_time: confirmed_block.block_time,
            block_height: confirmed_block.block_height,
        }
    }
}

/// A proxy for [`TransactionExecutionResult`] to enable serialization and deserialization
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransactionExecutionResultProxy {
    Executed {
        details: TransactionExecutionDetails,
        programs_modified_by_tx: Vec<Pubkey>,
    },
    NotExecuted(TransactionError),
}

impl From<TransactionExecutionResultProxy> for TransactionExecutionResult {
    fn from(proxy: TransactionExecutionResultProxy) -> Self {
        match proxy {
            TransactionExecutionResultProxy::Executed {
                details,
                programs_modified_by_tx,
            } => TransactionExecutionResult::Executed {
                details,
                programs_modified_by_tx: programs_modified_by_tx
                    .into_iter()
                    .map(|key| (key, Arc::new(Default::default())))
                    .collect(),
            },
            TransactionExecutionResultProxy::NotExecuted(err) => {
                TransactionExecutionResult::NotExecuted(err)
            }
        }
    }
}

impl From<TransactionExecutionResult> for TransactionExecutionResultProxy {
    fn from(result: TransactionExecutionResult) -> Self {
        match result {
            TransactionExecutionResult::Executed {
                details,
                programs_modified_by_tx,
            } => TransactionExecutionResultProxy::Executed {
                details,
                programs_modified_by_tx: programs_modified_by_tx.keys().copied().collect(),
            },
            TransactionExecutionResult::NotExecuted(err) => {
                TransactionExecutionResultProxy::NotExecuted(err)
            }
        }
    }
}

/// A wrapper around [`Transaction`] and [`TransactionExecutionResult`] to store transaction data.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutedTransactionData {
    pub transaction: Transaction,
    pub execution_result: TransactionExecutionResultProxy,
}

impl Default for ExecutedTransactionData {
    fn default() -> Self {
        Self {
            transaction: Transaction::default(),
            // Default to `AccountNotFound` error - could be any other error as well
            execution_result: TransactionExecutionResultProxy::NotExecuted(
                TransactionError::AccountNotFound,
            ),
        }
    }
}

impl ExecutedTransactionData {
    /// Create a new instance of [`ExecutedTransactionData`].
    pub fn new(transaction: Transaction, execution_result: TransactionExecutionResult) -> Self {
        Self {
            transaction,
            execution_result: execution_result.into(),
        }
    }
}

#[cfg(feature = "native")]
impl From<ExecutedTransactionData>
    for solana_transaction_status::VersionedTransactionWithStatusMeta
{
    fn from(data: ExecutedTransactionData) -> Self {
        use solana_sdk::transaction::VersionedTransaction;
        use solana_transaction_status::{map_inner_instructions, TransactionStatusMeta};

        let transaction = VersionedTransaction::from(data.transaction.clone());
        let result: TransactionExecutionResult = data.execution_result.into();
        let flattened_result = result.flattened_result();

        let Some(details) = result.details() else {
            return Self {
                transaction,
                meta: TransactionStatusMeta {
                    status: flattened_result,
                    ..TransactionStatusMeta::default()
                },
            };
        };

        let meta = TransactionStatusMeta {
            status: details.status.clone(),
            fee: details.fee_details.total_fee(),
            inner_instructions: details
                .inner_instructions
                .clone()
                .map(|i| map_inner_instructions(i).collect()),
            log_messages: details.log_messages.clone(),
            return_data: details.return_data.clone(),
            compute_units_consumed: Some(details.executed_units),
            ..TransactionStatusMeta::default()
        };
        // TODO: Implement the following fields
        // pre_balances: todo!(),
        // post_balances: todo!(),
        // pre_token_balances: todo!(),
        // post_token_balances: todo!(),
        // rewards: todo!(),
        // loaded_addresses: todo!(),

        Self { transaction, meta }
    }
}
