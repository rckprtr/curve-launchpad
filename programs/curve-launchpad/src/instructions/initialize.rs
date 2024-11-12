use anchor_lang::prelude::*;

use crate::state::Global;

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

    /// CHECK: fee recipient account
    fee_recipient: UncheckedAccount<'info>,

    /// CHECK: withdraw authority account
    withdraw_authority: UncheckedAccount<'info>,

    system_program: Program<'info, System>,
}

pub fn handle(ctx: Context<Initialize>) -> Result<()> {
    let global = &mut ctx.accounts.global;

    global.authority = ctx.accounts.authority.to_account_info().key();
    global.fee_recipient = ctx.accounts.fee_recipient.to_account_info().key();
    global.withdraw_authority = ctx.accounts.withdraw_authority.to_account_info().key();
    global.fee_basis_points = 0;
    global.initial_virtual_token_reserves = 0;

    Ok(())
}
