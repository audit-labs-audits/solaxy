//! This crate provides custom [`serde`] implementations for dealing with human-readable values of
//! [`solana_sdk::pubkey::Pubkey`] and implementations for deserializing
//! [`solana_sdk::signature::Keypair`] from bytes.
//! This is mostly useful for handling config files with keys stored in the [`solana_sdk::bs58`]
//! format.

/// Keypair implementations for [`serde`].
#[cfg(feature = "keypair")]
pub mod keypair;
/// Pubkey implementations for [`serde`]
pub mod pubkey;
#[cfg(test)]
mod tests;
