mod da_simulation;
mod registered;
mod sequencer;
mod stf_tests;
mod tx_revert_tests;
mod unregistered;
use std::env;

use sov_bank::Bank;
use sov_mock_da::{MockAddress, MockBlob, MockDaSpec};
use sov_modules_api::capabilities::TransactionAuthenticator;
use sov_modules_api::prelude::UnwrapInfallible;
use sov_modules_api::transaction::{
    PriorityFeeBips, Transaction, TxDetails, UnsignedTransaction, VersionedTx,
};
use sov_modules_api::{
    Amount, ApiStateAccessor, DaSpec, FullyBakedTx, Gas, PrivateKey, RawTx, Rewards, Spec,
};
use sov_modules_stf_blueprint::{BatchReceipt, Runtime};
use sov_sequencer_registry::{AllowedSequencerError, SequencerRegistry};
use sov_test_utils::runtime::genesis::optimistic::HighLevelOptimisticGenesisConfig;
use sov_test_utils::runtime::{config_gas_token_id, Payable, TestRunner};
use sov_test_utils::{
    generate_optimistic_runtime, TestSequencer, TestUser, TEST_DEFAULT_MAX_FEE,
    TEST_DEFAULT_USER_BALANCE,
};
use sov_value_setter::ValueSetter;

type S = sov_test_utils::TestSpec;
type RT = IntegTestRuntime<S>;
type Call = IntegTestRuntimeCall<S>;

generate_optimistic_runtime!(IntegTestRuntime <= value_setter: ValueSetter<S>);

fn get_balance(payable: impl Payable<S>, state: &mut ApiStateAccessor<S>) -> Amount {
    Bank::<S>::default()
        .get_balance_of(payable, config_gas_token_id(), state)
        .unwrap_infallible()
        .unwrap()
}

fn get_seq_bond(
    sequencer_da_address: &<<S as Spec>::Da as DaSpec>::Address,
    state: &mut ApiStateAccessor<S>,
) -> Result<Amount, AllowedSequencerError> {
    let sequencer_module = SequencerRegistry::<S>::default();
    sequencer_module
        .is_sender_allowed(sequencer_da_address, state)
        .map(|s| s.balance)
}

#[allow(clippy::type_complexity)]
fn setup(
    nb_of_users: usize,
) -> (
    TestRunner<IntegTestRuntime<S>, S>,
    Vec<TestUser<S>>,
    TestSequencer<S>,
) {
    let mut genesis_config = HighLevelOptimisticGenesisConfig::generate();

    for _ in 0..nb_of_users {
        genesis_config
            .additional_accounts
            .push(TestUser::<S>::generate(TEST_DEFAULT_USER_BALANCE));
    }
    let admin = genesis_config.additional_accounts[0].address();

    let genesis = GenesisConfig::from_minimal_config(
        genesis_config.clone().into(),
        sov_value_setter::ValueSetterConfig { admin },
    );

    let runner: TestRunner<IntegTestRuntime<S>, S> =
        TestRunner::new_with_genesis(genesis.into_genesis_params(), Default::default());

    let sequencer_account = genesis_config.initial_sequencer.clone();

    (
        runner,
        genesis_config.additional_accounts,
        sequencer_account,
    )
}

#[derive(PartialEq, Eq)]
pub(crate) enum TxStatus {
    Success,
    BadGeneration,
    BadChainId,
    BadSignature,
    BadSerialization,
    SignerDoesNotExist,
    OutOfGas,
    Reverted,
}

impl TxStatus {
    fn nb_of_valid_txs(statuses: &[TxStatus]) -> usize {
        statuses
            .iter()
            .filter(|s| s.is_valid())
            .collect::<Vec<_>>()
            .len()
    }

    fn nb_of_skipped_txs(statuses: &[TxStatus]) -> usize {
        statuses
            .iter()
            .filter(|s| !s.is_valid())
            .collect::<Vec<_>>()
            .len()
    }

    fn is_valid(&self) -> bool {
        // Reverted transactions pass the authentication process; therefore, we count them as valid.
        matches!(self, TxStatus::Success | TxStatus::Reverted)
    }
}

fn create_tx_bad_sig(
    nonce: u64,
    max_priority_fee_bips: PriorityFeeBips,
    signer: &TestUser<S>,
    chain_id: u64,
    message: Call,
) -> Transaction<RT, S> {
    let utx = UnsignedTransaction::<RT, S>::new(
        message,
        chain_id,
        max_priority_fee_bips,
        TEST_DEFAULT_MAX_FEE,
        nonce,
        None,
    );

    let signed_tx =
        Transaction::new_signed_tx(&signer.private_key, &IntegTestRuntime::<S>::CHAIN_HASH, utx);

    // Create a signature for a different message so it won't verify in the stf.
    let bad_signature = signer.private_key.sign(&[1, 2, 3]);

    match signed_tx.versioned_tx {
        VersionedTx::V0(inner) => Transaction::new_with_details_v0(
            inner.pub_key.clone(),
            inner.runtime_call.clone(),
            bad_signature,
            inner.generation,
            TxDetails {
                max_priority_fee_bips,
                max_fee: Amount::new(200_000),
                gas_limit: None,
                chain_id,
            },
        ),
    }
}

