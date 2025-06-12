# How to Create a Rollup from Scratch

Many rollups have concepts like `Account` or `Token` and access the state in a similar manner. This is where the [sov-modules-api](../../crates/module-system/sov-modules-api/README.md) becomes useful. It offers a standardized approach to writing rollup business logic. However, there are cases where your rollup requirements may be so unique that the `module-system` could become a hindrance. In this tutorial, we will bypass the `module-system` and directly create a simple rollup by implementing a `StateTransitionFunction` "from scratch".

In this tutorial, we’ll build an STF which checks if the input data (called a preimage) results in a specific output (called a digest) when fed through a hash function. It's important to note that our rollup is designed to be "stateless," meaning that implementing state access is not covered in this tutorial. However, if you're interested, you can refer to the [sov-state](../../crates/module-system/sov-state/README.md) for an example of how it can be done.

## Implementing the State Transition Function

The [State Transition Function
interface](../../crates/rollup-interface/specs/interfaces/stf.md) serves as the core component of our rollup, where the business logic will reside.
Implementations of this trait can be integrated with any zkVM and DA Layer resulting in a fully functional rollup. To begin, we will create a structure called `CheckHashPreimageStf`, and implement the `StateTransitionFunction` trait for it. You can find the complete code in the `lib.rs` file, but we will go over the most important parts here:

```rust, ignore
pub struct CheckHashPreimageStf {}
```

The `ApplyBlobResult` represents the outcome of the state transition, and its specific usage will be explained later:

```rust, ignore
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub enum ApplyBlobResult {
    Failure,
    Success,
}
```

Now let's discuss the implementation. First, we define some types that are relevant to our rollup:

```rust, ignore
impl StateTransitionFunction for CheckHashPreimageStf {
    // Since our rollup is stateless, we don't need to consider the StateRoot.
    type StateRoot = ();

    // This represents the initial configuration of the rollup, but it is not supported in this tutorial.
    type InitialState = ();

    // We could incorporate the concept of a transaction into the rollup, but we leave it as an
    // exercise for the reader.
    type TxReceiptContents = ();

    // This is the type that will be returned as a result of `apply_blob`.
    type BatchReceiptContents = ApplyBlobResult;

   // This data is produced during actual batch execution or validated with proof during verification.
   // However, in this tutorial, we won't use it.
    type Witness = ();

    // This represents a proof of misbehavior by the sequencer, but we won't utilize it in this tutorial.
    type MisbehaviorProof = ();
}
```

Now that we have defined the necessary types, we need to implement the following functions:

```rust, ignore
    // Perform one-time initialization for the genesis block.
    fn init_chain(&mut self, 
        genesis_rollup_header: &<S::Da as DaSpec>::BlockHeader,
        pre_state: Self::PreState,
        params: Self::GenesisParams) {
        // Do nothing
    }

    // Called at the beginning of each DA-layer block - whether or not that block contains any
    // data relevant to the rollup.
    fn begin_slot(
        &self,
        state: &mut StateCheckpoint<S>,
        slot_header: &<S::Da as DaSpec>::BlockHeader,
        pre_state_root: &<S::Storage as Storage>::Root,
        visible_hash: &<S::Storage as Storage>::Root,
    ) {
        // Do nothing
    }
```

These functions handle the initialization and preparation stages of our rollup, but as we are not modifying the rollup state, their implementation is simply left empty.

Next we need to write the core logic in `apply_slot`:

```rust, ignore
    // The core logic of our rollup.
    fn apply_slot<'a, I>(
        &self,
        _pre_state_root: &[u8; 0],
        _base_state: Self::PreState,
        _witness: Self::Witness,
        _slot_header: &Da::BlockHeader,
        relevant_blobs: RelevantBlobIters<I>,
    ) -> ApplySlotOutput<Vm, Da, Self>
    where
        I: IntoIterator<Item = &'a mut Da::BlobTransaction>,
    {
        let mut receipts = vec![];
        for blob in relevant_blobs.batch_blobs {
            let data = blob.verified_data();

            // Check if the sender submitted the preimage of the hash.
            let hash = sha2::Sha256::digest(data).into();
            let desired_hash = [
                102, 104, 122, 173, 248, 98, 189, 119, 108, 143, 193, 139, 142, 159, 142, 32, 8,
                151, 20, 133, 110, 226, 51, 179, 144, 42, 89, 29, 13, 95, 41, 37,
            ];

            let result = if hash == desired_hash {
                ApplySlotResult::Success
            } else {
                ApplySlotResult::Failure
            };

            // Return the `BatchReceipt`
            receipts.push(BatchReceipt {
                batch_hash: hash,
                tx_receipts: vec![],
                inner: result,
                gas_price: vec![],
            });
        }

        SlotResult {
            state_root: [],
            change_set: (),
            batch_receipts: receipts,
            witness: (),
        }
    }
```

