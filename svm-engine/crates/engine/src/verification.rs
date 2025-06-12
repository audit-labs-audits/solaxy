use std::collections::HashSet;

use solana_program::pubkey::Pubkey;
use solana_sdk::transaction::{self, SanitizedTransaction, Transaction, TransactionError};
use solana_svm::account_loader::{CheckedTransactionDetails, TransactionCheckResult};

/// Sanitize and verify signatures of a transaction and return a [`SanitizedTransaction`] if successful.
pub fn sanitize_and_verify_tx(
    transaction: &Transaction,
    reserved_account_keys: &HashSet<Pubkey>,
) -> Result<SanitizedTransaction, TransactionError> {
    let sanitized_transaction = sanitize_tx(transaction, reserved_account_keys)
        .map_err(|_e| TransactionError::SanitizeFailure)?;

    sanitized_transaction.verify()?;

    Ok(sanitized_transaction)
}

/// Sanitize a transaction and return a [`SanitizedTransaction`] if successful.
pub fn sanitize_tx(
    transaction: &Transaction,
    reserved_account_keys: &HashSet<Pubkey>,
) -> Result<SanitizedTransaction, TransactionError> {
    let sanitized_transaction = SanitizedTransaction::try_from_legacy_transaction(
        transaction.clone(),
        reserved_account_keys,
    )
    .map_err(|_e| TransactionError::SanitizeFailure)?;

    Ok(sanitized_transaction)
}

pub fn get_transaction_check_results(
    sanitized_tx: &[SanitizedTransaction],
    max_age: usize,
) -> Vec<transaction::Result<CheckedTransactionDetails>> {
    sanitized_tx
        .iter()
        .map(|tx| check_age(tx, max_age))
        .collect()
}

// Completely stripped version of this function
fn check_age(_sanitized_tx: &SanitizedTransaction, _max_age: usize) -> TransactionCheckResult {
    Ok(CheckedTransactionDetails {
        nonce: None,
        lamports_per_signature: 1_000,
    })
}
