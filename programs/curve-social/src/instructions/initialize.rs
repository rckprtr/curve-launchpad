use crate::{state::Global, CurveSocialError};
use anchor_lang::prelude::*;


#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    authority: Signer<'info>,

    #[account(
        init,
        space = 8 + Global::INIT_SPACE,
        seeds = [Global::SEED_PREFIX],
        bump,
        payer = authority,
    )]
    global: Box<Account<'info, Global>>,

    system_program: Program<'info, System>,
}


pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
    let global = &mut ctx.accounts.global;

    if global.initialized {
        return Err(CurveSocialError::AlreadyInitialized.into());
    }

    global.authority = *ctx.accounts.authority.to_account_info().key;
    global.initialized = true;
    global.initial_token_supply = 10_000_000;

    msg!("Initialized global state");

    Ok(())
}