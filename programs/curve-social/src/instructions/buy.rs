use anchor_lang::{prelude::*, solana_program::system_instruction};
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::{
    amm,
    state::{BondingCurve, Global},
    CurveSocialError,
};

#[derive(Accounts)]
pub struct Buy<'info> {
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

pub fn buy(ctx: Context<Buy>, token_amount: u64, max_sol_cost: u64) -> Result<()> {
    let _global = &mut ctx.accounts.global;

    if ctx.accounts.bonding_curve.complete {
        return Err(CurveSocialError::BondingCurveComplete.into());
    }

    let targe_token_amount = if ctx.accounts.bonding_curve_token_account.amount < token_amount {
        ctx.accounts.bonding_curve_token_account.amount
    } else {
        token_amount
    };

    let amm = amm::amm::AMM::new(
        ctx.accounts.bonding_curve.virtual_sol_reserves,
        ctx.accounts.bonding_curve.virtual_token_reserves,
        ctx.accounts.bonding_curve.real_sol_reserves,
        ctx.accounts.bonding_curve.real_token_reserves,
        ctx.accounts.global.initial_virtual_token_reserves,
    );

    let buy_price = amm.get_buy_price(targe_token_amount);

    if buy_price > max_sol_cost || buy_price == 0 {
        return Err(CurveSocialError::InsufficientSOL.into());
    }

    if ctx.accounts.user.lamports() < buy_price {
        return Err(CurveSocialError::InsufficientSOL.into());
    }

    msg!("value of token: {}", buy_price);
    msg!("max_sol_cost: {}", max_sol_cost);

    let from_account = &ctx.accounts.user;
    let to_account = &ctx.accounts.bonding_curve;

    // transfer SOL
    let transfer_instruction = system_instruction::transfer(
        from_account.key,
        to_account.to_account_info().key,
        buy_price,
    );

    anchor_lang::solana_program::program::invoke_signed(
        &transfer_instruction,
        &[
            from_account.to_account_info(),
            to_account.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        &[],
    )?;

    //transfer SPL
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
        token_amount,
    )?;

    let bonding_curve = &mut ctx.accounts.bonding_curve;
    bonding_curve.real_token_reserves -= targe_token_amount;
    bonding_curve.real_sol_reserves += buy_price;

    if bonding_curve.real_token_reserves == 0 {
        bonding_curve.complete = true;
    }

    Ok(())
}
