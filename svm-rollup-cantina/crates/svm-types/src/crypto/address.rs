use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_sdk::{bs58, pubkey::Pubkey};
use sov_address::Not28Bytes;
#[cfg(feature = "test-utils")]
use sov_mock_zkvm::crypto::Ed25519PublicKey;
use sov_modules_api::{macros::UniversalWallet, schemars, BasicAddress};
use sov_sp1_adapter::crypto::SP1PublicKey;

use crate::crypto::SolanaPublicKey;

/// An address displayed in the Solana style base-58 encoding.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    BorshDeserialize,
    BorshSerialize,
    UniversalWallet,
)]
pub struct SolanaAddress(
    #[serde(with = "svm_engine_keys::pubkey::single")]
    #[sov_wallet(as_ty = "SolanaAddressSchema")]
    Pubkey,
);

const SOLANA: &str = "solana";

#[derive(UniversalWallet)]
#[allow(dead_code)]
#[doc(hidden)]
struct SolanaAddressSchema(#[sov_wallet(display(bech32(prefix = "SOLANA")))] Vec<u8>);

impl schemars::JsonSchema for SolanaAddress {
    fn schema_name() -> String {
        "PubkeyWrapper".to_string()
    }

    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        serde_json::from_value(serde_json::json!({
            "type": "string",
            "pattern": "[1-9A-HJ-NP-Za-km-z]{32,44}",
            "description": "32 bytes in base58 encoding",
        }))
        .unwrap()
    }
}

impl From<Pubkey> for SolanaAddress {
    fn from(pubkey: Pubkey) -> Self {
        SolanaAddress(pubkey)
    }
}

impl From<&Pubkey> for SolanaAddress {
    fn from(pubkey: &Pubkey) -> Self {
        SolanaAddress(*pubkey)
    }
}

impl From<SolanaAddress> for Pubkey {
    fn from(wrapper: SolanaAddress) -> Self {
        wrapper.0
    }
}

impl From<&SolanaAddress> for Pubkey {
    fn from(wrapper: &SolanaAddress) -> Self {
        wrapper.0
    }
}

impl From<&SP1PublicKey> for SolanaAddress {
    fn from(key: &SP1PublicKey) -> Self {
        // HACK: We convert the SP1PublicKey to a PubkeyWrapper via borsh since
        // SP1PublicKey doesn't expose any accessor for the raw bytes. This will be fixed in
        // the next version of the SDK, at which point this hack can be removed.
        let key_bytes = borsh::to_vec(key).expect("Serialization to a vec is infallible");
        let output: [u8; 32] = <[u8; 32] as BorshDeserialize>::deserialize(&mut &key_bytes[..])
            .expect("A dalek public key is 32 bytes");
        Self(Pubkey::new_from_array(output))
    }
}

impl From<SolanaAddress> for SP1PublicKey {
    fn from(address: SolanaAddress) -> Self {
        // HACK: We convert the PubkeyWrapper to a SP1PublicKey via borsh since
        // SP1PublicKey doesn't expose a constructor from raw bytes. This will be fixed in
        // the next version of the SDK, at which point this hack can be removed.
        let address_bytes =
            borsh::to_vec(&address.0.to_bytes()).expect("Serialization to a vec is infallible");
        SP1PublicKey::try_from_slice(&address_bytes)
            .expect("A PubkeyWrapper should always be convertible to a SP1PublicKey")
    }
}

#[cfg(feature = "test-utils")]
impl From<&Ed25519PublicKey> for SolanaAddress {
    fn from(key: &Ed25519PublicKey) -> Self {
        // HACK: We convert the Ed25519PublicKey to a PubkeyWrapper via borsh since
        // Ed25519PublicKey doesn't expose any accessor for the raw bytes. This will be fixed in
        // the next version of the SDK, at which point this hack can be removed.
        let key_bytes = borsh::to_vec(key).expect("Serialization to a vec is infallible");
        let output = <[u8; 32] as BorshDeserialize>::deserialize(&mut &key_bytes[..])
            .expect("A dalek public key is 32 bytes");
        Self(Pubkey::new_from_array(output))
    }
}

