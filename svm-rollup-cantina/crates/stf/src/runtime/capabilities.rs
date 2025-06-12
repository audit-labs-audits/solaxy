use std::convert::Infallible;

use sov_address::FromVmAddress;
#[cfg(feature = "native")]
use sov_attester_incentives::BondingProofServiceImpl;
use sov_bank::{config_gas_token_id, utils::TokenHolderRef, Amount, Coins, IntoPayable, Payable};
#[cfg(feature = "native")]
use sov_modules_api::{capabilities::HasKernel, StateUpdateInfo};
use sov_modules_api::{
    capabilities::{
        AuthorizationData, GasEnforcer, Guard, HasCapabilities, ProofProcessor,
        SequencerAuthorization, SequencerRemuneration, TransactionAuthorizer,
    },
    prelude::*,
    transaction::{AuthenticatedTransactionData, ProverReward, RemainingFunds, SequencerReward},
    AggregatedProofPublicData, DaSpec, Gas, GetGasPrice, InfallibleStateAccessor,
    InvalidProofError, ModuleInfo, Rewards, SerializedAggregatedProof, SovAttestation,
    SovStateTransitionPublicData, Spec, StateReader, StateWriter, TxState,
};
use sov_rollup_interface::common::SlotNumber;
use sov_sequencer_registry::SequencerRegistry;
use sov_state::{Kernel, Storage, User};
use svm_types::SolanaAddress;

use super::Runtime;

/// The capabilities required for a SVM zk-rollup runtime.
pub struct SVMRollupCapabilities<'a, S: Spec>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    /// The Bank module.
    pub bank: &'a mut sov_bank::Bank<S>,
    /// The Sequencer Registry module.
    pub sequencer_registry: &'a mut SequencerRegistry<S>,
    /// The Accounts module.
    pub accounts: &'a mut sov_accounts::Accounts<S>,
    /// The Prover Incentives module.
    pub prover_incentives: &'a mut sov_prover_incentives::ProverIncentives<S>,
    /// The Attester Incentives module.
    pub attester_incentives: &'a mut sov_attester_incentives::AttesterIncentives<S>,
}

impl<'a, S: Spec> SVMRollupCapabilities<'a, S>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    fn get_prover_token_holder(
        &'a self,
        state: &mut impl InfallibleStateAccessor,
    ) -> TokenHolderRef<'a, S> {
        let reward_prover_incentives = self.prover_incentives.should_reward_fees(state);
        let reward_attester_incentives = self.attester_incentives.should_reward_fees(state);

        assert!(
            reward_prover_incentives ^ reward_attester_incentives,
            "Exactly one of prover or attester incentives should be rewarded"
        );

        let rewarded_module = if reward_prover_incentives {
            self.prover_incentives.id().to_payable()
        } else {
            self.attester_incentives.id().to_payable()
        };

        rewarded_module
    }
}

