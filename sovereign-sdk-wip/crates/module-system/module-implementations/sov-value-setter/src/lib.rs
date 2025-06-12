#![deny(missing_docs)]
#![doc = include_str!("../README.md")]
mod call;
mod genesis;

mod event;
pub use call::*;
pub use event::Event;
pub use genesis::*;
mod hooks;
use sov_modules_api::{
    AccessoryStateValue, Context, DaSpec, Error, Gas, GenesisState, Module, ModuleId, ModuleInfo,
    ModuleRestApi, Spec, StateValue, StateVec, TxState,
};

/// A new module:
/// - Must derive `ModuleInfo`
/// - Must contain `[id]` field
/// - Can contain any number of ` #[state]` or `[module]` fields
/// - Can derive ModuleRestApi to automatically generate Rest API endpoints
#[derive(Clone, ModuleInfo, ModuleRestApi)]
pub struct ValueSetter<S: Spec> {
    /// The ID of the module.
    #[id]
    pub id: ModuleId,

    /// Some value kept in the state.
    #[state]
    pub value: StateValue<u32>,

    /// Some more values kept in state.
    #[state]
    pub many_values: StateVec<u8>,

    /// The number of times the `begin_slot` hook has been called.
    #[state]
    pub begin_rollup_block_hook_count: StateValue<u32>,

    /// The number of times the `end_slot` hook has been called.
    #[state]
    pub end_rollup_block_hook_count: StateValue<u32>,

    /// The number of times the `finalize` hook has been called.
    #[state]
    pub finalize_hook_count: AccessoryStateValue<u32>,

    /// Holds the address of the admin user who is allowed to update the value.
    #[state]
    pub admin: StateValue<S::Address>,
}

/// Gas configuration for the bank module
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct ValueSeterGasConfig<GU: Gas> {
    /// Gas price multiplier for the set_value operation
    pub set_value: GU,
}

impl<S: Spec> Module for ValueSetter<S> {
    type Spec = S;

    type Config = ValueSetterConfig<S>;

    type CallMessage = call::CallMessage<S>;

    type Event = Event;

    fn genesis(
        &mut self,
        _genesis_rollup_header: &<<S as Spec>::Da as DaSpec>::BlockHeader,
        config: &Self::Config,
        state: &mut impl GenesisState<S>,
    ) -> Result<(), Error> {
        // The initialization logic
        Ok(self.init_module(config, state)?)
    }

    fn call(
        &mut self,
        msg: Self::CallMessage,
        context: &Context<Self::Spec>,
        state: &mut impl TxState<S>,
    ) -> Result<(), Error> {
        let mut state_wrapped = state.to_revertable();
        let state = &mut state_wrapped;
        let res = match msg {
            call::CallMessage::SetValue {
                value: new_value,
                gas,
            } => Ok(self.set_value(new_value, gas, context, state)?),
            CallMessage::SetManyValues(many) => Ok(self.set_values(many, context, state)?),
            CallMessage::AssertVisibleSlotNumber {
                expected_visible_slot_number,
            } => {
                Ok(self.assert_visible_slot_number(expected_visible_slot_number, context, state)?)
            }
        };
        state_wrapped.commit();
        res
    }
}
