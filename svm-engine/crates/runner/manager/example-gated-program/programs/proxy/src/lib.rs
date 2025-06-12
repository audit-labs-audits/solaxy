#![allow(unexpected_cfgs)]

pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;
pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("3ATxf2BgWJ1SvMvtz3sZJWfVMnFzZvtXyLZ5Shh6UnnA");

#[program]
pub mod proxy {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, lock_duration: u16) -> Result<()> {
        initialize_handler(ctx, lock_duration)
    }

    pub fn configure(ctx: Context<Configure>, lock_duration: u16) -> Result<()> {
        configure_handler(ctx, lock_duration)
    }

    pub fn proxy(
        ctx: Context<Proxy>,
        instruction_data: Vec<u8>,
        account_metas: Vec<u8>,
    ) -> Result<()> {
        proxy_handler(ctx, instruction_data, account_metas)
    }
}

pub fn find_lock_address(program_id: Pubkey, target: Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[SEED, target.as_ref()], &program_id).0
}

pub fn find_lock_config_address(program_id: Pubkey, target: Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[SEED, CONFIG_SEED, target.as_ref()], &program_id).0
}

pub fn find_lock_signer_address(program_id: Pubkey, target: Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[SEED, SIGNER_SEED, target.as_ref()], &program_id).0
}
