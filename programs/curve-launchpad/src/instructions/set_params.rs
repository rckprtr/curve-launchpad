use crate::state::Global;
use anchor_lang::prelude::*;

#[event_cpi]
#[derive(Accounts)]
pub struct SetParams<'info> {
    #[account(
        mut,
        seeds = [Global::SEED_PREFIX],
        bump,
        has_one = authority,
    )]
    global: Box<Account<'info, Global>>,

    authority: Signer<'info>,

    new_authority: Option<UncheckedAccount<'info>>,

    fee_recipient: Option<UncheckedAccount<'info>>,

    withdraw_authority: Option<UncheckedAccount<'info>>,

    system_program: Program<'info, System>,
}

pub fn handle(
    ctx: Context<SetParams>,
    initial_virtual_token_reserves: Option<u64>,
    fee_basis_points: Option<u64>,
) -> Result<()> {
    let global = &mut ctx.accounts.global;

    if let Some(new_authority) = &ctx.accounts.new_authority {
        global.authority = new_authority.key();
    }

    if let Some(fee_recipient) = &ctx.accounts.fee_recipient {
        global.fee_recipient = fee_recipient.key();
    };

    if let Some(withdraw_authority) = &ctx.accounts.withdraw_authority {
        global.withdraw_authority = withdraw_authority.key();
    }

    if let Some(fee_basis_points) = fee_basis_points {
        global.fee_basis_points = fee_basis_points;
    }

    if let Some(initial_virtual_token_reserves) = initial_virtual_token_reserves {
        global.initial_virtual_token_reserves = initial_virtual_token_reserves;
    }

    Ok(())
}
