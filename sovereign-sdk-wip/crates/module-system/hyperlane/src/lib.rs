#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
#[cfg(feature = "native")]
use sov_modules_api::rest::HasRestApi;
use sov_modules_api::{
    Context, DaSpec, Error, GenesisState, HexHash, HexString, Module, ModuleId, ModuleInfo,
    ModuleRestApi, Spec, StateMap, StateReader, StateValue, TxState,
};
use sov_state::User;

#[cfg(feature = "native")]
mod api;
mod call;
#[cfg(feature = "test-utils")]
pub mod crypto;
#[cfg(not(feature = "test-utils"))]
mod crypto;
mod event;
mod genesis;
pub mod igp;
mod ism;
mod merkle;
#[cfg(feature = "test-utils")]
pub mod test_recipient;
pub mod traits;
mod types;
pub mod warp;

pub use call::*;
pub use event::Event;
pub use igp::{
    CallMessage as InterchainGasPaymasterCallMessage, Event as InterchainGasPaymasterEvent,
    InterchainGasPaymaster,
};
pub use ism::Ism;
pub use merkle::{Event as MerkleTreeEvent, MerkleTreeHook};
pub use types::{EthAddress, Message, StorageLocation, ValidatorSignature};
pub use warp::{CallMessage as WarpCallMessage, Event as WarpEvent, Warp};

/// A fixed assumed address of mailbox contract on sovereign rollup.
///
/// Hyperlane uses contract addresses for various operations, however in this implementation
/// we use sovereign-sdk module system. To satisfy hyperlane protocol, we use this constant
/// as a stub for an address of a mailbox on the rollup.
pub const MAILBOX_ADDR: [u8; 32] = [0; 32];

/// The state of the mailbox.
#[derive(
    Clone, BorshDeserialize, BorshSerialize, Debug, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct DispatchState {
    /// The nonce for the current dispatch.
    pub nonce: u32,
    /// The last message ID that has been dispatched.
    pub last_dispatched_id: HexHash,
}

type MessageId = HexHash;

/// The delivery receipt of a message.
#[derive(
    Clone, BorshDeserialize, BorshSerialize, Debug, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct Delivery {
    /// The sender of the message.
    pub sender: HexHash,
    /// The block number it was dispatched in.
    pub block_number: u64,
}

/// The mailbox module is the entrypoint for the hyperlane protocol. All messages sent or received are routed through this module.
#[derive(Clone, ModuleInfo, ModuleRestApi)]
pub struct Mailbox<S: Spec, R: Recipient<S>> {
    /// The ID of the module.
    #[id]
    pub id: ModuleId,

    /// The number of messages and latest ID that have been dispatched.
    #[state]
    pub dispatch_state: StateValue<DispatchState>,

    /// A map of message IDs to their delivery status.
    #[state]
    pub deliveries: StateMap<MessageId, Delivery>,

    /// A map of announced validator addresses to their signature locations.
    #[state]
    pub validators: StateMap<EthAddress, Vec<StorageLocation>>,

    /// A reference to the merkle tree hooks module.
    #[module]
    pub merkle_tree_hook: MerkleTreeHook<S>,

    /// A reference to the interchain gas paymaster module.
    ///
    /// IGP is a custom hook in Hyperlane's Ethereum implementation that allows users to select a relayer through the relayer's IGP smart contract.
    /// We provide users with the ability to select a relayer by including the relayer's address as an additional parameter sent with the message (though this address is not part of the IGP message itself).
    #[module]
    pub interchain_gas_paymaster: InterchainGasPaymaster<S>,

    /// A reference to the recipient module.
    #[module]
    pub recipients: R,

    #[phantom]
    phantom: std::marker::PhantomData<S>,
}

impl<S: Spec, R: Recipient<S>> Module for Mailbox<S, R>
where
    S::Address: HyperlaneAddress,
{
    type Spec = S;

    type Config = ();

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
        match msg {
            call::CallMessage::Dispatch {
                domain,
                recipient,
                body,
                metadata,
                relayer,
                gas_payment_limit,
            } => {
                self.dispatch(
                    domain,
                    recipient,
                    context.sender().to_sender(),
                    HexString::new(body.0.into()),
                    metadata.map(|m| HexString::new(m.0.into())),
                    relayer,
                    gas_payment_limit,
                    context,
                    state,
                )?;
                Ok(())
            }
            call::CallMessage::Process { metadata, message } => Ok(self.process(
                HexString::new(metadata.0.into()),
                HexString::new(message.0.into()),
                context,
                state,
            )?),
            call::CallMessage::Announce {
                validator_address,
                storage_location,
                signature,
            } => Ok(self.announce(validator_address, storage_location, signature, state)?),
        }
    }
}

