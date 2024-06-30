use anchor_lang::{prelude::*, solana_program::system_instruction};
use anchor_spl::token_interface::{self as token, Mint, TokenInterface, TokenAccount, TransferChecked};

use crate::{
    amm, calculate_fee, state::{BondingCurve, Global}, CompleteEvent, CurveLaunchpadError, TradeEvent
};

#[event_cpi]
#[derive(Accounts)]
pub struct Buy<'info> {
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

    mint: InterfaceAccount<'info, Mint>,

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
    bonding_curve_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = user,
    )]
    user_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    system_program: Program<'info, System>,

    token_program: Interface<'info, TokenInterface>,
}

pub fn buy(ctx: Context<Buy>, token_amount: u64, max_sol_cost: u64) -> Result<()> {
    require!(
        ctx.accounts.global.initialized,
        CurveLaunchpadError::NotInitialized
    );

    //bonding curve is not complete
    require!(
        ctx.accounts.bonding_curve.complete == false,
        CurveLaunchpadError::BondingCurveComplete,
    );

    //invalid fee recipient
    require!(
        ctx.accounts.fee_recipient.key == &ctx.accounts.global.fee_recipient,
        CurveLaunchpadError::InvalidFeeRecipient,
    );

    //bonding curve has enough tokens
    require!(
        ctx.accounts.bonding_curve.real_token_reserves >= token_amount,
        CurveLaunchpadError::InsufficientTokens,
    );

    require!(token_amount > 0, CurveLaunchpadError::MinBuy,);

    let targe_token_amount = if ctx.accounts.bonding_curve_token_account.amount < token_amount {
        ctx.accounts.bonding_curve_token_account.amount
    } else {
        token_amount
    };

    let mut amm = amm::amm::AMM::new(
        ctx.accounts.bonding_curve.virtual_sol_reserves as u128,
        ctx.accounts.bonding_curve.virtual_token_reserves as u128,
        ctx.accounts.bonding_curve.real_sol_reserves as u128,
        ctx.accounts.bonding_curve.real_token_reserves as u128,
        ctx.accounts.global.initial_virtual_token_reserves as u128,
    );

    let buy_result = amm.apply_buy(targe_token_amount as u128).unwrap();
    let fee = calculate_fee(buy_result.sol_amount, ctx.accounts.global.fee_basis_points);
    let buy_amount_with_fee = buy_result.sol_amount + fee;

    //check if the amount of SOL to transfe plus fee is less than the max_sol_cost
    require!(
        buy_amount_with_fee <= max_sol_cost,
        CurveLaunchpadError::MaxSOLCostExceeded,
    );

    //check if the user has enough SOL
    require!(
        ctx.accounts.user.lamports() >= buy_amount_with_fee,
        CurveLaunchpadError::InsufficientSOL,
    );
    
    // transfer SOL to bonding curve
    let from_account = &ctx.accounts.user;
    let to_bonding_curve_account = &ctx.accounts.bonding_curve;

    let transfer_instruction = system_instruction::transfer(
        from_account.key,
        to_bonding_curve_account.to_account_info().key,
        buy_result.sol_amount,
    );

    anchor_lang::solana_program::program::invoke_signed(
        &transfer_instruction,
        &[
            from_account.to_account_info(),
            to_bonding_curve_account.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        &[],
    )?;

    //transfer SOL to fee recipient
    let to_fee_recipient_account = &ctx.accounts.fee_recipient;

    let transfer_instruction = system_instruction::transfer(
        from_account.key,
        to_fee_recipient_account.key,
        fee,
    );

    anchor_lang::solana_program::program::invoke_signed(
        &transfer_instruction,
        &[
            from_account.to_account_info(),
            to_fee_recipient_account.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        &[],
    )?;

    //transfer SPL
    let cpi_accounts = TransferChecked {
        from: ctx
            .accounts
            .bonding_curve_token_account
            .to_account_info()
            .clone(),
        to: ctx.accounts.user_token_account.to_account_info().clone(),
        authority: ctx.accounts.bonding_curve.to_account_info().clone(),
        mint: ctx.accounts.mint.to_account_info().clone(),
    };

    let signer: [&[&[u8]]; 1] = [&[
        BondingCurve::SEED_PREFIX,
        ctx.accounts.mint.to_account_info().key.as_ref(),
        &[ctx.bumps.bonding_curve],
    ]];

    token::transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            &signer,
        ),
        buy_result.token_amount,
        crate::DEFAULT_DECIMALS.try_into().unwrap()
    )?;

    //apply the buy to the bonding curve
    let bonding_curve = &mut ctx.accounts.bonding_curve;
    bonding_curve.real_token_reserves = amm.real_token_reserves as u64;
    bonding_curve.real_sol_reserves = amm.real_sol_reserves as u64;
    bonding_curve.virtual_token_reserves = amm.virtual_token_reserves as u64;
    bonding_curve.virtual_sol_reserves = amm.virtual_sol_reserves as u64;

    emit_cpi!(TradeEvent {
        mint: *ctx.accounts.mint.to_account_info().key,
        sol_amount: buy_result.sol_amount,
        token_amount: buy_result.token_amount,
        is_buy: true,
        user: *ctx.accounts.user.to_account_info().key,
        timestamp: Clock::get()?.unix_timestamp,
        virtual_sol_reserves: bonding_curve.virtual_sol_reserves,
        virtual_token_reserves: bonding_curve.virtual_token_reserves,
        real_sol_reserves: bonding_curve.real_sol_reserves,
        real_token_reserves: bonding_curve.real_token_reserves,
    });

    if bonding_curve.real_token_reserves == 0 {
        bonding_curve.complete = true;

        emit_cpi!(CompleteEvent {
            user: *ctx.accounts.user.to_account_info().key,
            mint: *ctx.accounts.mint.to_account_info().key,
            bonding_curve: *ctx.accounts.bonding_curve.to_account_info().key,
            timestamp: Clock::get()?.unix_timestamp,
        });
    }

    msg!("bonding_curve: {:?}", amm);

    Ok(())
}
