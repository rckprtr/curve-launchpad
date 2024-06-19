use crate::{
    amm,
    state::{BondingCurve, Global},
    CurveSocialError,
};
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
    //check if bonding curve is complete
    require!(
        !ctx.accounts.bonding_curve.complete,
        CurveSocialError::BondingCurveComplete,
    );

    //confirm user has enough tokens
    require!(
        ctx.accounts.user_token_account.amount >= token_amount,
        CurveSocialError::InsufficientTokens,
    );

    //confirm bonding curve has enough tokens
    require!(
        ctx.accounts.bonding_curve_token_account.amount >= token_amount,
        CurveSocialError::InsufficientTokens,
    );

    require!(
        token_amount > 0,
        CurveSocialError::MinSell,
    );

    let mut amm = amm::amm::AMM::new(
        ctx.accounts.bonding_curve.virtual_sol_reserves as u128,
        ctx.accounts.bonding_curve.virtual_token_reserves as u128,
        ctx.accounts.bonding_curve.real_sol_reserves as u128,
        ctx.accounts.bonding_curve.real_token_reserves as u128,
        ctx.accounts.global.initial_virtual_token_reserves as u128,
    );

    msg!("{}", amm);

    let sell_result = amm.apply_sell(token_amount as u128);

    //confirm min sol output is greater than sol output
    require!(
        sell_result.sol_amount >= min_sol_output,
        CurveSocialError::MinSOLOutputExceeded,
    );

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
        sell_result.token_amount,
    )?;

    //transfer SOL back to user
    //TODO: check if this is correct
    let from_account = &ctx.accounts.bonding_curve;
    let to_account = &ctx.accounts.user;

    **from_account.to_account_info().try_borrow_mut_lamports()? -= sell_result.sol_amount;
    **to_account.try_borrow_mut_lamports()? += sell_result.sol_amount;

    let bonding_curve = &mut ctx.accounts.bonding_curve;
    bonding_curve.real_token_reserves = amm.real_token_reserves as u64;
    bonding_curve.real_sol_reserves = amm.real_sol_reserves as u64;
    bonding_curve.virtual_token_reserves = amm.virtual_token_reserves as u64;
    bonding_curve.virtual_sol_reserves = amm.virtual_sol_reserves as u64;

    Ok(())
}
