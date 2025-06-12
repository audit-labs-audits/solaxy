use sov_modules_api::configurable_spec::ConfigurableSpec;
#[cfg(feature = "native")]
use sov_modules_api::CryptoSpec;
#[cfg(feature = "native")]
use sov_sp1_adapter::SP1CryptoSpec;
#[cfg(feature = "native")]
use sov_state::{DefaultStorageSpec, ProverStorage};

use crate::MultiAddressSvm;

pub type SolanaSpec<DA, InnerZk, OuterZk, CryptoSpec, Mode, Storage> =
    ConfigurableSpec<DA, InnerZk, OuterZk, CryptoSpec, MultiAddressSvm, Mode, Storage>;

#[cfg(feature = "native")]
pub type NativeStorage = ProverStorage<DefaultStorageSpec<<SP1CryptoSpec as CryptoSpec>::Hasher>>;

#[cfg(all(feature = "test-utils", feature = "native"))]
pub type SolanaTestSpec<DA> = SolanaSpec<
    DA,
    sov_mock_zkvm::MockZkvm,
    sov_mock_zkvm::MockZkvm,
    sov_mock_zkvm::MockZkvmCryptoSpec,
    sov_modules_api::execution_mode::Native,
    NativeStorage,
>;