fn create_tx_bad_sender(
    nonce: u64,
    max_priority_fee_bips: PriorityFeeBips,
    chain_id: u64,
    message: Call,
) -> Transaction<RT, S> {
    let utx = UnsignedTransaction::new(
        message,
        chain_id,
        max_priority_fee_bips,
        Amount::new(200_000),
        nonce,
        None,
    );

    let signer = TestUser::<S>::generate(Amount::ZERO);
    Transaction::<RT, S>::new_signed_tx(
        signer.private_key(),
        &IntegTestRuntime::<S>::CHAIN_HASH,
        utx,
    )
}

fn create_tx_valid(
    nonce: u64,
    max_priority_fee_bips: PriorityFeeBips,
    signer: &TestUser<S>,
    chain_id: u64,
    message: Call,
) -> Transaction<RT, S> {
    let utx = UnsignedTransaction::new(
        message,
        chain_id,
        max_priority_fee_bips,
        TEST_DEFAULT_MAX_FEE,
        nonce,
        None,
    );

    Transaction::<RT, S>::new_signed_tx(
        signer.private_key(),
        &<IntegTestRuntime<S>>::CHAIN_HASH,
        utx,
    )
}

// Transaction with zero gas limit.
fn create_tx_out_of_gas(
    nonce: u64,
    max_priority_fee_bips: PriorityFeeBips,
    signer: &TestUser<S>,
    chain_id: u64,
    message: Call,
) -> Transaction<RT, S> {
    let utx = UnsignedTransaction::new(
        message,
        chain_id,
        max_priority_fee_bips,
        Amount::new(200_000),
        nonce,
        Some(<<S as Spec>::Gas as Gas>::zero()),
    );

    Transaction::<RT, S>::new_signed_tx(
        signer.private_key(),
        &IntegTestRuntime::<S>::CHAIN_HASH,
        utx,
    )
}

/// Builds a [`MockBlob`] from a [`Batch`] and a given address.
pub fn new_test_blob_from_batch(
    batch: Vec<FullyBakedTx>,
    address: &[u8],
) -> <MockDaSpec as DaSpec>::BlobTransaction {
    let address = MockAddress::try_from(address).unwrap();
    let data = borsh::to_vec(&batch).unwrap();
    MockBlob::new_with_hash(data, address)
}

/// Builds a new test blob for direct sequencer registration.
pub fn new_test_blob_for_direct_registration(
    tx: RawTx,
    address: &[u8],
    hash: [u8; 32],
) -> <MockDaSpec as DaSpec>::BlobTransaction {
    let batch = tx;
    let address = MockAddress::try_from(address).unwrap();
    let data = borsh::to_vec(&batch).unwrap();
    MockBlob::new(data, address, hash)
}

/// Checks if the given [`BatchReceipt`] contains any events.
pub fn has_tx_events<S: Spec>(apply_blob_outcome: &BatchReceipt<S>) -> bool {
    let events = apply_blob_outcome
        .tx_receipts
        .iter()
        .flat_map(|receipts| receipts.events.iter());

    events.peekable().peek().is_some()
}

fn default_rewards() -> Rewards {
    Rewards {
        accumulated_reward: Amount::ZERO,
        accumulated_penalty: Amount::ZERO,
    }
}

pub(crate) fn reset_constants() {
    env::set_var(
        "SOV_TEST_CONST_OVERRIDE_DEFAULT_GAS_TO_CHARGE_PER_BYTE_BORSH_DESERIALIZATION",
        "[1, 1]",
    );
    env::set_var(
        "SOV_TEST_CONST_OVERRIDE_MAX_ALLOWED_DATA_SIZE_RETURNED_BY_BLOB_STORAGE",
        "10000000",
    );

    env::set_var(
        "SOV_TEST_CONST_OVERRIDE_MAX_ALLOWED_DATA_SIZE_RETURNED_BY_BLOB_STORAGE",
        "10000000",
    );

    env::set_var(
        "SOV_TEST_CONST_OVERRIDE_MAX_UNREGISTERED_SEQUENCER_EXEC_GAS_PER_TX",
        "[10000000,10000000]",
    );
}

pub(crate) mod helpers {
    use sov_attester_incentives::AttesterIncentives;
    use sov_bank::IntoPayable;
    use sov_modules_api::macros::config_value;
    use sov_modules_api::transaction::PriorityFeeBips;
    use sov_modules_api::{Amount, DaSpec, FullyBakedTx, ModuleInfo, Spec};
    use sov_test_utils::{EncodeCall, TestSequencer, TestUser};
    use sov_value_setter::{CallMessage, ValueSetter};

    use super::{IntegTestRuntime, *};
    use crate::stf_blueprint::{
        create_tx_bad_sender, create_tx_bad_sig, create_tx_out_of_gas, create_tx_valid,
        IntegTestRuntimeCall,
    };

