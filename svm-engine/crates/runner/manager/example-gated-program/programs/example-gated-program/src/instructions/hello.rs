use anchor_lang::prelude::*;

use crate::{Message, SEED};

#[derive(Accounts)]
pub struct Hello<'info> {
    #[account(
        seeds = [proxy::SEED, proxy::SIGNER_SEED, crate::id().as_ref()],
        seeds::program = proxy::id(),
        bump,
    )]
    pub pda_signer: Signer<'info>,

    #[account(
        init_if_needed,
        payer = payer,
        space = Message::DISCRIMINATOR.len() + Message::INIT_SPACE,
        seeds = [SEED, payer.key().as_ref()],
        bump
    )]
    pub message: Account<'info, Message>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn hello_handler(ctx: Context<Hello>, message: String) -> Result<()> {
    msg!(
        "Calling the target program with message: {} by {}",
        message,
        ctx.accounts.payer.key()
    );
    ctx.accounts.message.message = message;
    Ok(())
}
