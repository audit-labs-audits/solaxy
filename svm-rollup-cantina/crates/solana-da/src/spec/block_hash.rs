use std::{
    fmt::{self},
    hash::Hash,
    str::FromStr,
};

use solana_sdk::bs58;
use sov_rollup_interface::da::BlockHashTrait;

/// A hash of a block on the DA layer. This is a wrapper around the native [`solana_sdk::hash::Hash`]
/// type, which is a SHA256 hash.
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    borsh::BorshSerialize,
    borsh::BorshDeserialize,
)]
pub struct SolanaBlockHash(pub solana_sdk::hash::Hash);

impl FromStr for SolanaBlockHash {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl TryFrom<&str> for SolanaBlockHash {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut bytes = [0u8; 32];
        bs58::decode(value).onto(&mut bytes)?;
        Ok(Self(bytes.into()))
    }
}

impl fmt::Display for SolanaBlockHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl AsRef<[u8]> for SolanaBlockHash {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<SolanaBlockHash> for [u8; 32] {
    fn from(slot_hash: SolanaBlockHash) -> Self {
        slot_hash.0.to_bytes()
    }
}

impl From<[u8; 32]> for SolanaBlockHash {
    fn from(value: [u8; 32]) -> Self {
        Self(value.into())
    }
}

/// Trait with a collection of trait bounds for a block hash.
impl BlockHashTrait for SolanaBlockHash {}
