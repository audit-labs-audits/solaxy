use std::marker::PhantomData;

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_sdk::transaction::Transaction;
use sov_address::FromVmAddress;
use sov_bank::Amount;
use sov_modules_api::{
    capabilities::{
        fatal_deserialization_error, AuthenticationError, AuthenticationOutput, AuthorizationData,
        BatchFromUnregisteredSequencer, FatalError, TransactionAuthenticator, UniquenessData,
        UnregisteredAuthenticationError,
    },
    macros::config_value,
    transaction::{
        AuthenticatedTransactionAndRawHash, AuthenticatedTransactionData, Credentials,
        PriorityFeeBips, TxDetails,
    },
    CredentialId, DispatchCall, FullyBakedTx, HexString, ProvableStateReader, RawTx, Runtime, Spec,
};
use sov_state::User;
use svm_types::SolanaAddress;

pub fn authenticate<Accessor: ProvableStateReader<User, Spec = S>, S: Spec>(
    raw_tx: &[u8],
    _state: &mut Accessor,
) -> Result<AuthenticationOutput<S, Vec<u8>>, AuthenticationError>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    let solana_tx =
        decode_svm_tx(raw_tx).map_err(|e| AuthenticationError::FatalError(e, [0; 32].into()))?;

    let tx_hash = solana_tx.verify_and_hash_message().map_err(|e| {
        AuthenticationError::FatalError(
            FatalError::SigVerificationFailed(format!("Failed to verify transaction: {e}")),
            [0; 32].into(),
        )
    })?;

    let message = solana_tx.message();
    let signers = message.signer_keys();

    let authenticated_tx = AuthenticatedTransactionData::<S>(TxDetails {
        chain_id: config_value!("CHAIN_ID"),
        max_priority_fee_bips: PriorityFeeBips::ZERO,
        // TODO: maybe this is too high - do we want to infer this from the transaction? double the
        // lamports per signature or similar?
        max_fee: Amount(10_000_000),
        gas_limit: None,
    });

    let tx_and_raw_hash = AuthenticatedTransactionAndRawHash {
        authenticated_tx,
        raw_tx_hash: tx_hash.to_bytes().into(),
    };

    let signer: SolanaAddress = signers
        .first()
        .copied()
        .ok_or_else(|| {
            AuthenticationError::FatalError(
                FatalError::SigVerificationFailed("Transaction has no signers".to_string()),
                tx_hash.to_bytes().into(),
            )
        })?
        .into();

    let hash = sov_rollup_interface::TxHash::new(tx_hash.to_bytes());
    let auth_data = AuthorizationData {
        uniqueness: UniquenessData::Generation(0),
        default_address: S::Address::from_vm_address(signer),
        credentials: Credentials::new(signer),
        credential_id: CredentialId(HexString(signer.into())),
        tx_hash: hash,
    };

    let call = raw_tx.to_vec();

    Ok((tx_and_raw_hash, auth_data, call))
}

/// Decode a byte sequence into a Solana transaction without verifying the transaction.
pub fn decode_svm_tx(raw_tx: &[u8]) -> Result<Transaction, FatalError> {
    borsh::from_slice(raw_tx).map_err(|e| {
        FatalError::MessageDecodingFailed(format!("Failed to decode transaction: {e}"))
    })
}

/// Indicates that a runtime supports the `Solana` transaction authenticator
/// and provides suitable methods for encoding and decoding Solana transactions.
pub trait SolanaAuthenticator<S: Spec>: Runtime<S> {
    /// Add the Solana discriminant to a transaction the runtime.
    fn add_solana_auth(tx: RawTx) -> <Self::Auth as TransactionAuthenticator<S>>::Input;

    /// Encode a transaction with the Solana discriminant for the runtime.
    fn encode_with_solana_auth(data: Vec<u8>) -> FullyBakedTx {
        <Self::Auth as TransactionAuthenticator<S>>::encode_authenticator_input(
            &Self::add_solana_auth(RawTx::new(data)),
        )
    }
}

/// SVM-compatible transaction authenticator. See [`TransactionAuthenticator`].
pub struct SvmAuthenticator<S, Rt>(PhantomData<(S, Rt)>);

