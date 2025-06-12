//! The Rollup entrypoint.
//!
//! On a high level, the rollup node receives serialized call messages from the DA layer and executes them as atomic transactions.
//! Upon reception, the message has to be deserialized and forwarded to an appropriate module.
//!
//! The module-specific logic is implemented by module creators, but all the glue code responsible for message
//! deserialization/forwarding is handled by a rollup `runtime`.
//!
//! In order to define the runtime we need to specify all the modules supported by our rollup (see the `Runtime` struct bellow)
//!
//! The `Runtime` together with associated interfaces (`Genesis`, `DispatchCall`, `MessageCodec`)
//! and derive macros defines:
//! - how the rollup modules are wired up together.
//! - how the state of the rollup is initialized.
//! - how messages are dispatched to appropriate modules.
//!
//! Runtime lifecycle:
//!
//! 1. Initialization:
//!     When a rollup is deployed for the first time, it needs to set its genesis state.
//!     The `#[derive(Genesis)` macro will generate `Runtime::genesis(config)` method which returns
//!     `Storage` with the initialized state.
//!
//! 2. Calls:      
//!     The `Module` interface defines a `call` method which accepts a module-defined type and triggers the specific `module logic.`
//!     In general, the point of a call is to change the module state, but if the call throws an error,
//!     no module specific state is updated (the transaction is reverted).
//!
//! `#[derive(MessageCodec)` adds deserialization capabilities to the `Runtime` (implements `decode_call` method).
//! `Runtime::decode_call` accepts serialized call message and returns a type that implements the `DispatchCall` trait.
//!  The `DispatchCall` implementation (derived by a macro) forwards the message to the appropriate module and executes its `call` method.

use sov_address::FromVmAddress;
use sov_kernels::soft_confirmations::SoftConfirmationsKernel;
#[cfg(feature = "native")]
use sov_modules_api::macros::{expose_rpc, CliWallet};
use sov_modules_api::{
    capabilities::{Guard, HasKernel, TransactionAuthenticator},
    prelude::*,
    DispatchCall, Event, Genesis, Hooks, MessageCodec, OperatingMode, RawTx, Spec,
};
use svm::authentication::{Auth, SolanaAuthenticator, SvmAuthenticator};
use svm_types::SolanaAddress;

use crate::chain_hash;
#[cfg(feature = "native")]
use crate::genesis_config::GenesisPaths;

/// The capabilities required for a SVM zk-rollup runtime.
pub mod capabilities;
#[cfg(feature = "test-utils")]
mod test_utils;

/// The `stf runtime`.
#[derive(Default, Genesis, Hooks, DispatchCall, Event, MessageCodec, RuntimeRestApi)]
#[cfg_attr(feature = "native", derive(CliWallet), expose_rpc)]
pub struct Runtime<S: Spec>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    /// The Bank module.
    pub bank: sov_bank::Bank<S>,
    /// The Sequencer Registry module.
    pub sequencer_registry: sov_sequencer_registry::SequencerRegistry<S>,
    /// The Attester Incentives module.
    pub attester_incentives: sov_attester_incentives::AttesterIncentives<S>,
    /// The Prover Incentives module.
    pub prover_incentives: sov_prover_incentives::ProverIncentives<S>,
    /// The Accounts module.
    pub accounts: sov_accounts::Accounts<S>,
    /// The Chain state module.
    pub chain_state: sov_chain_state::ChainState<S>,
    /// The Blob storage module.
    pub blob_storage: sov_blob_storage::BlobStorage<S>,
    #[cfg_attr(feature = "native", cli_skip)]
    /// The SVM module.
    pub svm: svm::SVM<S>,
}

impl<S> sov_modules_stf_blueprint::Runtime<S> for Runtime<S>
where
    S: Spec,
    S::Address: FromVmAddress<SolanaAddress>,
{
    const CHAIN_HASH: [u8; 32] = chain_hash::CHAIN_HASH;
    type GenesisConfig = GenesisConfig<S>;

    #[cfg(feature = "native")]
    type GenesisInput = GenesisPaths;

    type Auth = SvmAuthenticator<S, Self>;

    #[cfg(feature = "native")]
    fn endpoints(api_state: sov_modules_api::rest::ApiState<S>) -> sov_modules_api::NodeEndpoints {
        use ::sov_modules_api::rest::HasRestApi;

        let axum_router = Self::default().rest_api(api_state.clone());

        sov_modules_api::NodeEndpoints {
            axum_router,
            jsonrpsee_module: get_rpc_methods::<S>(api_state.clone()),
            background_handles: Vec::new(),
        }
    }

    #[cfg(feature = "native")]
    fn genesis_config(genesis_paths: &Self::GenesisInput) -> anyhow::Result<Self::GenesisConfig> {
        crate::genesis_config::create_genesis_config(genesis_paths)
    }

    fn operating_mode(genesis: &Self::GenesisConfig) -> OperatingMode {
        genesis.chain_state.operating_mode
    }

    /// Wraps [`TransactionAuthenticator::Input`] into a call message.
    fn wrap_call(
        auth_data: <Self::Auth as TransactionAuthenticator<S>>::Decodable,
    ) -> Self::Decodable {
        match auth_data {
            Auth::Svm(call) => Self::Decodable::Svm(call),
            Auth::Mod(call) => call,
        }
    }

    fn allow_unregistered_tx(call: &Self::Decodable) -> bool {
        matches!(
            call,
            Self::Decodable::SequencerRegistry(
                sov_sequencer_registry::CallMessage::Register { .. }
            )
        )
    }
}

impl<S: Spec> HasKernel<S> for Runtime<S>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    type Kernel<'a> = SoftConfirmationsKernel<'a, S>;

    fn inner(&mut self) -> Guard<Self::Kernel<'_>> {
        Guard::new(SoftConfirmationsKernel {
            chain_state: &mut self.chain_state,
            blob_storage: &mut self.blob_storage,
        })
    }

    #[cfg(feature = "native")]
    fn kernel_with_slot_mapping(
        &self,
    ) -> std::sync::Arc<dyn sov_modules_api::capabilities::KernelWithSlotMapping<S>> {
        std::sync::Arc::new(self.chain_state.clone())
    }
}

impl<S: Spec> SolanaAuthenticator<S> for Runtime<S>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    fn add_solana_auth(tx: RawTx) -> Auth {
        Auth::Svm(tx)
    }
}
