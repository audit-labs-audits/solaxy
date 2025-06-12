//! Defines a module that can send and receive tokens using hyperlane's Warp protocol.
//!
//! <https://docs.hyperlane.xyz/docs/protocol/warp-routes/warp-routes-overview>

use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sov_bank::derived_holder::DerivedHolder;
use sov_bank::{config_gas_token_id, Bank, Coins, IntoPayable, TokenId};
use sov_modules_api::digest::Digest;
use sov_modules_api::macros::UniversalWallet;
use sov_modules_api::{
    Amount, Context, CryptoSpec, Error, EventEmitter, HexHash, HexString, Module, ModuleId,
    ModuleInfo, ModuleRestApi, Spec, StateMap, TxState,
};

use crate::ism::Ism;
use crate::{HyperlaneAddress, Mailbox, Recipient};

mod types;

pub use types::*;

/// Implements support for Hyperlane Warp Routes
#[derive(Clone, ModuleInfo, ModuleRestApi)]
pub struct Warp<S: Spec> {
    /// The ID of the module.
    #[id]
    pub id: ModuleId,

    /// A mapping from route IDs to their instances.
    #[state]
    warp_routes: StateMap<WarpRouteId, WarpRouteInstance<S>>,

    /// A mapping from router keys to their instances.
    #[state]
    routers: StateMap<RouterKey, RemoteRouterAddress>,

    /// The bank module.
    #[module]
    pub bank: Bank<S>,

    #[phantom]
    phantom: std::marker::PhantomData<S>,
}

/// Call messages for the test recipient module.
#[derive(
    Clone,
    BorshDeserialize,
    BorshSerialize,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
    UniversalWallet,
)]
pub enum CallMessage<S: Spec> {
    /// Register a route with the given token source and ISM.
    Register {
        /// The authority that can modify the route, if any.
        admin: Admin<S>,
        /// The token source for the route.
        token_source: TokenKind,
        /// The ISM for this route.
        ism: Ism,
    },
    /// Update an existing route with new admin or ISM.
    Update {
        /// The ID of the warp route on the local chain to update.
        warp_route: WarpRouteId,
        /// New authority that can modify the route.
        admin: Option<Admin<S>>,
        /// New ISM for this route.
        ism: Option<Ism>,
    },
    /// Add a counterparty router on another chain. This router is trusted. A malicious remote router can steal funds.
    /// Each warp route can have at most one remote router for a given destination domain.
    EnrollRemoteRouter {
        /// The ID of the warp route on the local chain.
        warp_route: WarpRouteId,
        /// The domain of the remote chain.
        remote_domain: u32,
        /// The router address on the remote chain.
        remote_router_address: HexHash,
    },
    /// Remove a counterparty router on another chain.
    UnEnrollRemoteRouter {
        /// The ID of the warp route on the local chain.
        warp_route: WarpRouteId,
        /// The domain of the remote chain.
        remote_domain: u32,
    },
    /// Transfer a token from the local chain to the remote chain.
    TransferRemote {
        /// The route to use for the transfer.
        warp_route: WarpRouteId,
        /// The domain of the destination chain.
        destination_domain: u32,
        /// The recipient on the destination chain.
        recipient: HexHash,
        /// The amount to transfer.
        amount: Amount,
        /// Selected relayer
        relayer: Option<S::Address>,
        /// A limit for the payment to relayer to cover gas needed for message delivery.
        gas_payment_limit: Amount,
    },
}

