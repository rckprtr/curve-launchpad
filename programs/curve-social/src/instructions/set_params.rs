use crate::{state::Global, CurveSocialError, SetParamsEvent};
use anchor_lang::prelude::*;

#[event_cpi]
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
    initial_token_supply: u64,
    fee_basis_points: u64,
) -> Result<()> {
    let global = &mut ctx.accounts.global;

    //confirm program is initialized
    require!(
        global.initialized,
        CurveSocialError::NotInitialized
    );

    //confirm user is the authority
    require!(
        global.authority == *ctx.accounts.user.to_account_info().key,
        CurveSocialError::InvalidAuthority
    );
    
    global.fee_recipient = fee_recipient;
    global.initial_virtual_token_reserves = initial_virtual_token_reserves;
    global.initial_virtual_sol_reserves = initial_virtual_sol_reserves;
    global.initial_real_token_reserves = initial_real_token_reserves;
    global.initial_token_supply = initial_token_supply;
    global.fee_basis_points = fee_basis_points;

    emit_cpi!(SetParamsEvent {
        fee_recipient,
        initial_virtual_token_reserves,
        initial_virtual_sol_reserves,
        initial_real_token_reserves,
        initial_token_supply,
        fee_basis_points,
    });

    Ok(())
}
