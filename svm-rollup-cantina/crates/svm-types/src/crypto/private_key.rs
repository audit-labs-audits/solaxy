use rand::rngs::OsRng;
use solana_sdk::{signature::Keypair, signer::Signer};

use crate::crypto::{SolanaPublicKey, SolanaSignature};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct SolanaPrivateKey {
    #[serde(with = "svm_engine_keys::keypair")]
    key_pair: Keypair,
}

impl std::fmt::Display for SolanaPrivateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.key_pair.to_base58_string())
    }
}

impl Clone for SolanaPrivateKey {
    fn clone(&self) -> Self {
        Self {
            key_pair: self.key_pair.insecure_clone(),
        }
    }
}

impl core::fmt::Debug for SolanaPrivateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SolanaPrivateKey")
            .field("public_key", &self.key_pair.pubkey().to_string())
            .field("private_key", &"***REDACTED***")
            .finish()
    }
}

impl sov_rollup_interface::crypto::PrivateKey for SolanaPrivateKey {
    type PublicKey = SolanaPublicKey;
    type Signature = SolanaSignature;

    fn generate() -> Self {
        let mut csprng = OsRng {};

        Self {
            key_pair: Keypair::generate(&mut csprng),
        }
    }

    fn pub_key(&self) -> Self::PublicKey {
        SolanaPublicKey {
            pub_key: self.key_pair.pubkey(),
        }
    }

    fn sign(&self, msg: &[u8]) -> Self::Signature {
        SolanaSignature {
            msg_sig: self.key_pair.sign_message(msg),
        }
    }
}

#[cfg(feature = "arbitrary")]
mod arbitrary_impls {
    use proptest::{
        prelude::{any, BoxedStrategy},
        strategy::Strategy,
    };
    use rand::{rngs::StdRng, SeedableRng};

    use super::*;

    impl<'a> arbitrary::Arbitrary<'a> for SolanaPrivateKey {
        fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
            // it is important to generate the secret deterministically from the arbitrary argument
            // so keys and signatures will be reproducible for a given seed.
            // this unlocks fuzzy replay
            let seed = <[u8; 32]>::arbitrary(u)?;
            let rng = &mut StdRng::from_seed(seed);
            let key_pair = Keypair::generate(rng);

            Ok(Self { key_pair })
        }
    }

    impl proptest::arbitrary::Arbitrary for SolanaPrivateKey {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            any::<[u8; 32]>()
                .prop_map(|seed| Self {
                    key_pair: Keypair::generate(&mut StdRng::from_seed(seed)),
                })
                .boxed()
        }
    }
}

#[cfg(all(test, feature = "native"))]
mod tests {
    use sov_modules_api::PrivateKey;

    use crate::SolanaPrivateKey;

    #[test]
    fn test_privatekey_serde_bincode() {
        let key_pair = SolanaPrivateKey::generate();
        let serialized = bincode::serialize(&key_pair).expect("Serialization to vec is infallible");
        let output = bincode::deserialize::<SolanaPrivateKey>(&serialized)
            .expect("SigningKey is serialized correctly");

        assert_eq!(key_pair.to_string(), output.to_string());
    }

    #[test]
    fn test_privatekey_serde_json() {
        let key_pair = SolanaPrivateKey::generate();
        let serialized = serde_json::to_vec(&key_pair).expect("Serialization to vec is infallible");
        let output = serde_json::from_slice::<SolanaPrivateKey>(&serialized)
            .expect("Keypair is serialized correctly");

        assert_eq!(key_pair.to_string(), output.to_string());
    }
}

#[cfg(all(test, feature = "arbitrary", feature = "native"))]
mod proptest_tests {
    use proptest::{
        collection::vec,
        prelude::{any, proptest},
    };
    use sov_modules_api::{PrivateKey, Signature};

    use super::*;

    proptest! {
        #[test]
        fn sig_verification_works(msg in vec(any::<u8>(), 0..100)) {
            let key = SolanaPrivateKey::generate();
            let signature = key.sign(&msg);
            let pubkey = key.pub_key();
            assert!(signature.verify(&pubkey, &msg).is_ok());
        }
    }
}
