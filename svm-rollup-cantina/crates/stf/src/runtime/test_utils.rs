use sov_address::FromVmAddress;
use sov_modules_api::{Genesis, Spec};
use sov_sequencer_registry::SequencerRegistry;
use sov_test_utils::runtime::traits::MinimalGenesis;
use svm_types::SolanaAddress;

use super::Runtime;

impl<S> MinimalGenesis<S> for Runtime<S>
where
    S: Spec,
    S::Address: FromVmAddress<SolanaAddress>,
{
    /// Returns a reference to the sequencer registry config.
    fn sequencer_registry_config(
        config: &Self::Config,
    ) -> &<SequencerRegistry<S> as Genesis>::Config {
        &config.sequencer_registry
    }
}
