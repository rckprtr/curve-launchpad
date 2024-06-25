use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};

use crate::{
    state::{BondingCurve, Global, LastWithdraw},
    CurveLaunchpadError,
};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    user: Signer<'info>,

    #[account(
        seeds = [Global::SEED_PREFIX],
        bump,
    )]
    global: Box<Account<'info, Global>>,

    mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        space = 8 + LastWithdraw::INIT_SPACE,
        seeds = [LastWithdraw::SEED_PREFIX],
        bump,
        payer = user,
    )]
    last_withdraw: Box<Account<'info, LastWithdraw>>,

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
        init_if_needed,
        payer = user,
        associated_token::mint = mint,
        associated_token::authority = user,
    )]
    user_token_account: Box<Account<'info, TokenAccount>>,

    associated_token_program: Program<'info, AssociatedToken>,

    system_program: Program<'info, System>,

    token_program: Program<'info, Token>,
}

pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
    require!(
        ctx.accounts.global.initialized,
        CurveLaunchpadError::NotInitialized
    );

    require!(
        ctx.accounts.bonding_curve.complete == true,
        CurveLaunchpadError::BondingCurveNotComplete,
    );

    require!(
        ctx.accounts.user.key() == ctx.accounts.global.withdraw_authority,
        CurveLaunchpadError::InvalidWithdrawAuthority,
    );

    //transfer tokens to withdraw authority from bonding curve
    let cpi_accounts = Transfer {
        from: ctx
            .accounts
            .bonding_curve_token_account
            .to_account_info()
            .clone(),
        to: ctx.accounts.user_token_account.to_account_info().clone(),
        authority: ctx.accounts.bonding_curve.to_account_info().clone(),
    };

    let signer: [&[&[u8]]; 1] = [&[
        BondingCurve::SEED_PREFIX,
        ctx.accounts.mint.to_account_info().key.as_ref(),
        &[ctx.bumps.bonding_curve],
    ]];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            &signer,
        ),
        ctx.accounts.bonding_curve_token_account.amount,
    )?;

    //transer sol to withdraw authority from bonding curve
    let from_account = &ctx.accounts.bonding_curve;
    let to_account = &ctx.accounts.user;

    let min_balance = Rent::get()?.minimum_balance(8 + BondingCurve::INIT_SPACE as usize);

    let total_bonding_curve_lamports = from_account.get_lamports() - min_balance;

    **from_account.to_account_info().try_borrow_mut_lamports()? -= total_bonding_curve_lamports;
    **to_account.try_borrow_mut_lamports()? += total_bonding_curve_lamports;

    //update last withdraw
    let last_withdraw = &mut ctx.accounts.last_withdraw;
    last_withdraw.last_withdraw_timestamp = Clock::get()?.unix_timestamp;

    Ok(())
}
