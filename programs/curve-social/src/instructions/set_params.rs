use crate::{state::Global, CurveSocialError};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct SetParams<'info> {
    #[account(
        mut,
        seeds = [Global::SEED_PREFIX],
        bump,
    )]
    global: Box<Account<'info, Global>>,

    user: Signer<'info>,

    system_program: Program<'info, System>,
}

pub fn set_params(
    ctx: Context<SetParams>,
    fee_recipient: Pubkey,
    initial_virtual_token_reserves: u64,
    initial_virtual_sol_reserves: u64,
    initial_real_token_reserves: u64,
    token_total_supply: u64,
    fee_basis_points: u64,
) -> Result<()> {
    let global = &mut ctx.accounts.global;

    if global.authority != *ctx.accounts.user.to_account_info().key {
        return Err(CurveSocialError::InvalidAuthority.into());
    }

    if !global.initialized {
        return Err(CurveSocialError::NotInitialized.into());
    }
    
    global.fee_recipient = fee_recipient;
    global.initial_virtual_token_reserves = initial_virtual_token_reserves;
    global.initial_virtual_sol_reserves = initial_virtual_sol_reserves;
    global.initial_real_token_reserves = initial_real_token_reserves;
    global.initial_token_supply = token_total_supply;
    global.fee_basis_points = fee_basis_points;

    Ok(())
}
