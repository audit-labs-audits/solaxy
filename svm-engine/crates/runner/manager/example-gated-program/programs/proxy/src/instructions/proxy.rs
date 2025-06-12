use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke_signed},
};

use crate::{Lock, LockConfig, CONFIG_SEED, SEED, SIGNER_SEED};

#[derive(Accounts)]
pub struct Proxy<'info> {
    #[account(
        mut,
        seeds = [SEED, target.key().as_ref()],
        bump
    )]
    pub lock: Account<'info, Lock>,

    #[account(
        mut,
        seeds = [SEED, CONFIG_SEED, target.key().as_ref()],
        bump
    )]
    pub lock_config: Account<'info, LockConfig>,

    #[account(
        mut,
        seeds = [SEED, SIGNER_SEED, target.key().as_ref()],
        bump
    )]
    pub pda_signer: SystemAccount<'info>,

    pub system_program: Program<'info, System>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(executable)]
    pub target: UncheckedAccount<'info>,
}

pub fn proxy_handler(
    ctx: Context<Proxy>,
    instruction_data: Vec<u8>,
    account_metas: Vec<u8>,
) -> Result<()> {
    let is_authorized = ctx
        .accounts
        .lock_config
        .is_authorized(ctx.accounts.payer.key());
    let is_expired = ctx.accounts.lock.is_expired();

    // If the lock is not expired and the caller is not authorized, we don't allow the transaction
    // to go through
    if !is_expired && !is_authorized {
        panic!("Unauthorized caller when lock is not expired");
    }

    // If we are authorized, we can refresh the lock
    if is_authorized {
        msg!("Refreshing lock");
        let lock_until = Clock::get()?.unix_timestamp + ctx.accounts.lock_config.lock_duration();
        ctx.accounts.lock.refresh(lock_until);
    }

    let program_id = ctx.accounts.target.key();

    let signer_seeds = &[
        SEED,
        SIGNER_SEED,
        program_id.as_ref(),
        &[ctx.bumps.pda_signer],
    ];

    let accounts = bincode::deserialize::<Vec<AccountMeta>>(&account_metas)
        .map_err(|_| ProgramError::InvalidInstructionData)?;
    let data = instruction_data;

    msg!("Invoking target program");
    invoke_signed(
        &Instruction {
            program_id,
            accounts,
            data,
        },
        ctx.remaining_accounts,
        &[signer_seeds],
    )?;

    Ok(())
}
