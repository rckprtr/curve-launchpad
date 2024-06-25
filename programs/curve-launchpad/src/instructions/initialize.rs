use crate::{state::Global, CurveLaunchpadError, DEFAULT_TOKEN_SUPPLY};
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

    require!(
        !global.initialized,
        CurveLaunchpadError::AlreadyInitialized,
    );

    global.authority = *ctx.accounts.authority.to_account_info().key;
    global.initialized = true;
    global.initial_token_supply = DEFAULT_TOKEN_SUPPLY;
    global.initial_real_sol_reserves = 0;
    global.initial_real_token_reserves = DEFAULT_TOKEN_SUPPLY;
    global.initial_virtual_sol_reserves = 30_000_000_000;
    global.initial_virtual_token_reserves = 1_073_000_000_000_000;
    global.fee_basis_points = 50;

    msg!("Initialized global state");

    Ok(())
}