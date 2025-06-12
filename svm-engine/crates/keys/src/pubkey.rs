use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serializer, de};
use solana_sdk::pubkey::Pubkey;

/// Provides [`serialize`] and [`deserialize`] implementations for a single [`Pubkey`].
pub mod single {
    use super::*;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)
            .and_then(|key| Pubkey::from_str(&key).map_err(de::Error::custom))
    }

    pub fn serialize<S: Serializer>(pubkey: &Pubkey, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&pubkey.to_string())
    }
}

/// Provides [`serialize`] and [`deserialize`] implementations for a vector of [`Pubkey`]s.
pub mod vec {
    use serde::ser::SerializeSeq;

    use super::*;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Pubkey>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<String>::deserialize(deserializer)?
            .iter()
            .map(|key| Pubkey::from_str(key).map_err(de::Error::custom))
            .collect()
    }

    pub fn serialize<S: Serializer>(
        pubkeys: &Vec<Pubkey>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(pubkeys.len()))?;
        for key in pubkeys {
            seq.serialize_element(&key.to_string())?;
        }
        seq.end()
    }
}
