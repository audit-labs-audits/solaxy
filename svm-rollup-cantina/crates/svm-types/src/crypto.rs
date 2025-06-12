use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sov_modules_api::{schemars, CryptoSpec};

mod address;
#[cfg(feature = "native")]
mod private_key;
mod public_key;
mod signature;

pub use address::*;
#[cfg(feature = "native")]
pub use private_key::*;
pub use public_key::*;
pub use signature::*;

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
    BorshDeserialize,
    BorshSerialize,
)]
/// A [`CryptoSpec`] implementation for SVM rollups. Uses the ed25519_dalek signature scheme with
/// sha256 as the default hasher for other operations.
pub struct SvmCryptoSpec;

impl CryptoSpec for SvmCryptoSpec {
    #[cfg(feature = "native")]
    type PrivateKey = SolanaPrivateKey;

    type PublicKey = SolanaPublicKey;

    type Hasher = Sha256;

    type Signature = SolanaSignature;
}
