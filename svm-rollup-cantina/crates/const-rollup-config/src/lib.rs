/// The namespace used by the rollup to store its data. This is a raw slice of 10 bytes.
/// The rollup stores its data in the namespace b"sov-test" on Celestia. Which in this case is encoded using the
/// ascii representation of each character.
pub const ROLLUP_BATCH_NAMESPACE_RAW: [u8; 10] = [0, 0, 115, 111, 118, 45, 116, 101, 115, 116];

/// The namespace used by the rollup to store aggregated ZK proofs.
pub const ROLLUP_PROOF_NAMESPACE_RAW: [u8; 10] = [115, 111, 118, 45, 116, 101, 115, 116, 45, 112];

/// The Solana PDA of the blober account used by the rollup to store batches.
/// We hardcode these values here because we cannot use the blober crate inside SP1.
pub const SOLANA_BLOBER_BATCH_ADDRESS: &str = "GjgHgjvRV9H6wmAv5B8Q7WQhFTzpXEBjwFPQw7a44xae";

/// The Solana PDA of the blober account used by the rollup to store proofs.
pub const SOLANA_BLOBER_PROOFS_ADDRESS: &str = "5hrYCqZvkZYvx2ZcWYaCspjfPEW2SPHkeqeXnsLKe28i";