impl<S, Rt> TransactionAuthenticator<S> for SvmAuthenticator<S, Rt>
where
    S: Spec,
    Rt: Runtime<S> + DispatchCall<Spec = S>,
    S::Address: FromVmAddress<SolanaAddress>,
{
    type Decodable = Auth<Vec<u8>, <Rt as DispatchCall>::Decodable>;
    type Input = Auth;

    #[cfg(feature = "native")]
    fn decode_serialized_tx(tx: &FullyBakedTx) -> Result<Self::Decodable, FatalError> {
        let auth_variant = borsh::from_slice::<Auth>(&tx.data)
            .map_err(|e| FatalError::DeserializationFailed(e.to_string()))?;

        match auth_variant {
            Auth::Mod(raw_tx) => {
                let call = sov_modules_api::capabilities::decode_sov_tx::<S, Rt>(&raw_tx.data)?;
                Ok(Auth::Mod(call))
            }
            Auth::Svm(tx) => {
                let _solana_tx = decode_svm_tx(&tx.data)?;
                Ok(Auth::Svm(tx.data))
            }
        }
    }

    fn authenticate<Accessor: ProvableStateReader<User, Spec = S>>(
        tx: &FullyBakedTx,
        accessor: &mut Accessor,
    ) -> Result<AuthenticationOutput<S, Self::Decodable>, AuthenticationError> {
        let input = borsh::from_slice::<Auth>(&tx.data)
            .map_err(|e| fatal_deserialization_error::<_, S, _>(&tx.data, e, accessor))?;

        match input {
            Auth::Mod(tx) => {
                let (tx_and_raw_hash, auth_data, runtime_call) =
                    sov_modules_api::capabilities::authenticate::<_, S, Rt>(
                        &tx.data,
                        &Rt::CHAIN_HASH,
                        accessor,
                    )
                    .unwrap();

                Ok((tx_and_raw_hash, auth_data, Auth::Mod(runtime_call)))
            }
            Auth::Svm(tx) => {
                let (tx_and_raw_hash, auth_data, runtime_call) =
                    authenticate::<_, S>(&tx.data, accessor)?;

                Ok((tx_and_raw_hash, auth_data, Auth::Svm(runtime_call)))
            }
        }
    }

    #[cfg(feature = "native")]
    fn compute_tx_hash(tx: &FullyBakedTx) -> anyhow::Result<sov_modules_api::TxHash> {
        use sov_modules_api::TxHash;

        let input: Auth = borsh::from_slice(&tx.data)?;

        match input {
            Auth::Svm(tx) => {
                let tx = decode_svm_tx(&tx.data)?;
                let hash = tx.verify_and_hash_message()?;
                Ok(TxHash::new(hash.to_bytes()))
            }
            Auth::Mod(tx) => Ok(sov_modules_api::capabilities::calculate_hash(
                &tx.data,
                &mut sov_modules_api::gas::UnlimitedGasMeter::<S>::default(),
            )?),
        }
    }

    fn authenticate_unregistered<Accessor: ProvableStateReader<User, Spec = S>>(
        batch: &BatchFromUnregisteredSequencer,
        pre_exec_ws: &mut Accessor,
    ) -> Result<AuthenticationOutput<S, Self::Decodable>, UnregisteredAuthenticationError> {
        let (tx_and_raw_hash, auth_data, runtime_call) =
            sov_modules_api::capabilities::RollupAuthenticator::<S, Rt>::authenticate_unregistered(
                batch,
                pre_exec_ws,
            )?;

        if Rt::allow_unregistered_tx(&runtime_call) {
            Ok((tx_and_raw_hash, auth_data, Auth::Mod(runtime_call)))
        } else {
            Err(UnregisteredAuthenticationError::FatalError(
                FatalError::Other(
                    "The runtime call included in the transaction was invalid.".to_string(),
                ),
                tx_and_raw_hash.raw_tx_hash,
            ))?
        }
    }

    fn add_standard_auth(tx: RawTx) -> Self::Input {
        Auth::Mod(tx)
    }
}

/// Describes which authenticator to use to deserialize and check the signature on the transaction.
#[derive(Debug, PartialEq, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub enum Auth<Svm = RawTx, Mod = RawTx> {
    /// Authenticate using the `SVM` authenticator, which expects a standard SVM transaction.
    Svm(Svm),
    /// Authenticate using the standard `sov-module` authenticator, which uses the default
    /// signature scheme and hashing algorithm defined in the rollup's [`Spec`].
    Mod(Mod),
}