impl<S: Spec> GasEnforcer<S> for SVMRollupCapabilities<'_, S>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    /// Reserves enough gas for the transaction to be processed, if possible.
    fn try_reserve_gas(
        &mut self,
        tx: &AuthenticatedTransactionData<S>,
        gas_price: &<S::Gas as Gas>::Price,
        context: &mut Context<S>,
        state: &mut impl StateAccessor,
    ) -> anyhow::Result<()> {
        self.bank
            .reserve_gas(tx, gas_price, context.sender(), state)
            .map_err(Into::into)
    }

    fn try_reserve_gas_for_proof(
        &mut self,
        tx: &AuthenticatedTransactionData<S>,
        gas_price: &<<S as Spec>::Gas as Gas>::Price,
        sender: &<S as Spec>::Address,
        scratchpad: &mut impl StateAccessor,
    ) -> anyhow::Result<()> {
        self.bank
            .reserve_gas(tx, gas_price, sender, scratchpad)
            .map_err(Into::into)
    }

    fn reward_prover(
        &mut self,
        prover_reward: &ProverReward,
        state: &mut impl InfallibleStateAccessor,
    ) {
        let rewarded_module = self.get_prover_token_holder(state);

        self.bank
            .transfer_from(
                self.bank.id().clone().to_payable(),
                rewarded_module.to_owned().as_token_holder(),
                Coins {
                    amount: prover_reward.0,
                    token_id: config_gas_token_id(),
                },
                state,
            )
            // SAFETY: It is safe to unwrap here because the caller must ensure that sufficient funds are reserved.
            .expect("Caller failed to ensure sufficient funds are reserved, but this is required for reward_prover to remain infallible");
    }

    fn refund_remaining_gas(
        &mut self,
        recipient: &S::Address,
        remaining_funds: &RemainingFunds,
        state: &mut impl InfallibleStateAccessor,
    ) {
        // We refund the payer. We need to give back the remaining funds on the gas meter, plus the unspent tip.
        // This is also the maximum fee minus everything that was spent for the tip and base fee (ie the total reward).
        self.bank
            .transfer_from(
                self.bank.id().clone().to_payable(),
                recipient,
                Coins {
                    amount: remaining_funds.0,
                    token_id: config_gas_token_id(),
                },
                state,
            )
            // SAFETY: It is safe to unwrap here because the caller must ensure that sufficient funds are reserved.
            .expect("Caller failed to ensure sufficient funds are reserved, but this is required for refund_remaining_gas to remain infallible");
    }

    fn reward_prover_from_sequencer_balance(
        &mut self,
        amount: Amount,
        _sequencer: &S::Address,
        state: &mut impl InfallibleStateAccessor,
    ) -> anyhow::Result<()> {
        let rewarded_prover_module = self.get_prover_token_holder(state);
        // Transfer the penalty from the sequencer bank to the sequencer
        self.bank.transfer_from(
            self.bank.id().clone().to_payable(),
            rewarded_prover_module.to_owned(),
            Coins {
                amount,
                token_id: config_gas_token_id(),
            },
            state,
        )
    }

    fn return_escrowed_funds_to_sequencer<
        Accessor: StateReader<Kernel, Error = Infallible>
            + StateWriter<Kernel, Error = Infallible>
            + StateWriter<User, Error = Infallible>
            + StateReader<User, Error = Infallible>,
    >(
        &mut self,
        bond_amount: Amount,
        reward: Rewards,
        sequencer: &<S::Da as DaSpec>::Address,
        state: &mut Accessor,
    ) {
        let mut net_amount = bond_amount
            .checked_sub(reward.accumulated_penalty)
            .expect("A sequencer can never be penalized more than the amount they have escrowed, regardless of reward accumulation!");
        net_amount = net_amount
            .checked_add(reward.accumulated_reward)
            .expect("Total sequencer reward + escrow amount is greater than the max possible token supply. This is a bug in gas accounting.");

        self.sequencer_registry.add_to_stake(
            self.bank.id().to_payable(),
            sequencer,
            net_amount,
            state,
        ).expect("Attempted to send more funds to the sequencer than they have escrowed. This is a bug in gas accounting.");
    }
}

impl<S: Spec> SequencerAuthorization<S> for SVMRollupCapabilities<'_, S>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    fn is_preferred_sequencer(
        &self,
        sequencer: &<S::Da as DaSpec>::Address,
        state: &mut impl InfallibleStateAccessor,
    ) -> bool {
        self.sequencer_registry.preferred_sequencer(state).as_ref() == Some(sequencer)
    }
}