/// An event emitted by the Warp module.
#[derive(
    borsh::BorshDeserialize,
    borsh::BorshSerialize,
    serde::Serialize,
    serde::Deserialize,
    Debug,
    PartialEq,
    Clone,
    JsonSchema,
)]
#[serde(
    bound = "S::Address: serde::Serialize + serde::de::DeserializeOwned",
    rename_all = "snake_case"
)]
pub enum Event<S: Spec> {
    /// A route was registered.
    RouteRegistered {
        /// The ID of the new route.
        route_id: WarpRouteId,
        /// The token source for the route.
        token_source: StoredTokenKind,
        /// The ISM for this route.
        ism: Ism,
        /// The authority that can modify the route, if any.
        admin: Admin<S>,
    },
    /// A route was updated.
    RouteUpdated {
        /// The ID of the new route.
        route_id: WarpRouteId,
        /// New ISM for this route.
        updated_ism: Option<Ism>,
        /// New authority that can modify the route, if any.
        updated_admin: Option<Admin<S>>,
    },
    /// A remote router was enrolled.
    RouterEnrolled {
        /// The ID of the warp route on the local chain.
        route_id: WarpRouteId,
        /// The domain of the remote chain.
        domain: u32,
        /// The address of the remote router.
        router: HexHash,
    },
    /// A remote router was unenrolled.
    RouterUnEnrolled {
        /// The ID of the warp route on the local chain.
        route_id: WarpRouteId,
        /// The domain of the remote chain.
        domain: u32,
    },
    /// A token was transferred to the remote chain.
    TokenTransferredRemote {
        /// The ID of the warp route.
        route_id: WarpRouteId,
        /// The domain of the remote chain.
        to_domain: u32,
        /// The recipient on the remote chain, in hyperlane address format.
        recipient: HexHash,
        /// The amount transferred, in *remote* token units.
        amount: HexHash,
    },
    /// A token was received from the remote chain.
    TokenTransferReceived {
        /// The ID of the warp route.
        route_id: WarpRouteId,
        /// The domain of the remote chain.
        from_domain: u32,
        /// The recipient on the local chain, in hyperlane address format.
        recipient: HexHash,
        /// The amount transferred, in *local* token units.
        amount: Amount,
    },
}

impl<S: Spec> Module for Warp<S>
where
    S::Address: HyperlaneAddress,
{
    type Spec = S;
    type Config = ();
    type CallMessage = CallMessage<S>;
    type Event = Event<S>;

    fn call(
        &mut self,
        msg: Self::CallMessage,
        context: &Context<Self::Spec>,
        state: &mut impl TxState<S>,
    ) -> Result<(), Error> {
        match msg {
            CallMessage::Register {
                admin,
                token_source,
                ism,
            } => {
                self.register(admin, token_source, ism, context, state)?;
            }
            CallMessage::Update {
                warp_route,
                admin,
                ism,
            } => {
                self.update(warp_route, admin, ism, context, state)?;
            }
            CallMessage::EnrollRemoteRouter {
                warp_route,
                remote_domain,
                remote_router_address,
            } => {
                self.enroll_remote_router(
                    warp_route,
                    remote_domain,
                    remote_router_address,
                    context,
                    state,
                )?;
            }
            CallMessage::UnEnrollRemoteRouter {
                warp_route,
                remote_domain,
            } => {
                self.unenroll_remote_router(warp_route, remote_domain, context, state)?;
            }
            CallMessage::TransferRemote {
                warp_route,
                destination_domain,
                recipient,
                amount,
                relayer,
                gas_payment_limit,
            } => {
                self.transfer_remote(
                    warp_route,
                    destination_domain,
                    recipient,
                    amount,
                    relayer,
                    gas_payment_limit,
                    context,
                    state,
                )?;
            }
        }
        Ok(())
    }
}

/// Generate a warp route ID from the deployer and token source. Note that only the token ID and kind are included in the hash,
/// so the same ID will be generated even if the deployer changes the decimals or scale of the token.
fn generate_route_id<S: Spec>(token_source: &TokenKind, deployer: &S::Address) -> WarpRouteId {
    // TODO: Charge for hashing
    let mut hasher = <S::CryptoSpec as CryptoSpec>::Hasher::new();
    let (token_id, token_kind) = token_source.id_and_kind();
    hasher.update(borsh::to_vec(&token_id).expect("Serialization to vec is infallible"));
    hasher.update(borsh::to_vec(&token_kind).expect("Serialization to vec is infallible"));
    hasher.update(borsh::to_vec(&deployer).expect("Serialization to vec is infallible"));
    hasher.finalize().into()
}

