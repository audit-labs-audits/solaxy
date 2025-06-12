use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Message {
    #[max_len(13)]
    // 13 is the exact length of the "Hello, World!" string we want to place here. Borsch deserialize
    // does not correctly reason about strings with null padding set up here; it will return an
    // error that the stream has unhandled data if it contains a variable-length String followed
    // by the padding added by the max_len() directive.
    pub message: String,
}
