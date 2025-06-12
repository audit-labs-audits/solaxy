use anchor_lang::{prelude::*, Discriminator};

use crate::{Lock, LockConfig, CONFIG_SEED, SEED};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = payer,
        space = Lock::DISCRIMINATOR.len() + Lock::INIT_SPACE,
        seeds = [SEED, target.key().as_ref()],
        bump
    )]
    pub lock: Account<'info, Lock>,

    #[account(
        init,
        payer = payer,
        space = LockConfig::DISCRIMINATOR.len() + LockConfig::INIT_SPACE,
        seeds = [SEED, CONFIG_SEED, target.key().as_ref()],
        bump
    )]
    pub lock_config: Account<'info, LockConfig>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,

    #[account(executable)]
    pub target: UncheckedAccount<'info>,
}

pub fn initialize_handler(ctx: Context<Initialize>, lock_duration: u16) -> Result<()> {
    let lock_until = Clock::get()?.unix_timestamp + lock_duration as i64;
    ctx.accounts
        .lock_config
        .set_inner(LockConfig::new(lock_duration, ctx.accounts.payer.key()));
    ctx.accounts.lock.set_inner(Lock::new(lock_until));
    Ok(())
}