impl<S: Spec, R: Recipient<S>> Mailbox<S, R> {
    pub(crate) fn with_default_relayer(
        &self,
        preselected_relayer: Option<S::Address>,
        recipient_address: &HexHash,
        state: &impl StateReader<User, Error: Into<anyhow::Error>>,
    ) -> anyhow::Result<S::Address> {
        match preselected_relayer {
            Some(relayer) => Ok(relayer),
            None => {
                let default_relayer = self.recipients.default_relayer(recipient_address, state)?;

                default_relayer
                    .ok_or_else(|| anyhow::anyhow!("relayer not selected and no default one"))
            }
        }
    }
}

/// An address which is compatible with the hyperlane protocol.
///
/// Implementers of this trait must ensure that their addresses can be unambiguously represented in 32 bytes.
/// For example, if the address type is an enum where at least one variant is 32 bytes long, the impelementation
/// must pick one variant and always deserialize into that type (since there's no room to encode the discriminant).
pub trait HyperlaneAddress: Sized {
    /// Convert the address to a Hyperlane sender address.
    fn to_sender(&self) -> HexHash;
    /// Convert a Hyperlane sender address back to the original..
    fn from_sender(recipient: HexHash) -> anyhow::Result<Self>;
}

impl HyperlaneAddress for sov_modules_api::Address {
    fn to_sender(&self) -> HexHash {
        const START_INDEX: usize = 32 - sov_modules_api::Address::LENGTH;
        // Pad the address with leading zeros to 32 bytes. This is the hyperlane convention
        let mut bytes = [0u8; 32];
        bytes[START_INDEX..].copy_from_slice(self.as_ref());
        bytes.into()
    }

    fn from_sender(recipient: HexHash) -> anyhow::Result<Self> {
        const START_INDEX: usize = 32 - sov_modules_api::Address::LENGTH;
        // Check that the address is padded with leading zeros to match the hyperlane convention.
        let (padding, address) = recipient.0.split_at(START_INDEX);

        // Ensure padding is all zeros:
        anyhow::ensure!(
            padding.iter().all(|&byte| byte == 0),
            "Invalid address - not enough leading zeros"
        );

        Ok(Self::new(address.try_into().expect(
            "Infallible conversion failed; this is a bug, please report it",
        )))
    }
}

/// A module that can receive messages via the hyperlane protocol.
///
/// This module may be a "wrapper" module, which internally dispatches to several
/// other recipients.
pub trait Recipient<S: Spec>:
    Module + Clone + std::default::Default + ModuleInfo + HasNativeRestApi<S> + Send + Sync + 'static
{
    /// Get the [`ISM`](Ism) for a given recipient address.
    fn ism(&self, recipient: &HexHash, state: &mut impl TxState<S>) -> anyhow::Result<Option<Ism>>;

    /// Get the default [`ISM`](Ism).
    ///
    /// Default `ISM` will be used for verifying messages
    /// to recipients which don't have a dedicated `ISM` configured.
    /// If no dedicated `ISM` is set for a recipient, and this function
    /// returns `None`, then a delivery of a message will fail.
    fn default_ism(&self, _state: &mut impl TxState<S>) -> anyhow::Result<Option<Ism>>;

    /// Handle an inbound message.
    fn handle(
        &mut self,
        origin: u32,
        sender: HexHash,
        recipient: &HexHash,
        body: HexString,
        state: &mut impl TxState<S>,
    ) -> anyhow::Result<()>;

    /// Handle validator announcement.
    ///
    /// Implement this to react to to validators announcing themselves.
    /// It is called after the identity of validator has already been confirmed.
    /// Default implementation just ignores any announcements.
    fn handle_validator_announce(
        &self,
        _validator_address: &EthAddress,
        _storage_location: &StorageLocation,
        _state: &mut impl TxState<S>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// Returns default relayer
    ///
    /// Implement this if recipient should provide default relayer if user did not specify it
    fn default_relayer(
        &self,
        _recipient: &HexHash,
        _state: &impl StateReader<User, Error: Into<anyhow::Error>>,
    ) -> anyhow::Result<Option<S::Address>> {
        Ok(None)
    }
}

/// A helper trait which requires `HasRestApi` if the `native` feature is enabled.
#[cfg(feature = "native")]
pub trait HasNativeRestApi<S: Spec>: HasRestApi<S> {}

#[cfg(feature = "native")]
impl<S: Spec, T: HasRestApi<S>> HasNativeRestApi<S> for T {}

/// A helper trait which requires `HasRestApi` if the `native` feature is enabled.
#[cfg(not(feature = "native"))]
pub trait HasNativeRestApi<S: Spec> {}

#[cfg(not(feature = "native"))]
impl<S: Spec, T> HasNativeRestApi<S> for T {}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use sov_modules_api::Address;

    use super::*;

    #[test]
    fn test_address_conversion() {
        let address =
            Address::from_str("sov1l6n2cku82yfqld30lanm2nfw43n2auc8clw7r5u5m6s7qhzze66").unwrap();
        let sender = address.to_sender();
        let recovered = Address::from_sender(sender).unwrap();
        assert_eq!(address, recovered);
    }
}
