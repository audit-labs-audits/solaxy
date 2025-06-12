use solana_program::{bpf_loader_upgradeable, sysvar};
use solana_program_runtime::loaded_programs::ProgramCacheEntryType;
use solana_sdk::{
    account::ReadableAccount,
    transaction::{SanitizedTransaction, TransactionError},
};
use solana_svm::{
    account_loader::{TransactionCheckResult, TransactionLoadResult},
    transaction_processor::LoadAndExecuteSanitizedTransactionsOutput,
    transaction_results::TransactionExecutionResult,
};
use tracing::debug;

use crate::{Engine, SVMEngineResult};

#[derive(Debug, thiserror::Error)]
pub enum SVMExecutionError {
    #[error("Loaded Transaction Results and Execution Results vectors do not match in length")]
    /// Loaded Transaction Results and Execution Results vectors do not match in length
    LengthMismatch,

    #[error("Transaction number {0} failed to execute: {1}")]
    /// Transaction number {0} failed to execute: {1}
    FailedExecution(usize, TransactionError),

    #[error("Transaction number {0} failed to load: {1}")]
    /// Transaction number {0} failed to load: {1}
    FailedLoading(usize, TransactionError),

    #[error("Poisoned lock: {0}")]
    /// Poisoned lock {0}
    Poison(String),
}

