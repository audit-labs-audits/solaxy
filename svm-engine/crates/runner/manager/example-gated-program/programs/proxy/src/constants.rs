use anchor_lang::prelude::*;

#[constant]
pub const SEED: &[u8] = b"proxy-lock";

#[constant]
pub const CONFIG_SEED: &[u8] = b"proxy-lock-config";

#[constant]
pub const SIGNER_SEED: &[u8] = b"proxy-lock-signer";
