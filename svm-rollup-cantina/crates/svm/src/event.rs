//! Sovereign's event definition for module SVM.

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

/// SVM events
#[derive(
    BorshDeserialize,
    BorshSerialize,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Clone,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Event {
    ExecuteTransactions,
}
