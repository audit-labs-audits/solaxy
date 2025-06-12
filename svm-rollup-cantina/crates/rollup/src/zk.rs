use std::sync::Arc;

pub use sov_mock_zkvm::{
    MockCodeCommitment as OuterCodeCommitment, MockZkvm as OuterZkvm, MockZkvmHost as OuterZkvmHost,
};
pub use sov_sp1_adapter::{
    host::SP1Host as InnerZkvmHost, SP1CryptoSpec as InnerCryptoSpec, SP1 as InnerZkvm,
};

use crate::da::SupportedDaLayer;

/// Returns the host arguments for a rollup. This is the code that is proven by the rollup
pub fn rollup_host_args(da_layer: SupportedDaLayer) -> Arc<&'static [u8]> {
    if std::env::var("SKIP_GUEST_BUILD").is_ok() {
        return Arc::new(Vec::new().leak());
    }

    match da_layer {
        SupportedDaLayer::Celestia => Arc::new(&sp1::SP1_GUEST_CELESTIA_ELF),
        SupportedDaLayer::Mock => Arc::new(&sp1::SP1_GUEST_MOCK_ELF),
        SupportedDaLayer::Solana => Arc::new(&sp1::SP1_GUEST_SOLANA_ELF),
    }
}

/// Returns the outer zkvm host.
pub fn get_outer_vm() -> OuterZkvmHost {
    OuterZkvmHost::new_non_blocking()
}