impl<Storage> Engine<'_, Storage>
where
    Storage: crate::storage::SVMStorage,
{
    pub fn load_and_execute_transaction(
        &self,
        sanitized_txs: SanitizedTransaction,
        check_results: TransactionCheckResult,
    ) -> LoadAndExecuteSanitizedTransactionsOutput {
        let (config, environment) = self.transaction_config_and_environment();

        self.processor.load_and_execute_sanitized_transactions(
            &self.db,
            &[sanitized_txs],
            vec![check_results],
            &environment,
            &config,
        )
    }

    fn commit_transactions(
        &self,
        loaded_transactions: &[TransactionLoadResult],
        execution_results: &[TransactionExecutionResult],
        sanitized_transactions: &[SanitizedTransaction],
    ) -> SVMEngineResult {
        if execution_results.len() != loaded_transactions.len()
            || execution_results.len() != sanitized_transactions.len()
        {
            return Err(SVMExecutionError::LengthMismatch.into());
        }

        let mut cache = self.processor.program_cache.write().map_err(|_| {
            SVMExecutionError::Poison("Failed to acquire program cache write lock".to_string())
        })?;

        // execution_results and loaded_transactions are vectors of results from transaction execution
        // both vectors have to be of the same size, but to be safe accessing items by indexes is avoided
        for (i, (execution_result, loaded_tx_result, sanitized_transaction)) in execution_results
            .iter()
            .zip(loaded_transactions.iter())
            .zip(sanitized_transactions.iter())
            .map(|((a, b), c)| (a, b, c))
            .enumerate()
        {
            let (details, programs_modified_by_tx) = match execution_result {
                TransactionExecutionResult::Executed {
                    details,
                    programs_modified_by_tx,
                } => {
                    if let Err(err) = &details.status {
                        return Err(SVMExecutionError::FailedExecution(i, err.clone()).into());
                    }
                    (details, programs_modified_by_tx)
                }
                TransactionExecutionResult::NotExecuted(e) => {
                    return Err(SVMExecutionError::FailedExecution(i, e.clone()).into());
                }
            };

            let loaded_tx = match loaded_tx_result {
                Ok(loaded_tx) => loaded_tx,
                Err(e) => {
                    return Err(SVMExecutionError::FailedLoading(i, e.clone()).into());
                }
            };

            let message = sanitized_transaction.message();

            // LoadedTransaction contains details of the transaction after its execution including loaded accounts
            // Persist account updates in SVM accounts from LoadedTransaction
            for (pubkey, account) in &loaded_tx.accounts {
                let is_non_executable_account = !account.executable();

                let is_not_sysvar_account = !sysvar::check_id(account.owner());

                let is_upgradeable_with_data =
                    bpf_loader_upgradeable::check_id(account.owner()) && !account.data().is_empty();

                let is_data_account = is_non_executable_account || is_upgradeable_with_data;

                let is_writable = message
                    .account_keys()
                    .iter()
                    .enumerate()
                    .any(|(index, key)| key == pubkey && message.is_writable(index));

                if is_data_account && is_not_sysvar_account && is_writable {
                    // Purge zero lamport accounts
                    if account.lamports() == 0 {
                        debug!("Commit Transactions: Purged account with pubkey {pubkey}");
                        self.remove_account(pubkey)?;
                    } else {
                        if self.get_account(pubkey)?.is_none() {
                            self.set_owner_index(account.owner(), pubkey)?;
                        }
                        self.set_account(pubkey, account.clone().into())?;
                    }
                }
            }

            if details.status.is_ok() && !programs_modified_by_tx.is_empty() {
                // Assign each modified and newly created program to the cache.
                for program_id in programs_modified_by_tx.keys() {
                    // On Solana, when a program owned by `bpf_upgradeable` is closed, the lamports from its `program_data` account are
                    // transferred to the destination account, and the `program_data` account is purged. However, the actual program
                    // account remains intact and is not modified as expected. To ensure that the system cannot interact with a closed
                    // program, it must be removed from the processor cache.
                    if let Some(program_cache) = programs_modified_by_tx.get(program_id) {
                        if matches!(program_cache.program, ProgramCacheEntryType::Closed) {
                            cache.remove_programs(vec![*program_id].into_iter());
                        }
                    }
                    self.load_program_to_cache(program_id, &mut cache, false);
                }
            }
        }

        Ok(())
    }

    pub fn load_execute_and_commit_transactions(
        &self,
        sanitized_txs: &[SanitizedTransaction],
        check_results: Vec<TransactionCheckResult>,
    ) -> SVMEngineResult<Vec<TransactionExecutionResult>> {
        let mut results = Vec::with_capacity(sanitized_txs.len());
        for (sanitized_transaction, check_result) in
            sanitized_txs.iter().cloned().zip(check_results.into_iter())
        {
            let LoadAndExecuteSanitizedTransactionsOutput {
                execution_results,
                loaded_transactions,
                ..
            } = self.load_and_execute_transaction(sanitized_transaction.clone(), check_result);

            debug!("Execution output: {execution_results:?}");

            self.commit_transactions(
                &loaded_transactions,
                &execution_results,
                &[sanitized_transaction],
            )?;

            results.push(execution_results);
        }
        self.update_slot()?;

        Ok(results.into_iter().flatten().collect())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap, HashSet};

    use solana_program::{
        clock::{Epoch, MAX_PROCESSING_AGE, Slot},
        hash::Hash,
        message::{LegacyMessage, Message, MessageHeader, SanitizedMessage},
        pubkey::Pubkey,
        system_instruction, system_program,
    };
    use solana_sdk::{
        account::{Account, AccountSharedData, ReadableAccount},
        fee::FeeDetails,
        reserved_account_keys::ReservedAccountKeys,
        signature::{Keypair, Signature, Signer},
        transaction::{SanitizedTransaction, Transaction, TransactionError},
    };
    use solana_svm::{
        account_loader::LoadedTransaction,
        rollback_accounts::RollbackAccounts,
        transaction_processor::TransactionBatchProcessor,
        transaction_results::{TransactionExecutionDetails, TransactionExecutionResult},
    };

    use crate::{
        Engine, SVMEngineError,
        execution::SVMExecutionError,
        storage::{AccountsDB, SVMStorage, SvmBankForks},
        verification::{get_transaction_check_results, sanitize_and_verify_tx},
    };

    #[test]
    fn test_load_and_execute_transactions() {
        let fee_payer1 = Keypair::new();
        let fee_payer2 = Keypair::new();
        let fee_payer3 = Keypair::new();

        let genesis_accounts: BTreeMap<Pubkey, Account> = BTreeMap::from_iter([
            (
                fee_payer1.pubkey(),
                Account::new(10_000_000, 0, &system_program::id()),
            ),
            (
                fee_payer2.pubkey(),
                Account::new(10_000_000, 0, &system_program::id()),
            ),
        ]);

        let db = AccountsDB::new(genesis_accounts);
        let programs = db.get_program_accounts();
        let processor = TransactionBatchProcessor::<SvmBankForks>::new(
            Slot::default(),
            Epoch::default(),
            HashSet::default(),
        );
        let engine = Engine::builder().db(db).processor(&processor).build();
        engine.initialize_cache();
        engine.initialize_transaction_processor();
        engine.fill_cache(&programs);

        /*
            Note: order of transactions does not matter because they are not commited
            each transaction is executed separately
            tx1 and tx3 are sol transfer transactions
            tx1: Acc1 -> Acc2 1_000_000 sol
            tx3: Acc2 -> Acc1 3_000_000 sol
            tx2 is Acc3 creation transaction
            tx2 Acc1 -> Acc3 2_000_000 for initial balance of Acc3
        */
        let recent_blockhash = Hash::new_unique();
        let tx1 = prepare_sol_transfer_transaction(
            &fee_payer1,
            &fee_payer2.pubkey(),
            1_000_000,
            recent_blockhash,
        );
        let san_tx1 = sanitize_and_verify_tx(&tx1, &ReservedAccountKeys::default().active).unwrap();

        let tx2 =
            prepare_new_account_transaction(&fee_payer1, &fee_payer3, 2_000_000, recent_blockhash);
        let san_tx2 = sanitize_and_verify_tx(&tx2, &ReservedAccountKeys::default().active).unwrap();

        let tx3 = prepare_sol_transfer_transaction(
            &fee_payer2,
            &fee_payer1.pubkey(),
            3_000_000,
            recent_blockhash,
        );
        let san_tx3 = sanitize_and_verify_tx(&tx3, &ReservedAccountKeys::default().active).unwrap();

        let check_results = get_transaction_check_results(
            &[san_tx1.clone(), san_tx2.clone(), san_tx3.clone()],
            MAX_PROCESSING_AGE,
        );

        let txs_execution_output =
            engine.load_and_execute_transaction(san_tx1, check_results[0].clone());

        // transfer 1_000_000 from Acc1 to Acc2
        // Acc1 10_000_000 - 1_000_000 = 9_000_000 - transaction fee
        // Acc2 10_000_000 + 1_000_000 = 11_000_000
        let acc = txs_execution_output.loaded_transactions.first().unwrap();
        let loaded_transaction_0 = acc.clone().unwrap();
        let (_, updated_acc_1) = loaded_transaction_0.accounts.first().unwrap();
        let (_, updated_acc_2) = loaded_transaction_0.accounts.get(1).unwrap();
        let fee = txs_execution_output
            .execution_results
            .first()
            .and_then(|result| result.details())
            .map(|details| details.fee_details.transaction_fee())
            .unwrap();

        assert_eq!(9_000_000 - fee, updated_acc_1.lamports());
        assert_eq!(11_000_000, updated_acc_2.lamports());

        let txs_execution_output =
            engine.load_and_execute_transaction(san_tx2, check_results[1].clone());

        // create Acc3 and transfer 2_000_000 from Acc1
        // Acc1 10_000_000 - 2_000_000 = 8_000_000 - transaction fee
        // Acc3 0 + 2_000_000 = 2_000_000
        let acc = txs_execution_output.loaded_transactions.first().unwrap();
        let loaded_transaction_0 = acc.clone().unwrap();
        let (_, updated_acc_1) = loaded_transaction_0.accounts.first().unwrap();
        let (_, updated_acc_3) = loaded_transaction_0.accounts.get(1).unwrap();
        let fee = txs_execution_output
            .execution_results
            .first()
            .and_then(|result| result.details())
            .map(|details| details.fee_details.transaction_fee())
            .unwrap();

        assert_eq!(8_000_000 - fee, updated_acc_1.lamports());
        assert_eq!(2_000_000, updated_acc_3.lamports());

        let txs_execution_output =
            engine.load_and_execute_transaction(san_tx3, check_results[2].clone());
        // transfer 3_000_000 from Acc2 to Acc1
        // Acc1 10_000_000 + 3_000_000 = 13_000_000
        // Acc2 10_000_000 - 3_000_000 = 7_000_000
        let acc = txs_execution_output.loaded_transactions.first().unwrap();
        let loaded_transaction_0 = acc.clone().unwrap();
        let (_, updated_acc_2) = loaded_transaction_0.accounts.first().unwrap();
        let (_, updated_acc_1) = loaded_transaction_0.accounts.get(1).unwrap();
        let fee = txs_execution_output
            .execution_results
            .first()
            .and_then(|result| result.details())
            .map(|details| details.fee_details.transaction_fee())
            .unwrap();

        assert_eq!(7_000_000 - fee, updated_acc_2.lamports());
        assert_eq!(13_000_000, updated_acc_1.lamports());
    }

    #[test]
    fn test_commit_transactions() {
        let db = AccountsDB::new(BTreeMap::new());
        let programs = db.get_program_accounts();
        let processor = TransactionBatchProcessor::<SvmBankForks>::new(
            Slot::default(),
            Epoch::default(),
            HashSet::default(),
        );
        let engine = Engine::builder().db(db).processor(&processor).build();
        engine.initialize_cache();
        engine.initialize_transaction_processor();
        engine.fill_cache(&programs);

        let sanitized_transaction = prepared_test_sanitized_transaction(None);

        let mut res = engine.commit_transactions(&[], &[], &[]);

        // commit empty transaction and execution results
        assert!(res.is_ok());

        let loaded_transaction = LoadedTransaction {
            accounts: Vec::new(),
            program_indices: Vec::new(),
            fee_details: Default::default(),
            rollback_accounts: RollbackAccounts::FeePayerOnly {
                fee_payer_account: AccountSharedData::default(),
            },
            compute_budget_limits: Default::default(),
            rent: 0,
            rent_debits: Default::default(),
            loaded_accounts_data_size: 0,
        };

        let mut test_loaded_transactions = vec![Ok(loaded_transaction)];

        let execution_result = TransactionExecutionResult::Executed {
            details: TransactionExecutionDetails {
                status: Ok(()),
                log_messages: None,
                inner_instructions: None,
                fee_details: FeeDetails::default(),
                return_data: None,
                executed_units: 0,
                accounts_data_len_delta: 0,
            },
            programs_modified_by_tx: HashMap::default(),
        };

        let mut test_execution_results = vec![execution_result.clone(), execution_result.clone()];

        res = engine.commit_transactions(
            &test_loaded_transactions,
            &test_execution_results,
            &vec![sanitized_transaction.clone()],
        );

        // anyhow errors do not implement PartialEq, so we compare String outputs
        // test case with different vectors lengths
        assert_eq!(
            SVMEngineError::Execution(SVMExecutionError::LengthMismatch).to_string(),
            res.unwrap_err().to_string()
        );

        test_execution_results = vec![TransactionExecutionResult::NotExecuted(
            TransactionError::UnbalancedTransaction,
        )];

        res = engine.commit_transactions(
            &test_loaded_transactions,
            &test_execution_results,
            &vec![sanitized_transaction.clone()],
        );

        // test case with not executed transaction
        assert_eq!(
            SVMEngineError::Execution(SVMExecutionError::FailedExecution(
                0,
                TransactionError::UnbalancedTransaction
            ))
            .to_string(),
            res.unwrap_err().to_string()
        );

        test_execution_results = vec![execution_result];

        res = engine.commit_transactions(
            &test_loaded_transactions,
            &test_execution_results,
            &vec![sanitized_transaction.clone()],
        );

        // testing transactions with empty accounts
        assert!(res.is_ok());

        let account1 = Keypair::new();
        let account1_pubkey = account1.pubkey();

        let account2 = Keypair::new();
        let account2_pubkey = account2.pubkey();

        // insert accounts for testing updates of their balances
        let genesis_accounts: BTreeMap<Pubkey, Account> = BTreeMap::from_iter([
            (
                account1_pubkey,
                Account::new(10_000_000, 0, &system_program::id()),
            ),
            (
                account2_pubkey,
                Account::new(10_000_000, 0, &system_program::id()),
            ),
        ]);

        let db = AccountsDB::new(genesis_accounts);
        let programs = db.get_program_accounts();
        let processor = TransactionBatchProcessor::<SvmBankForks>::new(
            Slot::default(),
            Epoch::default(),
            HashSet::default(),
        );
        let engine = Engine::builder().db(db).processor(&processor).build();
        engine.initialize_cache();
        engine.initialize_transaction_processor();
        engine.fill_cache(&programs);

        // loaded transaction with account to commit into SVM state
        let loaded_transaction = LoadedTransaction {
            accounts: vec![
                (
                    account1_pubkey,
                    AccountSharedData::new(8_000_000, 0, &system_program::id()),
                ),
                (
                    account2_pubkey,
                    AccountSharedData::new(12_000_000, 0, &system_program::id()),
                ),
            ],
            program_indices: Vec::new(),
            fee_details: Default::default(),
            rollback_accounts: RollbackAccounts::FeePayerOnly {
                fee_payer_account: AccountSharedData::default(),
            },
            compute_budget_limits: Default::default(),
            rent: 0,
            rent_debits: Default::default(),
            loaded_accounts_data_size: 0,
        };

        test_loaded_transactions = vec![Ok(loaded_transaction)];

        let message = Message {
            account_keys: vec![account1_pubkey, account2_pubkey],
            header: MessageHeader {
                num_required_signatures: 2,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 0,
            },
            instructions: Vec::new(),
            recent_blockhash: Hash::default(),
        };
        let sanitized_transaction = prepared_test_sanitized_transaction(Some(message));

        res = engine.commit_transactions(
            &test_loaded_transactions,
            &test_execution_results,
            &vec![sanitized_transaction],
        );

        // testing transactions with acc1 and acc2 new balances
        // imitating sol transfer instruction execution behavior
        assert!(res.is_ok());

        let mut res = engine
            .get_account(&account1_pubkey)
            .unwrap()
            .ok_or_else(|| eyre::eyre!("account1 not found"))
            .unwrap()
            .lamports;

        // acc1 initial balance 10_000_000, new balance 8_000_000
        assert_eq!(res, 8_000_000);

        res = engine
            .get_account(&account2_pubkey)
            .unwrap()
            .ok_or_else(|| eyre::eyre!("account2 not found"))
            .unwrap()
            .lamports;

        // acc2 initial balance 10_000_000, new balance 12_000_000
        assert_eq!(res, 12_000_000);
    }

    fn prepared_test_sanitized_transaction(message: Option<Message>) -> SanitizedTransaction {
        let fee_payer_address = Pubkey::new_unique();
        let message = if let Some(message) = message {
            message
        } else {
            Message {
                account_keys: vec![fee_payer_address],
                header: MessageHeader::default(),
                instructions: Vec::new(),
                recent_blockhash: Hash::default(),
            }
        };

        let sanitized_message =
            SanitizedMessage::Legacy(LegacyMessage::new(message, &HashSet::new()));
        SanitizedTransaction::new_for_tests(sanitized_message, vec![Signature::new_unique()], false)
    }

    fn prepare_new_account_transaction(
        payer: &Keypair,
        new_account: &Keypair,
        rent: u64,
        blockhash: Hash,
    ) -> Transaction {
        let instruction = system_instruction::create_account(
            &payer.pubkey(),
            &new_account.pubkey(),
            rent,
            0,
            &system_program::id(),
        );

        Transaction::new_signed_with_payer(
            &[instruction],
            Some(&payer.pubkey()),
            &[payer, new_account],
            blockhash,
        )
    }

    fn prepare_sol_transfer_transaction(
        payer: &Keypair,
        receiver: &Pubkey,
        lamports: u64,
        blockhash: Hash,
    ) -> Transaction {
        let instruction = system_instruction::transfer(&payer.pubkey(), receiver, lamports);

        Transaction::new_signed_with_payer(
            &[instruction],
            Some(&payer.pubkey()),
            &[payer],
            blockhash,
        )
    }
}
