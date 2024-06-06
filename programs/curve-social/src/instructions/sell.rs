use crate::state::{BondingCurve, Global};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
#[derive(Accounts)]
pub struct Sell<'info> {
    #[account(mut)]
    user: Signer<'info>,

    #[account(
        seeds = [Global::SEED_PREFIX],
        bump,
    )]
    global: Box<Account<'info, Global>>,

    mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [BondingCurve::SEED_PREFIX, mint.to_account_info().key.as_ref()],
        bump,
    )]
    bonding_curve: Box<Account<'info, BondingCurve>>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = bonding_curve,
    )]
    bonding_curve_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = user,
    )]
    user_token_account: Box<Account<'info, TokenAccount>>,

    system_program: Program<'info, System>,

    token_program: Program<'info, Token>,
}

pub fn sell(ctx: Context<Sell>, token_amount: u64, min_sol_output: u64) -> Result<()> {

    //transfer SPL
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_token_account.to_account_info().clone(),
        to: ctx
            .accounts
            .bonding_curve_token_account
            .to_account_info()
            .clone(),
        authority: ctx.accounts.user.to_account_info().clone(),
    };

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            &[],
        ),
        token_amount,
    )?;

    //transfer SOL back to user
    let from_account = &ctx.accounts.bonding_curve;
    let to_account = &ctx.accounts.user;
    
    **from_account.to_account_info().try_borrow_mut_lamports()? -= min_sol_output;
    **to_account.try_borrow_mut_lamports()? += min_sol_output;


    let bonding_curve = &mut ctx.accounts.bonding_curve;
    bonding_curve.real_token_reserve += token_amount;
    bonding_curve.real_sol_reserve -= min_sol_output;

    Ok(())
}
