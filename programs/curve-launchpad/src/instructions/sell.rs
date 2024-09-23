use std::{char::MAX, str::FromStr};

use crate::{
    amm, calculate_fee, state::{BondingCurve, Global}, CurveLaunchpadError, TradeEvent
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};
declare_program!(dynamic_amm);
#[event_cpi]
#[derive(Accounts)]
pub struct Sell<'info> {
    #[account(mut)]
    user: Signer<'info>,

    #[account(
        seeds = [Global::SEED_PREFIX],
        bump,
    )]
    global: Box<Account<'info, Global>>,

    /// CHECK: Using global state to validate fee_recipient account
    #[account(mut)]
    fee_recipient: AccountInfo<'info>,

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
    

    //confirm user has enough tokens
    require!(
        ctx.accounts.user_token_account.amount >= token_amount,
        CurveLaunchpadError::InsufficientTokens,
    );

    //invalid fee recipient
    require!(
        ctx.accounts.fee_recipient.key == &ctx.accounts.global.fee_recipient,
        CurveLaunchpadError::InvalidFeeRecipient,
    );

    //confirm bonding curve has enough tokens
    require!(
        ctx.accounts.bonding_curve_token_account.amount >= token_amount,
        CurveLaunchpadError::InsufficientTokens,
    );

    require!(token_amount > 0, CurveLaunchpadError::MinSell,);

    let mut amm = amm::amm::AMM::new(
        ctx.accounts.bonding_curve.virtual_sol_reserves as u128,
        ctx.accounts.bonding_curve.virtual_token_reserves as u128,
        ctx.accounts.bonding_curve.real_sol_reserves as u128,
        ctx.accounts.bonding_curve.real_token_reserves as u128,
        ctx.accounts.global.initial_virtual_token_reserves as u128,
    );

    let sell_result = amm.apply_sell(token_amount as u128).unwrap();
    let fee = calculate_fee(sell_result.sol_amount, ctx.accounts.global.fee_basis_points);

    //the fee is subtracted from the sol amount to confirm the user minimum sol output is met
    let sell_amount_minus_fee = sell_result.sol_amount - fee;

    //confirm min sol output is greater than sol output
    require!(
        sell_amount_minus_fee >= min_sol_output,
        CurveLaunchpadError::MinSOLOutputExceeded,
    );

    //transfer SPL
    let cpi_accounts = Burn {
        from: ctx.accounts.user_token_account.to_account_info().clone(),
        mint: ctx.accounts.mint.to_account_info().clone(),
        authority: ctx.accounts.user.to_account_info().clone(),
    };

    token::burn(
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

    //transfer fee to fee recipient
    **from_account.to_account_info().try_borrow_mut_lamports()? -= fee;
    **ctx.accounts.fee_recipient.try_borrow_mut_lamports()? += fee;


    let bonding_curve = &mut ctx.accounts.bonding_curve;
    bonding_curve.real_token_reserves = amm.real_token_reserves as u64;
    bonding_curve.real_sol_reserves = amm.real_sol_reserves as u64;
    bonding_curve.virtual_token_reserves = amm.virtual_token_reserves as u64;
    bonding_curve.virtual_sol_reserves = amm.virtual_sol_reserves as u64;

    emit_cpi!(TradeEvent {
        mint: *ctx.accounts.mint.to_account_info().key,
        sol_amount: sell_result.sol_amount,
        token_amount: sell_result.token_amount,
        is_buy: false,
        user: *ctx.accounts.user.to_account_info().key,
        timestamp: Clock::get()?.unix_timestamp,
        virtual_sol_reserves: bonding_curve.virtual_sol_reserves,
        virtual_token_reserves: bonding_curve.virtual_token_reserves,
        real_sol_reserves: bonding_curve.real_sol_reserves,
        real_token_reserves: bonding_curve.real_token_reserves,
    });

    Ok(())
}