impl<S: Spec> TransactionAuthorizer<S> for SVMRollupCapabilities<'_, S>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    /// Prevents duplicate transactions from running.
    fn check_uniqueness(
        &self,
        _auth_data: &AuthorizationData<S>,
        _context: &Context<S>,
        _state: &mut impl StateReader<User>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// Marks a transaction as having been executed, preventing it from executing again.
    fn mark_tx_attempted(
        &mut self,
        _auth_data: &AuthorizationData<S>,
        _sequencer: &<S::Da as DaSpec>::Address,
        _state: &mut impl StateAccessor,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// Resolves the context for a transaction.
    fn resolve_context(
        &mut self,
        auth_data: &AuthorizationData<S>,
        sequencer: &<S::Da as DaSpec>::Address,
        sequencer_rollup_address: S::Address,
        state: &mut impl StateAccessor,
    ) -> anyhow::Result<Context<S>> {
        // This should be resolved by the sequencer registry during blob selection
        let sender = self.accounts.resolve_sender_address(
            &auth_data.default_address,
            &auth_data.credential_id,
            state,
        )?;
        Ok(Context::new(
            sender,
            auth_data.credentials.clone(),
            sequencer_rollup_address,
            sequencer.clone(),
        ))
    }

    fn resolve_unregistered_context(
        &mut self,
        auth_data: &AuthorizationData<S>,
        sequencer: &<<S as Spec>::Da as DaSpec>::Address,
        state: &mut impl StateAccessor,
    ) -> anyhow::Result<Context<S>> {
        let sender = self.accounts.resolve_sender_address(
            &auth_data.default_address,
            &auth_data.credential_id,
            state,
        )?;
        // The tx sender & sequencer are the same entity
        Ok(Context::new(
            sender.clone(),
            auth_data.credentials.clone(),
            sender,
            sequencer.clone(),
        ))
    }
}

impl<S: Spec> ProofProcessor<S> for SVMRollupCapabilities<'_, S>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    #[cfg(feature = "native")]
    type BondingProofService<K: HasKernel<S>> = BondingProofServiceImpl<S, K>;

    #[cfg(feature = "native")]
    fn create_bonding_proof_service<K: HasKernel<S>>(
        &self,
        attester_address: <S as Spec>::Address,
        state_update_info: sov_modules_api::prelude::tokio::sync::watch::Receiver<
            StateUpdateInfo<<S as Spec>::Storage>,
        >,
    ) -> Self::BondingProofService<K> {
        use sov_attester_incentives::BondingProofServiceImpl;

        BondingProofServiceImpl::new(
            attester_address,
            self.attester_incentives.clone(),
            state_update_info,
        )
    }

    #[allow(clippy::type_complexity)]
    fn process_aggregated_proof<ST: TxState<S> + GetGasPrice<Spec = S>>(
        &mut self,
        proof: SerializedAggregatedProof,
        prover_address: &S::Address,
        state: &mut ST,
    ) -> Result<
        (
            AggregatedProofPublicData<S::Address, S::Da, <S::Storage as Storage>::Root>,
            SerializedAggregatedProof,
        ),
        InvalidProofError,
    > {
        let result = self
            .prover_incentives
            .process_proof(&proof, prover_address, state)?;

        Ok((result, proof))
    }

    fn process_attestation<ST: TxState<S> + GetGasPrice<Spec = S>>(
        &mut self,
        proof: sov_rollup_interface::optimistic::SerializedAttestation,
        prover_address: &<S as Spec>::Address,
        state: &mut ST,
    ) -> Result<SovAttestation<S>, InvalidProofError> {
        let result = self
            .attester_incentives
            .process_attestation(prover_address, proof, state)?;

        Ok(result)
    }

    fn process_challenge<ST: TxState<S> + GetGasPrice<Spec = S>>(
        &mut self,
        proof: sov_rollup_interface::optimistic::SerializedChallenge,
        rollup_height: SlotNumber,
        prover_address: &<S as Spec>::Address,
        state: &mut ST,
    ) -> Result<SovStateTransitionPublicData<S>, InvalidProofError> {
        let result = self.attester_incentives.process_challenge(
            prover_address,
            &proof,
            rollup_height,
            state,
        )?;

        Ok(result)
    }
}

impl<S: Spec> SequencerRemuneration<S> for SVMRollupCapabilities<'_, S>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    fn reward_sequencer_or_refund<
        Accessor: StateReader<Kernel, Error = Infallible>
            + StateWriter<Kernel, Error = Infallible>
            + StateWriter<User, Error = Infallible>
            + StateReader<User, Error = Infallible>,
    >(
        &mut self,
        sequencer: &<S::Da as DaSpec>::Address,
        sequencer_rollup_address: &S::Address,
        reward: SequencerReward,
        state: &mut Accessor,
    ) {
        let stake_increased = self.sequencer_registry.add_to_stake(
            self.bank.id().to_payable(),
            sequencer,
            reward.0,
            state,
        );

        // The error indicates that the forced registration was reverted.
        // In this case, we will refund the rewards to the user.
        if stake_increased.is_err() {
            self.bank
                .transfer_from(
                    self.bank.id.clone().to_payable(),
                    sequencer_rollup_address.as_token_holder(),
                    Coins { amount:reward.0, token_id: config_gas_token_id()},
                    state,
                )
                // SAFETY: It is safe to unwrap here because the caller must ensure that sufficient funds are reserved.
                .expect("Caller failed to ensure sufficient funds are reserved. Transferring the consumed base fee gas is infallible");
        }
    }

    fn preferred_sequencer(
        &self,
        scratchpad: &mut impl InfallibleStateAccessor,
    ) -> Option<<S::Da as DaSpec>::Address> {
        self.sequencer_registry.preferred_sequencer(scratchpad)
    }
}

impl<S: Spec> HasCapabilities<S> for Runtime<S>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    type Capabilities<'a> = SVMRollupCapabilities<'a, S>;

    fn capabilities(&mut self) -> Guard<Self::Capabilities<'_>> {
        Guard::new(SVMRollupCapabilities {
            bank: &mut self.bank,
            sequencer_registry: &mut self.sequencer_registry,
            accounts: &mut self.accounts,
            prover_incentives: &mut self.prover_incentives,
            attester_incentives: &mut self.attester_incentives,
        })
    }
}