    pub(crate) struct Actors {
        pub(crate) admin_account: TestUser<S>,
        pub(crate) not_admin_account: TestUser<S>,
        pub(crate) sequencer_account: TestSequencer<S>,
    }

    impl Actors {
        pub(crate) fn balances(&self, state: &mut ApiStateAccessor<S>) -> Balances {
            let attester_module = AttesterIncentives::<S>::default();
            Balances {
                admin_balance: get_balance(&self.admin_account.address(), state),
                not_admin_balance: get_balance(&self.not_admin_account.address(), state),
                attester_module_balance: get_balance(attester_module.id().to_payable(), state),
                sequencer_bond: get_seq_bond(&self.sequencer_account.da_address, state).unwrap(),
            }
        }
    }

    #[derive(Debug, Eq, PartialEq)]
    pub(crate) struct Balances {
        pub(crate) admin_balance: Amount,
        pub(crate) not_admin_balance: Amount,
        pub(crate) attester_module_balance: Amount,
        pub(crate) sequencer_bond: Amount,
    }

    impl Balances {
        pub(crate) fn total_balance(&self) -> Amount {
            self.admin_balance
                .checked_add(self.not_admin_balance)
                .unwrap()
                .checked_add(self.sequencer_bond)
                .unwrap()
                .checked_add(self.attester_module_balance)
                .unwrap()
        }
    }

    pub(crate) fn create_txs(
        statuses: &[TxStatus],
        max_priority_fee_bips: PriorityFeeBips,
        admin: &TestUser<S>,
        not_admin: &TestUser<S>,
    ) -> Vec<FullyBakedTx> {
        let mut generation = 10;
        let mut txs: Vec<FullyBakedTx> = Vec::new();
        for status in statuses {
            match status {
                TxStatus::Success => {
                    let tx = create_tx_valid(
                        generation,
                        max_priority_fee_bips,
                        admin,
                        config_value!("CHAIN_ID"),
                        encode_message(None),
                    );
                    txs.push(encode(tx));
                    generation += 1;
                }
                TxStatus::BadGeneration => {
                    if generation == 10 {
                        panic!("The first transaction will always have a valid generation");
                    } else {
                        let tx = create_tx_valid(
                            0,
                            max_priority_fee_bips,
                            admin,
                            config_value!("CHAIN_ID"),
                            encode_message(None),
                        );
                        txs.push(encode(tx));
                    }
                }
                TxStatus::BadChainId => {
                    let tx = create_tx_valid(
                        generation,
                        max_priority_fee_bips,
                        admin,
                        config_value!("CHAIN_ID") + 1,
                        encode_message(None),
                    );
                    txs.push(encode(tx));
                }

                TxStatus::BadSignature => {
                    let tx = create_tx_bad_sig(
                        generation,
                        max_priority_fee_bips,
                        admin,
                        config_value!("CHAIN_ID"),
                        encode_message(None),
                    );
                    txs.push(encode(tx));
                }
                TxStatus::OutOfGas => {
                    let tx = create_tx_out_of_gas(
                        generation,
                        max_priority_fee_bips,
                        admin,
                        config_value!("CHAIN_ID"),
                        encode_message(None),
                    );
                    txs.push(encode(tx));
                }
                TxStatus::Reverted => {
                    // A call message send by not admin will be reverted.
                    let tx = create_tx_valid(
                        0,
                        max_priority_fee_bips,
                        not_admin,
                        config_value!("CHAIN_ID"),
                        encode_message(None),
                    );
                    txs.push(encode(tx));
                }
                TxStatus::BadSerialization => {
                    let tx = FullyBakedTx::new(vec![1, 2, 3]);
                    txs.push(tx);
                }
                TxStatus::SignerDoesNotExist => {
                    let tx = create_tx_bad_sender(
                        0,
                        max_priority_fee_bips,
                        config_value!("CHAIN_ID"),
                        encode_message(None),
                    );
                    txs.push(encode(tx));
                }
            }
        }
        txs
    }

    pub(crate) fn create_blob(
        statuses: &[TxStatus],
        max_priority_fee_bips: PriorityFeeBips,
        admin: &TestUser<S>,
        not_admin: &TestUser<S>,
        seq_da_address: <<S as Spec>::Da as DaSpec>::Address,
    ) -> MockBlob {
        let txs: Vec<FullyBakedTx> = create_txs(statuses, max_priority_fee_bips, admin, not_admin);

        let blob = borsh::to_vec(&txs).unwrap();
        MockBlob::new_with_hash(blob, seq_da_address)
    }

    pub fn encode_message(gas: Option<<S as Spec>::Gas>) -> IntegTestRuntimeCall<S> {
        <IntegTestRuntime<S> as EncodeCall<ValueSetter<S>>>::to_decodable(CallMessage::SetValue {
            value: 8,
            gas,
        })
    }

    pub fn encode(tx: Transaction<IntegTestRuntime<S>, S>) -> FullyBakedTx {
        <IntegTestRuntime<S> as Runtime<S>>::Auth::encode_with_standard_auth(RawTx::new(
            borsh::to_vec(&tx).unwrap(),
        ))
    }
}
