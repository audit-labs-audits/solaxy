#[cfg(feature = "native")]
use solana_sdk::clock::UnixTimestamp;
use solana_sdk::{epoch_info::EpochInfo, message::Message};
use sov_address::FromVmAddress;
use sov_modules_api::{prelude::UnwrapInfallible, BlockHooks, Spec, StateCheckpoint, Storage};
use svm_types::SolanaAddress;

use crate::{wrappers::ExecutedTransactionData, SVM};
#[cfg(feature = "native")]
use crate::{ConfirmedBlockDetails, ConfirmedTransactionDetails};

impl<S> BlockHooks for SVM<S>
where
    S: Spec,
    S::Address: FromVmAddress<SolanaAddress>,
{
    type Spec = S;

    fn begin_rollup_block_hook(
        &mut self,
        _pre_state_user_root: &<S::Storage as Storage>::Root,
        _state: &mut StateCheckpoint<S>,
    ) {
    }

    fn end_rollup_block_hook(&mut self, state: &mut StateCheckpoint<S>) {
        let processed_transactions: Vec<ExecutedTransactionData> =
            self.processed_transactions.collect_infallible(state);

        let mut epoch_info = self
            .epoch_info
            .get(state)
            .unwrap_infallible()
            .unwrap_or(EpochInfo {
                epoch: 0,
                slot_index: 0,
                block_height: 0,
                absolute_slot: 0,
                slots_in_epoch: 0,
                transaction_count: Some(processed_transactions.len() as u64),
            });

        let mut blockhash_queue = self
            .blockhash_queue
            .get(state)
            .unwrap_infallible()
            .unwrap_or_default();

        let count = processed_transactions.len();

        // Register new hash into blockhash queue
        let lamports_per_signature = self
            .fee_structure
            .get(state)
            .unwrap_infallible()
            .unwrap_or_default()
            .lamports_per_signature;
        // While agave captures more details in the blockhash during each `tick`, we only generate
        // hashes on the end of a block so we generate the hash from the previous blockhash and the
        // hashes of the transactions in the block.
        let latest_blockhash = blockhash_queue.last_hash();
        let hashes = Vec::from_iter(
            std::iter::once(latest_blockhash).chain(
                processed_transactions
                    .iter()
                    .map(|tx| Message::hash_raw_message(&tx.transaction.message_data())),
            ),
        );
        let blockhash =
            solana_sdk::hash::hashv(&hashes.iter().map(|h| h.as_ref()).collect::<Vec<_>>());

        blockhash_queue.register_hash(&blockhash, lamports_per_signature);
        self.blockhash_queue
            .set(&blockhash_queue, state)
            .unwrap_infallible();
        self.processed_transactions.clear(state).unwrap_infallible();

        #[cfg(feature = "native")]
        self.handle_accessory_state(
            state,
            processed_transactions,
            &epoch_info,
            latest_blockhash,
            blockhash,
            count,
        );

        epoch_info.slot_index += 1;
        epoch_info.absolute_slot += 1;
        epoch_info.slots_in_epoch += 1;
        epoch_info.block_height += 1;
        epoch_info.transaction_count =
            Some(epoch_info.transaction_count.unwrap_or_default() + count as u64);
        self.epoch_info.set(&epoch_info, state).unwrap_infallible();
    }
}

#[cfg(feature = "native")]
impl<S> SVM<S>
where
    S: Spec,
    S::Address: FromVmAddress<SolanaAddress>,
{
    /// Here we handle all the accessory state updates that are needed for the RPC to work correctly.
    fn handle_accessory_state(
        &mut self,
        state: &mut StateCheckpoint<S>,
        processed_transactions: Vec<ExecutedTransactionData>,
        epoch_info: &EpochInfo,
        latest_blockhash: solana_sdk::hash::Hash,
        blockhash: solana_sdk::hash::Hash,
        count: usize,
    ) {
        let mut accessory_state = state.accessory_state();
        let mut transactions = Vec::with_capacity(count);
        let transactions_count = self
            .transactions_count
            .get(&mut accessory_state)
            .unwrap_infallible()
            .unwrap_or_default();

        // Store confirmed transactions
        for processed in processed_transactions {
            let block_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as UnixTimestamp;

            transactions.push(processed.clone());

            self.confirmed_transactions
                .set(
                    &processed.transaction.signatures[0].clone(),
                    &ConfirmedTransactionDetails {
                        slot: epoch_info.slot_index,
                        transaction: processed,
                        block_time: Some(block_time),
                    },
                    &mut accessory_state,
                )
                .unwrap();
        }

        // Store block information
        let slot = epoch_info.absolute_slot;

        let block_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as UnixTimestamp;

        self.confirmed_blocks
            .set(
                &slot,
                &ConfirmedBlockDetails {
                    previous_blockhash: latest_blockhash.to_string(),
                    blockhash: blockhash.to_string(),
                    parent_slot: slot.saturating_sub(1),
                    transactions,
                    block_time: Some(block_time),
                    block_height: Some(epoch_info.block_height),
                },
                &mut accessory_state,
            )
            .unwrap_infallible();
        self.confirmed_block_slots
            .push(&slot, &mut accessory_state)
            .unwrap_infallible();
        self.transactions_count
            .set(&(transactions_count + count as u64), &mut accessory_state)
            .unwrap_infallible();
    }
}
