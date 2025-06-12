use sov_address::FromVmAddress;
use sov_modules_api::{transaction::Transaction, RawTx, Spec};
use sov_test_utils::{generate_runtime, sov_universal_wallet::schema::SchemaGenerator};
use svm::{
    authentication::{Auth, SolanaAuthenticator},
    SVM,
};
use svm_types::SolanaAddress;

generate_runtime! {
    name: SvmRuntime,
    modules: [svm: SVM<S>],
    operating_mode: OperatingMode::Zk,
    minimal_genesis_config_type: sov_test_utils::runtime::genesis::optimistic::MinimalOptimisticGenesisConfig<S>,
    runtime_trait_impl_bounds: [S::Address: FromVmAddress<SolanaAddress>],
    kernel_type: sov_kernels::basic::BasicKernel<'a, S>,
    auth_type: svm::authentication::SvmAuthenticator<S, Self>,
    auth_call_wrapper: |call| match call {
        Auth::Svm(call) => SvmRuntimeCall::Svm(call),
        Auth::Mod(call) => call,
    },
}

impl<S: Spec> SolanaAuthenticator<S> for SvmRuntime<S>
where
    S::Address: FromVmAddress<SolanaAddress>,
    Transaction<Self, S>: SchemaGenerator,
{
    fn add_solana_auth(tx: RawTx) -> Auth {
        Auth::Svm(tx)
    }
}
