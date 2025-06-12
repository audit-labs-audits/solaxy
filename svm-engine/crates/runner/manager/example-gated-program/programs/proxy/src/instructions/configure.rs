use anchor_lang::prelude::*;

use crate::{LockConfig, CONFIG_SEED, SEED};

#[derive(Accounts)]
pub struct Configure<'info> {
    #[account(
        mut,
        seeds = [SEED, CONFIG_SEED, target.key().as_ref()],
        bump
    )]
    pub lock_config: Account<'info, LockConfig>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(executable)]
    pub target: UncheckedAccount<'info>,
}

pub fn configure_handler(ctx: Context<Configure>, lock_duration: u16) -> Result<()> {
    if !ctx
        .accounts
        .lock_config
        .is_authorized(ctx.accounts.payer.key())
    {
        panic!("Unauthorized caller");
    }
    ctx.accounts.lock_config.lock_duration = lock_duration;
    Ok(())
}
