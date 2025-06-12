use serde::{Deserialize, Deserializer, Serializer, de, ser::SerializeSeq};
use solana_sdk::signature::Keypair;

pub fn deserialize<'de, D>(deserializer: D) -> Result<Keypair, D::Error>
where
    D: Deserializer<'de>,
{
    Vec::<u8>::deserialize(deserializer)
        .and_then(|bytes| Keypair::from_bytes(&bytes).map_err(de::Error::custom))
}

pub fn serialize<S: Serializer>(keypair: &Keypair, serializer: S) -> Result<S::Ok, S::Error> {
    let bytes = keypair.to_bytes();
    let mut seq = serializer.serialize_seq(Some(bytes.len()))?;
    for byte in bytes {
        seq.serialize_element(&byte)?;
    }
    seq.end()
}