impl<S: Spec> Warp<S>
where
    S::Address: HyperlaneAddress,
{
    /// Create a new warp route.
    ///
    /// A warp route is a set of contracts on different chains representing a single underlying token such as ETH or SOL.
    /// There may be many warp routes wrapping the same underlying asset, but each route
    /// only trusts other contracts that are enrolled by the route's admin.
    fn register(
        &mut self,
        admin: Admin<S>,
        token_source: TokenKind,
        ism: Ism,
        context: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> anyhow::Result<()> {
        let route_id = generate_route_id::<S>(&token_source, context.sender());
        // Each deployer can only have one route per token source
        if self.warp_routes.get(&route_id, state)?.is_some() {
            let (token_id, token_kind) = token_source.id_and_kind();
            anyhow::bail!(
                "A route for token {} (origin: {}) was already registered by sender {}",
                token_id,
                token_kind.as_ref(),
                context.sender()
            );
        }

        // Deploy a synthetic token to represent the bridged asset if necessary.
        // (Use an exhaustive match in case more token sources are added.)
        let stored_token_kind = match &token_source {
            TokenKind::Synthetic {
                remote_token_id,
                local_decimals,
                remote_decimals,
            } => {
                let local_decimals = local_decimals.unwrap_or(*remote_decimals);
                let local_token_id =
                    self.deploy_synthetic_token(route_id, local_decimals, state)?;

                StoredTokenKind::Synthetic {
                    remote_token_id: *remote_token_id,
                    local_token_id,
                    local_decimals,
                    remote_decimals: *remote_decimals,
                }
            }
            TokenKind::Collateral { token } => StoredTokenKind::Collateral { token: *token },
            TokenKind::Native => StoredTokenKind::Native,
        };

        self.warp_routes.set(
            &route_id,
            &WarpRouteInstance {
                token_source: stored_token_kind.clone(),
                admin: admin.clone(),
                ism: ism.clone(),
            },
            state,
        )?;

        self.emit_event(
            state,
            Event::RouteRegistered {
                route_id,
                token_source: stored_token_kind,
                ism,
                admin,
            },
        );

        Ok(())
    }

    /// Update an existing warp route.
    ///
    /// Can be used to transfer / drop ownership of a route or to change the ISM (e.g. changing
    /// a list of trusted validators)
    fn update(
        &mut self,
        route_id: WarpRouteId,
        admin: Option<Admin<S>>,
        ism: Option<Ism>,
        context: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> anyhow::Result<()> {
        // Get the route that should be updated
        let mut route = self
            .warp_routes
            .borrow_mut(&route_id, state)?
            .ok_or_else(|| {
                anyhow::anyhow!("Attepmted to change nonexistent warp route '{route_id}'")
            })?;

        anyhow::ensure!(
            admin.is_some() || ism.is_some(),
            "Update should contain new admin or ism"
        );

        // Check if the request is properly authorized
        match &route.admin {
            Admin::None => anyhow::bail!("Cannot update immutable route {}", route_id),
            Admin::InsecureOwner(admin) => anyhow::ensure!(
                admin == context.sender(),
                "Cannot update route with authorization from {}. Route {} is owned by {}",
                context.sender(),
                route_id,
                admin
            ),
        }

        // Update the route
        if let Some(admin) = admin.clone() {
            route.admin = admin;
        }
        if let Some(ism) = ism.clone() {
            route.ism = ism;
        }
        route.save(state)?;

        self.emit_event(
            state,
            Event::RouteUpdated {
                route_id,
                updated_admin: admin,
                updated_ism: ism,
            },
        );

        Ok(())
    }

    fn deploy_synthetic_token(
        &mut self,
        warp_route: WarpRouteId,
        decimals: u8,
        state: &mut impl TxState<S>,
    ) -> anyhow::Result<TokenId> {
        let holder: DerivedHolder = warp_route.0.into();
        self.bank.create_token(
            format!("Synthetic token for {}", warp_route),
            Some(decimals),
            Amount::ZERO,              // No initial balance
            holder.to_payable(), // Mint the initial balance to the warp route. Since it's 0, this doesn't matter - but use the holder anyway
            vec![holder.to_payable()], // The warp route is the only admin
            None,                // No supply cap
            holder.to_payable(), // The mint authority is the warp route
            state,
        )
    }

    /// "Enroll" a remote router on another chain. Whenever this route needs to send/receive funds on that chain,
    /// it will use the remote "router" contract registered for that domain as the counterparty.
    ///
    /// Note that remote routers are *trusted*. If the remote router is compromised, it can steal funds from the warp route.
    fn enroll_remote_router(
        &mut self,
        warp_route: WarpRouteId,
        remote_domain: u32,
        remote_router_address: HexHash,
        context: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> anyhow::Result<()> {
        let Some(warp_route_instance) = self.warp_routes.get(&warp_route, state)? else {
            anyhow::bail!("Warp route {} not found", warp_route);
        };
        match warp_route_instance.admin {
            Admin::None => anyhow::bail!("Cannot enroll router. Route {} is immutable", warp_route),
            Admin::InsecureOwner(admin) => anyhow::ensure!(
                &admin == context.sender(),
                "Cannot enroll router with authorization from {}. Route {} is owned by {}",
                context.sender(),
                warp_route,
                admin
            ),
        }

        let router_key = RouterKey {
            route_id: warp_route,
            remote_domain,
        };
        anyhow::ensure!(
            self.routers.get(&router_key, state)?.is_none(),
            "Router already enrolled for {}",
            router_key,
        );
        self.routers.set(
            &router_key,
            &RemoteRouterAddress(remote_router_address),
            state,
        )?;
        self.emit_event(
            state,
            Event::RouterEnrolled {
                route_id: warp_route,
                domain: remote_domain,
                router: remote_router_address,
            },
        );
        Ok(())
    }

    /// "Unenroll" a remote router on another chain. See the docs on [`Warp::enroll_remote_router`] for more details.
    fn unenroll_remote_router(
        &mut self,
        warp_route: WarpRouteId,
        remote_domain: u32,
        context: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> anyhow::Result<()> {
        let Some(warp_route_instance) = self.warp_routes.get(&warp_route, state)? else {
            anyhow::bail!("Warp route {} not found", warp_route);
        };
        match warp_route_instance.admin {
            Admin::None => {
                anyhow::bail!("Cannot unenroll router. Route {} is immutable", warp_route)
            }
            Admin::InsecureOwner(admin) => anyhow::ensure!(
                &admin == context.sender(),
                "Cannot unenroll router with authorization from {}. Route {} is owned by {}",
                context.sender(),
                warp_route,
                admin
            ),
        }

        let router_key = RouterKey {
            route_id: warp_route,
            remote_domain,
        };
        anyhow::ensure!(
            self.routers.get(&router_key, state)?.is_some(),
            "Router {} does not exist",
            router_key,
        );

        self.routers.remove(&router_key, state)?;
        self.emit_event(
            state,
            Event::RouterUnEnrolled {
                route_id: warp_route,
                domain: remote_domain,
            },
        );
        Ok(())
    }

    /// Transfer a token from the local chain to the remote chain.
    #[allow(clippy::too_many_arguments)]
    fn transfer_remote(
        &mut self,
        warp_route: WarpRouteId,
        destination_domain: u32,
        recipient: HexHash,
        amount: Amount,
        relayer: Option<S::Address>,
        gas_payment_limit: Amount,
        context: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> anyhow::Result<()> {
        let route = self
            .warp_routes
            .get(&warp_route, state)?
            .ok_or_else(|| anyhow::anyhow!("Warp route {} not found", warp_route))?;
        let remote = self
            .routers
            .get(
                &RouterKey {
                    route_id: warp_route,
                    remote_domain: destination_domain,
                },
                state,
            )?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Route {} exists but does not have remote router for domain {}",
                    warp_route,
                    destination_domain
                )
            })?;

        let route_token_holder: DerivedHolder = warp_route.0.into();
        // Do the appropriate burn or transfer for the token source
        match &route.token_source {
            StoredTokenKind::Synthetic { local_token_id, .. } => {
                // If the local token is synthetic, burn the local token
                self.bank.burn(
                    Coins {
                        amount,
                        token_id: *local_token_id,
                    },
                    context.sender(),
                    state,
                )?;
            }
            StoredTokenKind::Collateral { token } => {
                // If the local token is collateral, transfer from the sender to the route
                self.bank.transfer(
                    route_token_holder.to_payable(),
                    Coins {
                        amount,
                        token_id: *token,
                    },
                    context,
                    state,
                )?;
            }
            StoredTokenKind::Native => {
                // If the local token is native, transfer the native token from the sender to the route
                self.bank.transfer(
                    route_token_holder.to_payable(),
                    Coins {
                        amount,
                        token_id: config_gas_token_id(),
                    },
                    context,
                    state,
                )?;
            }
        };

        let body = Self::pack_transfer_body(recipient, amount, &route.token_source)?;
        let remote_amount = HexString(route.token_source.outbound_amount(amount)?);
        self.emit_event(
            state,
            Event::TokenTransferredRemote {
                route_id: warp_route,
                to_domain: destination_domain,
                recipient,
                amount: remote_amount,
            },
        );

        Mailbox::<S, Self>::default().dispatch(
            destination_domain,
            remote.0,
            warp_route, // Use the route ID as the sender
            HexString(body),
            None,
            relayer,
            gas_payment_limit,
            context,
            state,
        )?;

        Ok(())
    }

    /// Pack a transfer body for a message.
    pub fn pack_transfer_body(
        recipient: HexHash,
        local_amount: Amount,
        token_source: &StoredTokenKind,
    ) -> anyhow::Result<Vec<u8>> {
        let mut out = Vec::with_capacity(64);
        out.extend_from_slice(&recipient.0);
        let amount = token_source.outbound_amount(local_amount)?; // Convert the local amount to the remote token amount
        out.extend_from_slice(&amount);
        Ok(out)
    }

    fn unpack_transfer_body(
        &self,
        body: &[u8],
        token_source: &StoredTokenKind,
    ) -> anyhow::Result<(HexHash, Amount)> {
        anyhow::ensure!(body.len() == 64, "Invalid transfer body");
        let recipient = HexString(body[..32].try_into().unwrap());
        let amount = body[32..64].try_into().unwrap();
        let amount = token_source.inbound_amount(amount)?;
        Ok((recipient, amount))
    }
}

impl<S: Spec> Recipient<S> for Warp<S>
where
    S::Address: HyperlaneAddress,
{
    fn ism(&self, recipient: &HexHash, state: &mut impl TxState<S>) -> anyhow::Result<Option<Ism>> {
        Ok(self.warp_routes.get(recipient, state)?.map(|r| r.ism))
    }

    /// Handles an inbound message. Note that this deviates from more standard Hyperlane `handle` API because all messages
    /// are dispatched through this module regardless of their ultimate destination, so we need to explicitly pass the recipient as an argument.
    fn handle(
        &mut self,
        origin: u32,
        sender: HexHash,
        route_id: &HexHash,
        body: HexString,
        state: &mut impl TxState<S>,
    ) -> anyhow::Result<()> {
        // Get the route instance
        let route = self
            .warp_routes
            .get(route_id, state)?
            .ok_or_else(|| anyhow::anyhow!("Warp route {} not found", route_id))?;

        let router = self
            .routers
            .get(
                &RouterKey {
                    route_id: *route_id,
                    remote_domain: origin,
                },
                state,
            )?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Remote router for route {} and origin {} not found",
                    route_id,
                    origin
                )
            })?;

        // Only accept messages from the enrolled router
        if router != RemoteRouterAddress(sender) {
            anyhow::bail!("Enrolled router does not match sender");
        }

        let (token_recipient_hex, amount) =
            self.unpack_transfer_body(body.as_ref(), &route.token_source)?;
        let token_recipient = S::Address::from_sender(token_recipient_hex)?;
        let route_token_holder: DerivedHolder = route_id.0.into();
        match &route.token_source {
            StoredTokenKind::Synthetic { local_token_id, .. } => {
                // If the local token is synthetic, mint to the recipient
                self.bank.mint(
                    Coins {
                        amount,
                        token_id: *local_token_id,
                    },
                    &token_recipient,
                    route_token_holder.to_payable(),
                    state,
                )?;
            }
            StoredTokenKind::Collateral { token } => {
                // If the local token is collateral, transfer from the route to the recipient
                self.bank.transfer_from(
                    route_token_holder.to_payable(),
                    &token_recipient,
                    Coins {
                        amount,
                        token_id: *token,
                    },
                    state,
                )?;
            }
            StoredTokenKind::Native => {
                // If the local token is native, transfer the native token from the route to the recipient
                self.bank.transfer_from(
                    route_token_holder.to_payable(),
                    &token_recipient,
                    Coins {
                        amount,
                        token_id: config_gas_token_id(),
                    },
                    state,
                )?;
            }
        };

        self.emit_event(
            state,
            Event::TokenTransferReceived {
                route_id: *route_id,
                from_domain: origin,
                recipient: token_recipient_hex,
                amount,
            },
        );

        Ok(())
    }

    /// Each warp route must have a dedicated ISM set and only they are recipients.
    fn default_ism(&self, _state: &mut impl TxState<S>) -> anyhow::Result<Option<Ism>> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    #[test]
    fn test_router_key() {
        const KEY_STR: &str =
            "0xb4c11951957c6f8f642c4af61cd6b24640fec6dc7fc607ee8206a99e92410d30:2321";
        let router_key = RouterKey::from_str(KEY_STR).unwrap();
        assert_eq!(
            router_key.route_id,
            HexString::from_str(
                "0xb4c11951957c6f8f642c4af61cd6b24640fec6dc7fc607ee8206a99e92410d30"
            )
            .unwrap()
        );
        assert_eq!(router_key.remote_domain, 2321);
        let router_key_str = router_key.to_string();
        assert_eq!(router_key_str, KEY_STR);
    }
}
