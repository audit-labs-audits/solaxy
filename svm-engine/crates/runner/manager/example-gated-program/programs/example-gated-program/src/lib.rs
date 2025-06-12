#![allow(unexpected_cfgs)]

pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;
pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("Asq13rW8nzJKyVU8pYHprK54i5gAgKKLxWsi3asd79Y2");

#[program]
pub mod example_gated_program {
    use super::*;

    pub fn hello(ctx: Context<Hello>, message: String) -> Result<()> {
        hello_handler(ctx, message)
    }
}

pub fn find_message_address(program_id: Pubkey, payer: Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[SEED, payer.as_ref()], &program_id).0
}
