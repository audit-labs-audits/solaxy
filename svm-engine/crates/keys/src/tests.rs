use std::str::FromStr;

use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};

use crate::{
    keypair,
    pubkey::{single, vec},
};

const KEYS: [&str; 4] = [
    "9rcNiKnZ2qQ1HgTKL4fcrZb2UrPKKDbVGwhNbHTYyi5S",
    "EYfLKQTAitf38MBeBwEUUwHhydjFgcbqJULZPSsBtZer",
    "6PoUdZtG8rA642EjwiXPxmQtMnbi3d3r3ZpaZLUh3Kzh",
    "4sTMv74AKbwbqY8fxCVY4CQguFAcquYRJGVK6WetxeHA",
];

const KEY: &str = KEYS[0];

const KEYPAIR: [u8; 64] = [
    76, 28, 178, 144, 236, 74, 65, 139, 131, 149, 140, 235, 121, 169, 207, 96, 36, 141, 61, 172,
    237, 223, 72, 32, 86, 187, 44, 38, 160, 235, 100, 146, 180, 74, 218, 123, 190, 227, 44, 21, 42,
    69, 150, 58, 118, 184, 32, 154, 229, 243, 237, 197, 16, 174, 226, 20, 169, 235, 169, 61, 105,
    234, 253, 38,
];

#[derive(Debug, Serialize, Deserialize)]
struct SingleTest(#[serde(with = "single")] Pubkey);

#[test]
fn test_bs58_string_deserialization() {
    let deserialized: SingleTest = serde_json::from_str(&format!("\"{KEY}\"")).unwrap();
    assert_eq!(deserialized.0, Pubkey::from_str(KEY).unwrap());
}

#[test]
fn test_bs58_string_serialization() {
    let serialized = serde_json::to_string(&SingleTest(Pubkey::from_str(KEY).unwrap())).unwrap();
    assert_eq!(serialized, format!("\"{KEY}\""));
}

#[derive(Debug, Serialize, Deserialize)]
struct VecTest(#[serde(with = "vec")] Vec<Pubkey>);

#[test]
fn test_bs58_string_vector_deserialization() {
    let deserialized: VecTest = serde_json::from_str(&format!("{KEYS:?}")).unwrap();
    assert_eq!(deserialized.0.len(), 4);

    for (key, string) in deserialized.0.iter().zip(KEYS.iter()) {
        assert_eq!(key.to_string(), *string);
    }
}

#[test]
fn test_bs58_string_vector_serialization() {
    let serialized = serde_json::to_string(&VecTest(
        KEYS.iter()
            .map(|key| Pubkey::from_str(key).unwrap())
            .collect(),
    ))
    .unwrap();
    assert_eq!(serialized, format!("{KEYS:?}").replace(" ", ""));
}

#[derive(Debug, Serialize, Deserialize)]
struct KeypairTest(#[serde(with = "keypair")] Keypair);

#[test]
fn test_keypair_deserialization() {
    let deserialized: KeypairTest = serde_json::from_str(&format!("{KEYPAIR:?}")).unwrap();
    assert_eq!(deserialized.0.to_bytes(), KEYPAIR);
}

#[test]
fn test_keypair_serialization() {
    let serialized =
        serde_json::to_string(&KeypairTest(Keypair::from_bytes(&KEYPAIR).unwrap())).unwrap();
    assert_eq!(serialized, format!("{KEYPAIR:?}").replace(" ", ""));
}