The above function reads the data from the blob, computes the `hash`, compares it with the `desired_hash`, and returns a `BatchReceipt` indicating whether the preimage was successfully submitted or not.

The last method is `end_slot`, like before the implementation is trivial:

```rust, ignore
    fn end_slot(
        &self,
        storage: S::Storage,
        gas_used: &S::Gas,
        mut checkpoint: StateCheckpoint<S>,
    ) -> (
        Self::StateRoot,
        Self::Witness,
        Vec<ConsensusSetUpdate<OpaqueAddress>>,
    ) {
        ((), (), vec![])
    }
```

### Exercise

In the current implementation, every blob contains the data we pass to the hash function.
As an exercise, you can introduce the concept of transactions. In this scenario,
the blob would contain multiple transactions (containing data) that we can loop over to check hash equality.
The first transaction that finds the correct hash would break the loop and return early.

## Testing

The `sov-mock-da` and `sov-mock-zkvm` crates provide two utilities that are useful for testing:

1. The `sov_mock_zkvm::MockZkvm` is an implementation of the `Zkvm` trait that can be used in tests.
2. The `sov_mock_da::MockBlob` is an implementation of the `BlobTransactionTrait` trait that can be used in tests. It accepts an `A: BasicAddress` as a generic parameter. For testing purposes, we use `MockAddress` struct from the same module 

You can find more details in the `stf_test.rs` file.

The following test checks the rollup logic. In the test, we call `init_chain, begin_slot, and end_slot` for completeness, even though these methods do nothing.

```rust
use demo_simple_stf::{ApplySlotResult, CheckHashPreimageStf, Root};
use sov_mock_da::{MockAddress, MockBlob, MockBlock, MockBlockHeader, MockDaSpec};
use sov_mock_zkvm::MockZkvm;
use sov_rollup_interface::da::RelevantBlobIters;
use sov_rollup_interface::stf::{ExecutionContext, StateTransitionFunction};

fn test_stf_success() {
    let address = MockAddress::from([1; 32]);

    let stf = &mut CheckHashPreimageStf::default();
    StateTransitionFunction::<MockZkvm, MockZkvm, MockDaSpec>::init_chain(stf, &Default::default(), (), ());

    let mut batch_blobs = {
        let incorrect_preimage = vec![1; 32];
        let correct_preimage = vec![0; 32];

        [
            MockBlob::new(incorrect_preimage, address, [0; 32]),
            MockBlob::new(correct_preimage, address, [0; 32]),
        ]
    };

    // Pretend we are in native code and progress the blobs to the verified state.
    for blob in &mut batch_blobs {
        blob.advance();
    }

    let mut proof_blobs = {
        [
            MockBlob::new(vec![0; 32], address, [0; 32]),
            MockBlob::new(vec![0; 32], address, [0; 32]),
        ]
    };

    for blob in &mut proof_blobs {
        blob.advance();
    }

    let relevant_blobs = RelevantBlobIters {
        proof_blobs: proof_blobs.as_mut_slice(),
        batch_blobs: batch_blobs.as_mut_slice(),
    };

    let result = StateTransitionFunction::<MockZkvm, MockZkvm, MockDaSpec>::apply_slot(
        stf,
        &Root([]),
        (),
        (),
        &MockBlockHeader::default(),
        relevant_blobs,
        ExecutionContext::Node,
    );

    assert_eq!(2, result.batch_receipts.len());

    let receipt = &result.batch_receipts[0];
    assert_eq!(receipt.inner, ApplySlotResult::Failure);

    let receipt = &result.batch_receipts[1];
    assert_eq!(receipt.inner, ApplySlotResult::Success);
}
```
