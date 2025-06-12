use std::hash::Hash;

use serde::de;
use solana_sdk::pubkey::{Pubkey, PUBKEY_BYTES};
use sov_modules_api::{
    digest::{consts::U32, Digest},
    schemars,
};
use sov_rollup_interface::crypto::PublicKeyHex;

#[derive(PartialEq, Eq, Hash, Clone, Debug, schemars::JsonSchema)]
pub struct SolanaPublicKey {
    #[schemars(
        flatten,
        with = "String",
        length(equal = "solana_sdk::pubkey::PUBKEY_BYTES * 2")
    )]
    pub(crate) pub_key: Pubkey,
}

impl sov_rollup_interface::crypto::PublicKey for SolanaPublicKey {
    fn credential_id<Hasher: Digest<OutputSize = U32>>(
        &self,
    ) -> sov_rollup_interface::crypto::CredentialId {
        let hash = {
            let mut hasher = Hasher::new();
            hasher.update(self.pub_key.to_bytes());
            hasher.finalize().into()
        };

        sov_rollup_interface::crypto::CredentialId(hash)
    }
}

impl borsh::BorshDeserialize for SolanaPublicKey {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut buffer = [0; PUBKEY_BYTES];
        reader.read_exact(&mut buffer)?;

        Ok(Self {
            pub_key: Pubkey::new_from_array(buffer),
        })
    }
}

impl borsh::BorshSerialize for SolanaPublicKey {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&self.pub_key.to_bytes())
    }
}

impl serde::Serialize for SolanaPublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            PublicKeyHex::from(self).serialize(serializer)
        } else {
            serializer.serialize_str(&self.pub_key.to_string())
        }
    }
}

impl<'de> serde::Deserialize<'de> for SolanaPublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use std::str::FromStr;

        if deserializer.is_human_readable() {
            PublicKeyHex::deserialize(deserializer)
                .and_then(|hex| Self::try_from(&hex).map_err(de::Error::custom))
        } else {
            String::deserialize(deserializer)
                .and_then(|s| Pubkey::from_str(&s).map_err(de::Error::custom))
                .map(|pub_key| Self { pub_key })
        }
    }
}

impl From<&SolanaPublicKey> for PublicKeyHex {
    fn from(pub_key: &SolanaPublicKey) -> Self {
        let hex = hex::encode(pub_key.pub_key.to_bytes());
        // UNWRAP: conversion to SafeString can error in only two cases: non-printable-ascii and too long.
        // A hex::encoded string should always be printable ascii, and a public key is 32 bytes =
        // 64 hex characters, well below the 128 character SafeString limit.
        Self {
            hex: hex.try_into().unwrap(),
        }
    }
}

impl TryFrom<&PublicKeyHex> for SolanaPublicKey {
    type Error = anyhow::Error;

    fn try_from(pub_key: &PublicKeyHex) -> Result<Self, Self::Error> {
        let bytes = hex::decode(&pub_key.hex)?;

        let bytes: [u8; PUBKEY_BYTES] = bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid public key size"))?;

        Ok(Self {
            pub_key: Pubkey::new_from_array(bytes),
        })
    }
}

#[cfg(feature = "native")]
impl std::str::FromStr for SolanaPublicKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pk_hex = PublicKeyHex::try_from(s)?;
        SolanaPublicKey::try_from(&pk_hex)
    }
}

#[cfg(feature = "arbitrary")]
mod arbitrary_impls {
    use proptest::prelude::{any, BoxedStrategy, Strategy};
    use sov_modules_api::PrivateKey;

    use super::SolanaPublicKey;
    use crate::SolanaPrivateKey;

    impl<'a> arbitrary::Arbitrary<'a> for SolanaPublicKey {
        fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
            SolanaPrivateKey::arbitrary(u).map(|p| p.pub_key())
        }
    }

    impl proptest::arbitrary::Arbitrary for SolanaPublicKey {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            any::<SolanaPrivateKey>()
                .prop_map(|key| key.pub_key())
                .boxed()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_pub_key_json() {
        let pub_key_hex: PublicKeyHex =
            "0204e690e67bd9d8cfc9c310ad3468de11416feefcc86da6e73613b89677a61b"
                .try_into()
                .unwrap();

        let pub_key = SolanaPublicKey::try_from(&pub_key_hex).unwrap();
        let pub_key_str: String = serde_json::to_string(&pub_key).unwrap();

        assert_eq!(
            pub_key_str,
            r#""0204e690e67bd9d8cfc9c310ad3468de11416feefcc86da6e73613b89677a61b""#
        );

        let deserialized: SolanaPublicKey = serde_json::from_str(&pub_key_str).unwrap();
        assert_eq!(deserialized, pub_key);
    }
}

#[cfg(all(test, feature = "native"))]
mod hex_tests {
    use sov_rollup_interface::crypto::PrivateKey;

    use super::*;
    use crate::SolanaPrivateKey;

    #[test]
    fn test_pub_key_hex() {
        let pub_key = SolanaPrivateKey::generate().pub_key();
        let pub_key_hex = PublicKeyHex::from(&pub_key);
        let converted_pub_key = SolanaPublicKey::try_from(&pub_key_hex).unwrap();
        assert_eq!(pub_key, converted_pub_key);
    }

    #[test]
    fn test_pub_key_hex_str() {
        let key = "0204e690e67bd9d8cfc9c310ad3468de11416feefcc86da6e73613b89677a61b";
        let pub_key_hex_lower: PublicKeyHex = key.try_into().unwrap();
        let pub_key_hex_upper: PublicKeyHex = key.to_uppercase().try_into().unwrap();
        let pub_key_lower = SolanaPublicKey::try_from(&pub_key_hex_lower).unwrap();
        let pub_key_upper = SolanaPublicKey::try_from(&pub_key_hex_upper).unwrap();

        assert_eq!(pub_key_lower, pub_key_upper);
    }
}

#[cfg(all(test, feature = "arbitrary", feature = "native"))]
mod proptest_tests {
    use proptest::prelude::{any, proptest};

    use super::*;

    proptest! {
        #[test]
        fn pub_key_json_schema_is_valid(item in any::<SolanaPublicKey>()) {
            let serialized = serde_json::to_value(item).unwrap();
            let schema = serde_json::to_value(schemars::schema_for!(SolanaPublicKey)).unwrap();

            jsonschema::validate(&schema, &serialized).unwrap();
        }
    }
}
