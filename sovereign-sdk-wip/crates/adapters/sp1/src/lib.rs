#![deny(missing_docs)]
//! # SP1 Adapter
//!
//! This crate contains an adapter allowing the SP1 to be used as a proof system for
//! Sovereign SDK rollups.
use std::fmt;
use std::fmt::Debug;

use anyhow::Error;
use crypto::{SP1PublicKey, SP1Signature};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sov_rollup_interface::zk::{CodeCommitment, CryptoSpec, ZkVerifier};
#[cfg(not(target_os = "zkvm"))]
use sp1_sdk::{ProverClient, SP1ProofWithPublicValues};

#[cfg(feature = "native")]
use crate::crypto::private_key::SP1PrivateKey;

pub mod crypto;
pub mod guest;
#[cfg(feature = "native")]
pub mod host;

#[cfg(all(feature = "native", feature = "bench"))]
pub mod metrics;

/// Uniquely identifies a SP1 binary. Stored as a serialized version of `SP1VerifyingKey`.
/// TODO: When there's a nice representation of SP1VerifyingKey that can be compiled in SP1, we can use that.
/// e.g. If SP1VerifyingKey is moved to a crate that can be compiled in an SP1 program.
///
///
/// Use the [`ZkvmHost::code_commitment`](sov_rollup_interface::zk::ZkvmHost) method to get the MethodId for a given binary.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SP1MethodId(Vec<u8>);

impl Debug for SP1MethodId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("SP1MethodId").field(&self.0).finish()
    }
}

impl CodeCommitment for SP1MethodId {
    type DecodeError = Error;

    fn encode(&self) -> Vec<u8> {
        self.0.clone()
    }

    fn decode(verifying_key_bytes: &[u8]) -> Result<Self, Self::DecodeError> {
        Ok(Self(verifying_key_bytes.to_vec()))
    }
}

/// The cryptographic primitives provided by SP1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Copy)]
pub struct SP1CryptoSpec;

impl CryptoSpec for SP1CryptoSpec {
    #[cfg(feature = "native")]
    type PrivateKey = SP1PrivateKey;
    type PublicKey = SP1PublicKey;
    type Hasher = sha2::Sha256;
    type Signature = SP1Signature;
}

/// A verifier for SP1 proofs.
#[derive(Default, Clone)]
pub struct SP1Verifier;

#[cfg(not(target_os = "zkvm"))]
impl ZkVerifier for SP1Verifier {
    type CodeCommitment = SP1MethodId;
    type CryptoSpec = SP1CryptoSpec;
    type Error = anyhow::Error;

    fn verify<T: DeserializeOwned>(
        serialized_proof: &[u8],
        code_commitment: &Self::CodeCommitment,
    ) -> Result<T, Self::Error> {
        let proof: SP1ProofWithPublicValues = bincode::deserialize(serialized_proof)?;

        let prover = ProverClient::new();
        let verifying_key = bincode::deserialize(&code_commitment.0)?;
        prover.verify(&proof, &verifying_key)?;

        Ok(bincode::deserialize(proof.public_values.as_slice())?)
    }
}

/// The SP1 Zkvm.
#[derive(Debug, Clone, Default, PartialEq, Eq, schemars::JsonSchema)]
pub struct SP1;

impl sov_rollup_interface::zk::Zkvm for SP1 {
    type Guest = crate::guest::SP1Guest;
    type Verifier = SP1Verifier;

    #[cfg(feature = "native")]
    type Host = crate::host::SP1Host<'static>;
}

#[cfg(target_os = "zkvm")]
impl ZkVerifier for SP1Verifier {
    type CodeCommitment = SP1MethodId;

    type CryptoSpec = SP1CryptoSpec;

    type Error = anyhow::Error;

    fn verify<T: DeserializeOwned>(
        _serialized_proof: &[u8],
        _code_commitment: &Self::CodeCommitment,
    ) -> Result<T, Self::Error> {
        // Implement this, SP1 already supports recursion.
        // Use sp1_zkvm::lib::verify::verify_sp1_proof(vkey, &public_values_digest.into());
        // Example can be found here: https://github.com/succinctlabs/sp1/blob/14eb569d41d24721ffbd407d6060e202482d659c/examples/aggregation/program/src/main.rs#L47-L60
        //
        // Note: Currently SP1 does not support this interface for recursion. It expects proofs to be written into SP1Stdin with stdin.write_proof,
        // which are then read within the verify_sp1_proof method. The `verify_sp1_proof` method takes in the vkey hash, and the public values digest as direct input.
        // In the future, SP1 will support an interface for passing all 3 in directly.
        todo!("Implement this.")
    }
}

#[cfg(test)]
mod tests {
    // See <https://github.com/Sovereign-Labs/sovereign-sdk-wip/issues/1209>.
    #[cfg(not(coverage))]
    #[test]
    fn test_sp1_method_id_codec_roundtrip() {
        use sov_rollup_interface::zk::CodeCommitment;
        use sp1_sdk::ProverClient;

        use crate::SP1MethodId;

        const ELF: &[u8] = include_bytes!("../test_data/riscv32im-succinct-zkvm-elf");

        let prover = ProverClient::new();
        let (_, vk) = prover.setup(ELF);
        let method_id = SP1MethodId(bincode::serialize(&vk).unwrap());
        let encoded = method_id.encode();
        let decoded = SP1MethodId::decode(&encoded).unwrap();

        assert_eq!(method_id, decoded);
    }
}
