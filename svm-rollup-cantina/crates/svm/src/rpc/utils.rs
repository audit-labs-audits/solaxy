use std::any::type_name;

use base64::{prelude::BASE64_STANDARD, Engine};
use bincode::Options;
use jsonrpsee::core::RpcResult;
use solana_rpc_client_api::response::{Response, RpcResponseContext};
use solana_sdk::{epoch_info::EpochInfo, packet::PACKET_DATA_SIZE};
use solana_transaction_status::TransactionBinaryEncoding;
use sov_address::FromVmAddress;
use sov_modules_api::{prelude::UnwrapInfallible, ApiStateAccessor, Spec};
use svm_types::SolanaAddress;

use crate::{rpc::create_client_error, SVM};

impl<S: Spec> SVM<S>
where
    S::Address: FromVmAddress<SolanaAddress>,
{
    /// Helper function to create a [`Response`] object.
    pub(crate) fn to_response<T>(
        &self,
        value: T,
        state: &mut ApiStateAccessor<S>,
    ) -> RpcResult<Response<T>> {
        let slot = self
            .epoch_info
            .get(state)
            .unwrap_infallible()
            .unwrap_or(EpochInfo {
                epoch: 0,
                slot_index: 0,
                slots_in_epoch: 0,
                absolute_slot: 0,
                block_height: 0,
                transaction_count: None,
            })
            .slot_index;
        Ok(Response {
            context: RpcResponseContext {
                slot,
                api_version: None,
            },
            value,
        })
    }
}

const MAX_BASE58_SIZE: usize = 1683; // Golden, bump if PACKET_DATA_SIZE changes
const MAX_BASE64_SIZE: usize = 1644; // Golden, bump if PACKET_DATA_SIZE changes
/// Helper function to decode and deserialize a base64-encoded string.
/// Simplified version of https://github.com/anza-xyz/agave/blob/fa620250ab962feb9bd8b1977affe14b94302291/svm/examples/json-rpc/server/src/rpc_process.rs#L807
pub fn decode_and_deserialize<T>(
    encoded: String,
    encoding: TransactionBinaryEncoding,
) -> RpcResult<(Vec<u8>, T)>
where
    T: serde::de::DeserializeOwned,
{
    let decoded = match encoding {
        TransactionBinaryEncoding::Base58 => {
            if encoded.len() > MAX_BASE58_SIZE {
                return Err(create_client_error(&format!(
                    "base58 encoded {} too large: {} bytes (max: encoded/raw {}/{})",
                    type_name::<T>(),
                    encoded.len(),
                    MAX_BASE58_SIZE,
                    PACKET_DATA_SIZE,
                )));
            }
            solana_sdk::bs58::decode(encoded)
                .into_vec()
                .map_err(|e| create_client_error(&format!("invalid base58 encoding: {e:?}")))?
        }
        TransactionBinaryEncoding::Base64 => {
            if encoded.len() > MAX_BASE64_SIZE {
                return Err(create_client_error(&format!(
                    "base64 encoded {} too large: {} bytes (max: encoded/raw {}/{})",
                    type_name::<T>(),
                    encoded.len(),
                    MAX_BASE64_SIZE,
                    PACKET_DATA_SIZE,
                )));
            }
            BASE64_STANDARD
                .decode(encoded)
                .map_err(|e| create_client_error(&format!("invalid base64 encoding: {e:?}")))?
        }
    };
    if decoded.len() > PACKET_DATA_SIZE {
        return Err(create_client_error(&format!(
            "decoded {} too large: {} bytes (max: {} bytes)",
            type_name::<T>(),
            decoded.len(),
            PACKET_DATA_SIZE
        )));
    }
    bincode::options()
        .with_limit(PACKET_DATA_SIZE as u64)
        .with_fixint_encoding()
        .allow_trailing_bytes()
        .deserialize_from(&decoded[..])
        .map_err(|err| {
            create_client_error(&format!(
                "failed to deserialize {}: {}",
                type_name::<T>(),
                &err.to_string()
            ))
        })
        .map(|output| (decoded, output))
}
