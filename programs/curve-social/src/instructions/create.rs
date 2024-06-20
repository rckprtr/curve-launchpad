use crate::{
    state::{BondingCurve, Global}, CreateEvent, CurveSocialError, DEFAULT_DECIMALS
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::{
        create_metadata_accounts_v3, mpl_token_metadata::types::DataV2, CreateMetadataAccountsV3,
        Metadata as Metaplex,
    },
    token::{
        self, mint_to, spl_token::instruction::AuthorityType, Mint, MintTo, Token, TokenAccount,
    },
};

#[event_cpi]
#[derive(Accounts)]
pub struct Create<'info> {
    #[account(
        init,
        payer = creator,
        mint::decimals = DEFAULT_DECIMALS as u8,
        mint::authority = mint_authority,
        mint::freeze_authority = mint_authority
    )]
    mint: Account<'info, Mint>,

    #[account(mut)]
    creator: Signer<'info>,

    /// CHECK: Using seed to validate mint_authority account
    #[account(
        seeds=[b"mint-authority"],
        bump,
    )]
    mint_authority: AccountInfo<'info>,

    #[account(
        init,
        payer = creator,
        seeds = [BondingCurve::SEED_PREFIX, mint.to_account_info().key.as_ref()],
        bump,
        space = 8 + BondingCurve::INIT_SPACE,
    )]
    bonding_curve: Box<Account<'info, BondingCurve>>,

    #[account(
        init_if_needed,
        payer = creator,
        associated_token::mint = mint,
        associated_token::authority = bonding_curve,
    )]
    bonding_curve_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [Global::SEED_PREFIX],
        bump,
    )]
    global: Box<Account<'info, Global>>,

    ///CHECK: Using seed to validate metadata account
    #[account(
        mut,
        seeds = [
            b"metadata", 
            token_metadata_program.key.as_ref(), 
            mint.to_account_info().key.as_ref()
        ],
        seeds::program = token_metadata_program.key(),
        bump,
    )]
    metadata: AccountInfo<'info>,

    system_program: Program<'info, System>,

    token_program: Program<'info, Token>,

    associated_token_program: Program<'info, AssociatedToken>,

    token_metadata_program: Program<'info, Metaplex>,

    rent: Sysvar<'info, Rent>,
}


pub fn create(ctx: Context<Create>, name: String, symbol: String, uri: String) -> Result<()> {
    //confirm program is initialized
    require!(
        ctx.accounts.global.initialized,
        CurveSocialError::NotInitialized
    );

    let seeds = &["mint-authority".as_bytes(), &[ctx.bumps.mint_authority]];
    let signer = [&seeds[..]];

    let token_data: DataV2 = DataV2 {
        name: name.clone(),
        symbol: symbol.clone(),
        uri: uri.clone(),
        seller_fee_basis_points: 0,
        creators: None,
        collection: None,
        uses: None,
    };

    let metadata_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_metadata_program.to_account_info(),
        CreateMetadataAccountsV3 {
            payer: ctx.accounts.creator.to_account_info(),
            update_authority: ctx.accounts.mint_authority.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            metadata: ctx.accounts.metadata.to_account_info(),
            mint_authority: ctx.accounts.mint_authority.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        },
        &signer,
    );

    create_metadata_accounts_v3(metadata_ctx, token_data, false, true, None)?;

    //mint tokens to bonding_curve_token_account
    mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                authority: ctx.accounts.mint_authority.to_account_info(),
                to: ctx.accounts.bonding_curve_token_account.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
            },
            &signer,
        ),
        ctx.accounts.global.initial_token_supply,
    )?;

    //remove mint_authority
    let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        token::SetAuthority {
            current_authority: ctx.accounts.mint_authority.to_account_info(),
            account_or_mint: ctx.accounts.mint.to_account_info(),
        },
        &signer,
    );
    token::set_authority(cpi_context, AuthorityType::MintTokens, None)?;

    let bonding_curve = &mut ctx.accounts.bonding_curve;
    bonding_curve.virtual_sol_reserves = ctx.accounts.global.initial_virtual_sol_reserves;
    bonding_curve.virtual_token_reserves = ctx.accounts.global.initial_virtual_token_reserves;
    bonding_curve.real_sol_reserves = 0;
    bonding_curve.real_token_reserves = ctx.accounts.global.initial_real_token_reserves;
    bonding_curve.token_total_supply = ctx.accounts.global.initial_token_supply;
    bonding_curve.complete = false;

    emit_cpi!(CreateEvent {
        name,
        symbol,
        uri,
        mint: *ctx.accounts.mint.to_account_info().key,
        bonding_curve: *ctx.accounts.bonding_curve.to_account_info().key,
        creator: *ctx.accounts.creator.to_account_info().key,
    });

    Ok(())
}
