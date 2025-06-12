use std::sync::Arc;

use nitro_da_indexer_api::CompoundProof;
use serde::{Deserialize, Serialize};
use solana_sdk::{bs58, clock::Slot};
use solana_transaction_status::EncodedConfirmedBlock;
use sov_rollup_interface::{
    da::{BlockHeaderTrait, DaProof, DaSpec, RelevantBlobs, RelevantProofs},
    node::da::SlotData,
};

use crate::{
    SolanaBlobTransaction, SolanaBlockHeader, SolanaCompletenessProof, SolanaDASpec,
    SolanaInclusionMultiProof,
};

/// A Solana block and any associated blobs and their proofs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolanaBlock {
    block: Arc<EncodedConfirmedBlock>,
    header: SolanaBlockHeader,
    /// Blobs and proofs from `batch` namespace.
    batch: SolanaData,
    /// Blobs and proofs from `proof` namespace.
    proof: SolanaData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolanaData {
    blobs: Vec<SolanaBlobTransaction>,
    proof: CompoundProof,
}

impl SolanaData {
    /// Create a new SolanaData
    pub fn new(blobs: Vec<SolanaBlobTransaction>, proof: CompoundProof) -> Self {
        Self { blobs, proof }
    }
}

impl From<&SolanaData> for DaProof<SolanaInclusionMultiProof, SolanaCompletenessProof> {
    fn from(data: &SolanaData) -> Self {
        Self {
            inclusion_proof: (&data.proof).into(),
            completeness_proof: (&data.proof).into(),
        }
    }
}

impl SolanaBlock {
    /// Create a new Solana block from a native [`EncodedConfirmedBlock`], the latest set of vote accounts,
    /// and the block's associated proofs and blobs. This works a bit differently in that the blobs and proofs
    /// can not be extracted from the block itself, they must be supplied by an indexer, but can be trusted due
    /// to the proofs verifying their contents.
    pub fn new(
        slot: Slot,
        block: EncodedConfirmedBlock,
        batch: SolanaData,
        proof: SolanaData,
    ) -> Self {
        let header = SolanaBlockHeader::new(slot, &block);
        Self {
            block: Arc::new(block),
            header,
            batch,
            proof,
        }
    }
}

/// "Extracts" the relevant blobs from a block. This information is already contained in the struct,
/// so it's just a cloned getter.
impl From<&SolanaBlock> for RelevantBlobs<SolanaBlobTransaction> {
    fn from(block: &SolanaBlock) -> Self {
        Self {
            proof_blobs: block.proof.blobs.clone(),
            batch_blobs: block.batch.blobs.clone(),
        }
    }
}

/// "Generates" a proof that the relevant blob transactions have been extracted correctly from the DA layer
/// block. This information is already contained in the struct, so it's just a cloned getter.
impl From<&SolanaBlock> for RelevantProofs<SolanaInclusionMultiProof, SolanaCompletenessProof> {
    fn from(block: &SolanaBlock) -> Self {
        Self {
            batch: (&block.batch).into(),
            proof: (&block.proof).into(),
        }
    }
}

/// `SlotData` is the subset of a DA layer block which is stored in the rollup's database.
/// At the very least, the rollup needs access to the hashes and headers of all DA layer blocks,
/// but rollup may choose to store partial (or full) block data as well.
impl SlotData for SolanaBlock {
    /// The header type for a DA layer block as viewed by the rollup. This need not be identical
    /// to the underlying rollup's header type, but it must be sufficient to reconstruct the block hash.
    ///
    /// For example, most fields of a Tendermint-based DA chain like Celestia are irrelevant to the rollup.
    /// For these fields, we only ever store their *serialized* representation in memory or on disk. Only a few special
    /// fields like `data_root` are stored in decoded form in the `CelestiaHeader` struct.
    type BlockHeader = <SolanaDASpec as DaSpec>::BlockHeader;

    /// The canonical hash of the DA layer block.
    fn hash(&self) -> [u8; 32] {
        bs58::decode(&self.block.blockhash)
            .into_vec()
            .expect("blockhash to be a valid base58 string")
            .try_into()
            .expect("blockhash to be 32 bytes")
    }

    /// The header of the DA layer block.
    fn header(&self) -> &Self::BlockHeader {
        &self.header
    }

    fn timestamp(&self) -> sov_rollup_interface::da::Time {
        self.header.time()
    }
}
