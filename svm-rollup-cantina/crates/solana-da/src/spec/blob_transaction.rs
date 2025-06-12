use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sov_rollup_interface::da::{BlobReaderTrait, CountedBufReader};
use svm_types::SolanaAddress;

use crate::SolanaBlockHash;

/// A transaction on the Solana blockchain. This is a wrapper around the native
/// [`solana_sdk::message::Message`] type, the blob data that was uploaded, and its sender.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SolanaBlobTransaction {
    sender: SolanaAddress,
    hash: SolanaBlockHash,
    blob_address: SolanaAddress,
    blob: CountedBufReader<Bytes>,
}

impl SolanaBlobTransaction {
    /// Create a new `SolanaBlobTransaction` from a [`solana_sdk::message::VersionedMessage`] and its
    /// associated blob data. The  should be the message where the [`chunker`] account was closed
    /// (and thus hashed by the [`hasher`] program).
    #[must_use]
    pub fn new(
        message: solana_sdk::message::VersionedMessage,
        blob_address: SolanaAddress,
        blob: Vec<u8>,
    ) -> Self {
        // "in the `Message` structure, the first account is always the fee-payer"
        let sender = message
            .static_account_keys()
            .first()
            .expect("Every transaction must have a payer")
            .into();
        let hash = SolanaBlockHash(message.hash());
        Self {
            sender,
            hash,
            blob_address,
            blob: CountedBufReader::new(blob.into()),
        }
    }

    /// Returns the address of the [`blober`] account that was used to upload this blob.
    pub fn blob_address(&self) -> SolanaAddress {
        self.blob_address
    }
}

/// This trait wraps "blob transaction" from a data availability layer allowing partial consumption of the
/// blob data by the rollup.
///
/// The motivation for this trait is limit the amount of validation work that a rollup has to perform when
/// verifying a state transition. In general, it will often be the case that a rollup only cares about some
/// portion of the data from a blob. For example, if a blob contains a malformed transaction then the rollup
/// will slash the sequencer and exit early - so it only cares about the content of the blob up to that point.
///
/// This trait allows the DaVerifier to track which data was read by the rollup, and only verify the relevant data.
impl BlobReaderTrait for SolanaBlobTransaction {
    /// The type used to represent addresses on the DA layer.
    type Address = SolanaAddress;

    /// The hash type of all DA blobs (e.g. transactions and proofs).
    type BlobHash = SolanaBlockHash;

    /// Returns the address (on the DA layer) of the entity which submitted the blob transaction
    fn sender(&self) -> SolanaAddress {
        self.sender
    }

    /// Returns the hash of the blob as it appears on the DA layer
    fn hash(&self) -> SolanaBlockHash {
        self.hash
    }

    /// Returns a slice containing all the data accessible to the rollup at this point in time.
    /// When running in native mode, the rollup can extend this slice by calling `advance`. In zk-mode,
    /// the rollup is limited to only the verified data.
    ///
    /// Rollups should use this method in conjunction with `advance` to read only the minimum amount
    /// of data required for execution
    fn verified_data(&self) -> &[u8] {
        self.blob.accumulator()
    }

    /// Returns the total number of bytes in the blob. Note that this may be unequal to `verified_data.len()`.
    fn total_len(&self) -> usize {
        self.blob.total_len()
    }

    /// Extends the `partial_data` accumulator with the next `num_bytes` of  data from the blob
    /// and returns a reference to the entire contents of the blob up to this point.
    ///
    /// If `num_bytes` is greater than the length of the remaining unverified data,
    /// then all remaining unverified data is added to the accumulator.
    ///
    /// ### Note:
    /// This method is only available when the `native` feature is enabled because it is unsafe to access
    /// unverified data during execution
    #[cfg(feature = "native")]
    fn advance(&mut self, num_bytes: usize) -> &[u8] {
        self.blob.advance(num_bytes);
        self.verified_data()
    }
}