pub fn claim_fee(ctx: Context<ClaimFee>) -> Result<()> {
    let accounts = dynamic_amm::cpi::accounts::ClaimFee {
        pool: ctx.accounts.pool.to_account_info(),
        lp_mint: ctx.accounts.lp_mint.to_account_info(),
        lock_escrow: ctx.accounts.lock_escrow.to_account_info(),
        owner: ctx.accounts.bonding_curve.to_account_info(),
        source_tokens: ctx.accounts.user_a_token.to_account_info(),
        escrow_vault: ctx.accounts.lock_escrow.to_account_info(),
        token_program: ctx.accounts.token_program.to_account_info(),
        a_token_vault: ctx.accounts.a_token_vault.to_account_info(),
        b_token_vault: ctx.accounts.b_token_vault.to_account_info(),
        a_vault: ctx.accounts.a_vault.to_account_info(),
        b_vault: ctx.accounts.b_vault.to_account_info(),
        a_vault_lp: ctx.accounts.a_vault_lp.to_account_info(),
        b_vault_lp: ctx.accounts.b_vault_lp.to_account_info(),
        a_vault_lp_mint: ctx.accounts.a_vault_lp_mint.to_account_info(),
        b_vault_lp_mint: ctx.accounts.b_vault_lp_mint.to_account_info(),
        user_a_token: ctx.accounts.user_a_token.to_account_info(),
        user_b_token: ctx.accounts.user_b_token.to_account_info(),
        vault_program: ctx.accounts.vault_program.to_account_info(),
    };
    let signer: [&[&[u8]]; 1] = [&[
        BondingCurve::SEED_PREFIX,
        ctx.accounts.a_vault_lp_mint.to_account_info().key.as_ref(),
        &[ctx.bumps.bonding_curve],
    ]];


    let cpi_context = CpiContext::new_with_signer(ctx.accounts.vault_program.to_account_info(), accounts, &signer);
    dynamic_amm::cpi::claim_fee(cpi_context, u64::MAX)?;


    Ok(())
}

#[derive(Accounts)]
pub struct ClaimFee<'info> {
    
    /// CHECK: Pool account
    #[account(mut)]
    pub pool: UncheckedAccount<'info>,

    /// CHECK: LP token mint of the pool
    #[account(mut)]
    pub lp_mint: UncheckedAccount<'info>,

    /// CHECK: Lock account
    #[account(mut)]
    pub lock_escrow: UncheckedAccount<'info>,

    /// CHECK: Owner of lock account
    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: owner lp token account
    #[account(mut)]
    pub source_tokens: UncheckedAccount<'info>,

    /// CHECK: Escrow vault
    #[account(mut)]
    pub escrow_vault: UncheckedAccount<'info>,

    /// CHECK: Token program.
    pub token_program: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Token vault account of vault A
    pub a_token_vault: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Token vault account of vault B
    pub b_token_vault: UncheckedAccount<'info>,

    /// CHECK: Vault account for token a. token a of the pool will be deposit / withdraw from this vault account.
    #[account(mut)]
    pub a_vault: UncheckedAccount<'info>,
    /// CHECK: Vault account for token b. token b of the pool will be deposit / withdraw from this vault account.
    #[account(mut)]
    pub b_vault: UncheckedAccount<'info>,
    /// CHECK: LP token account of vault A. Used to receive/burn the vault LP upon deposit/withdraw from the vault.
    #[account(mut)]
    pub a_vault_lp: UncheckedAccount<'info>,
    /// CHECK: LP token account of vault B. Used to receive/burn the vault LP upon deposit/withdraw from the vault.
    #[account(mut)]
    pub b_vault_lp: UncheckedAccount<'info>,
    /// CHECK: LP token mint of vault a
    #[account(mut)]
    pub a_vault_lp_mint: Account<'info, Mint>,
    /// CHECK: LP token mint of vault b
    #[account(mut)]
    pub b_vault_lp_mint: Account<'info, Mint>,

    #[account(mut, associated_token::mint = a_vault_lp_mint, associated_token::authority = Pubkey::from_str("6DVUvbq19v7EyBANDoqmJ2zvVpjpnYeKKY4fGaMa1iuL").unwrap())]
    /// CHECK: User token A account. Token will be transfer from this account if it is add liquidity operation. Else, token will be transfer into this account.
    pub user_a_token: Account<'info, TokenAccount>,
    #[account(mut, associated_token::mint = b_vault_lp_mint, associated_token::authority = Pubkey::from_str("6DVUvbq19v7EyBANDoqmJ2zvVpjpnYeKKY4fGaMa1iuL").unwrap())]
    /// CHECK: User token B account. Token will be transfer from this account if it is add liquidity operation. Else, token will be transfer into this account.
    pub user_b_token: Account<'info, TokenAccount>,

    /// CHECK: Vault program. the pool will deposit/withdraw liquidity from the vault.
    pub vault_program: UncheckedAccount<'info>,

    #[account(
        seeds = [BondingCurve::SEED_PREFIX, a_vault_lp_mint.to_account_info().key.as_ref()],
        bump,
    )]
    pub bonding_curve: Box<Account<'info, BondingCurve>>,
}
