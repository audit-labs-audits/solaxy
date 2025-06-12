use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_sdk::signature::Signature;
use sov_modules_api::schemars;
use sov_rollup_interface::crypto::SigVerificationError;

use crate::crypto::public_key::SolanaPublicKey;

#[derive(
    PartialEq,
    Eq,
    Debug,
    Clone,
    Serialize,
    Deserialize,
    BorshDeserialize,
    BorshSerialize,
    schemars::JsonSchema,
)]
pub struct SolanaSignature {
    /// The inner signature.
    #[schemars(flatten, with = "String", length(equal = "128"))]
    pub msg_sig: Signature,
}

impl TryFrom<&[u8]> for SolanaSignature {
    type Error = anyhow::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self {
            msg_sig: Signature::try_from(value).map_err(anyhow::Error::msg)?,
        })
    }
}

impl sov_rollup_interface::crypto::Signature for SolanaSignature {
    type PublicKey = SolanaPublicKey;

    fn verify(&self, pub_key: &Self::PublicKey, msg: &[u8]) -> Result<(), SigVerificationError> {
        if self.msg_sig.verify(&pub_key.pub_key.to_bytes(), msg) {
            Ok(())
        } else {
            Err(SigVerificationError {
                error: "Invalid signature".to_string(),
            })
        }
    }
}

#[cfg(feature = "native")]
impl std::str::FromStr for SolanaSignature {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(SolanaSignature {
            msg_sig: Signature::from_str(s).map_err(|_| anyhow::anyhow!("Invalid signature"))?,
        })
    }
}

#[cfg(feature = "arbitrary")]
mod arbitrary_impls {
    use proptest::prelude::{any, BoxedStrategy, Strategy};
    use sov_modules_api::PrivateKey;

    use super::SolanaSignature;
    use crate::SolanaPrivateKey;

    impl<'a> arbitrary::Arbitrary<'a> for SolanaSignature {
        fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
            // the secret/public pair is lost; it is impossible to verify this signature
            // to run a verification, generate the keys+payload individually
            let payload_len = u.arbitrary_len::<u8>()?;
            let payload = u.bytes(payload_len)?;
            SolanaPrivateKey::arbitrary(u).map(|s| s.sign(payload))
        }
    }

    impl proptest::arbitrary::Arbitrary for SolanaSignature {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            any::<(SolanaPrivateKey, Vec<u8>)>()
                .prop_map(|(key, bytes)| key.sign(&bytes))
                .boxed()
        }
    }
}

#[cfg(all(test, feature = "arbitrary", feature = "native"))]
mod proptest_tests {
    use proptest::prelude::{any, proptest};

    use super::*;

    proptest! {
        #[test]
        fn sig_json_schema_is_valid(item in any::<SolanaSignature>()) {
            let serialized = serde_json::to_value(item).unwrap();
            let schema = serde_json::to_value(schemars::schema_for!(SolanaSignature)).unwrap();

            jsonschema::validate(&schema, &serialized).unwrap();
        }

    }
}