impl From<[u8; 32]> for SolanaAddress {
    fn from(value: [u8; 32]) -> Self {
        Self(Pubkey::new_from_array(value))
    }
}

impl From<SolanaAddress> for [u8; 32] {
    fn from(value: SolanaAddress) -> Self {
        value.0.to_bytes()
    }
}

impl TryFrom<&[u8]> for SolanaAddress {
    type Error = anyhow::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != 32 {
            anyhow::bail!(
                "Invalid base58 address. Addresses are 32 bytes but only {} bytes could be decoded",
                value.len()
            );
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(value);
        Ok(Self(Pubkey::new_from_array(key)))
    }
}

impl AsRef<[u8]> for SolanaAddress {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl std::fmt::Display for SolanaAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&bs58::encode(&self.0.to_bytes()).into_string())
    }
}

impl std::str::FromStr for SolanaAddress {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut output = [0u8; 32];
        let bytes_decoded = bs58::decode(s).onto(&mut output)?;
        if bytes_decoded != 32 {
            anyhow::bail!(
                "Invalid base58 address. Addresses are 32 bytes but only {bytes_decoded} bytes could be decoded"
            );
        }
        Ok(Self(Pubkey::new_from_array(output)))
    }
}

impl<'a> From<&'a SolanaPublicKey> for SolanaAddress {
    fn from(key: &'a SolanaPublicKey) -> Self {
        Self::from(&key.pub_key)
    }
}

#[cfg(feature = "arbitrary")]
mod arbitrary_impls {
    use proptest::prelude::{any, Strategy};

    use super::SolanaAddress;

    impl<'a> arbitrary::Arbitrary<'a> for super::SolanaAddress {
        fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
            let mut bytes = [0u8; 32];
            u.fill_buffer(&mut bytes)?;
            Ok(SolanaAddress::from(bytes))
        }
    }

    impl proptest::arbitrary::Arbitrary for super::SolanaAddress {
        type Parameters = ();
        type Strategy = proptest::strategy::BoxedStrategy<Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            any::<[u8; 32]>().prop_map(SolanaAddress::from).boxed()
        }
    }
}

impl BasicAddress for SolanaAddress {}
impl Not28Bytes for SolanaAddress {}

#[cfg(all(test, feature = "native"))]
mod tests {
    use solana_sdk::pubkey::Pubkey;
    use sov_modules_api::PrivateKey;
    use sov_sp1_adapter::crypto::private_key::SP1PrivateKey;

    use super::SolanaAddress;
    use crate::SolanaPrivateKey;

    #[test]
    // Test that the address to public key conversion works for a random public key.
    fn test_address_from_pubkey() {
        let pubkey = SP1PrivateKey::generate().pub_key();
        let _ = SolanaAddress::from(&pubkey);
    }

    #[test]
    fn test_address_serde() {
        let address = SolanaAddress::from([0; 32]);

        let serialized = serde_json::to_string(&address).unwrap();
        assert_eq!(serialized, "\"11111111111111111111111111111111\"");
        let deserialized: SolanaAddress = serde_json::from_str(&serialized).unwrap();
        assert_eq!(address, deserialized, "{deserialized:?}");

        let serialized = bincode::serialize(&address).unwrap();
        let deserialized: SolanaAddress = bincode::deserialize(&serialized).unwrap();
        assert_eq!(address, deserialized, "{deserialized:?}");
    }

    #[test]
    fn test_deserialize_pubkey() {
        use serde_json::{from_str, to_string, Value};

        // Setup
        let input_pubkey = Pubkey::new_unique();
        let json_string = {
            let value = Value::String(input_pubkey.to_string());
            to_string(&value).unwrap()
        };

        // Deserialize
        let wrapper: SolanaAddress = from_str(&json_string).unwrap();
        let deserialized_pubkey: Pubkey = wrapper.into();

        // Check
        assert_eq!(input_pubkey, deserialized_pubkey);
    }

    #[test]
    fn test_key_to_addr() {
        let keypair = SolanaPrivateKey::generate();
        let pubkey = keypair.pub_key().pub_key.to_string();
        let found_addr: SolanaAddress = (&(keypair.pub_key())).into();
        assert_eq!(found_addr.to_string(), pubkey);
    }
}
