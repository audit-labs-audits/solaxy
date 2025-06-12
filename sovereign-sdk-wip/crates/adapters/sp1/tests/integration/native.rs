use serde::{Deserialize, Serialize};
use sov_rollup_interface::zk::{Proof, ZkvmGuest, ZkvmHost};
use sov_sp1_adapter::host::SP1Host;
use sp1_sdk::{SP1Proof, SP1PublicValues};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct TestStruct {
    ints: Vec<i32>,
    string: String,
}

#[test]
fn test_hints_roundtrip() {
    let mut host = SP1Host::new(&[]);

    let hint_a = TestStruct {
        ints: vec![1, 2, 3, 4, 5],
        string: "hello".to_string(),
    };
    let hint_b = TestStruct {
        ints: vec![1, 2, 3, 4, 5],
        string: "hello".to_string(),
    };

    host.add_hint(&hint_a);
    host.add_hint(&hint_b);

    let guest = host.simulate_with_hints();

    let mut received;
    received = guest.read_from_host();
    assert_eq!(hint_a, received);
    received = guest.read_from_host();
    assert_eq!(hint_b, received);
}

#[test]
fn test_fibonnaci_host() {
    let fibonacci_elf = include_bytes!("../../test_data/riscv32im-succinct-zkvm-elf");
    let mut host = SP1Host::new(fibonacci_elf);
    // Give the input 7 to the fibonnaci program
    host.add_hint(7u32);
    let proof = host.run(false);
    assert!(proof.is_ok());
    if let Ok(output) = proof {
        // The fibonnaci program we have hardcoded here outputs the n-1th and nth fibonnaci numbers
        let data: Proof<SP1Proof, SP1PublicValues> = bincode::deserialize(&output).unwrap();
        match data {
            Proof::PublicData(mut pv) => {
                let a = pv.read::<u32>();
                let b = pv.read::<u32>();
                assert_eq!(a, 7);
                assert_eq!(b, 13);
            }
            _ => panic!("Expected public data"),
        }
    } else {
        unreachable!()
    }
}
