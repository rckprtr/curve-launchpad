use crate::{
    state::{BondingCurve, Global}, CreateEvent, CurveLaunchpadError, DEFAULT_DECIMALS
};
use anchor_lang::{prelude::*, solana_program::program::invoke};
use anchor_spl::{
    associated_token::{spl_associated_token_account::instruction::create_associated_token_account, AssociatedToken},
    metadata::{
        create_metadata_accounts_v3, mpl_token_metadata::types::DataV2, CreateMetadataAccountsV3,
        Metadata as Metaplex,
    },
    token::{
        mint_to,  Mint, MintTo, Token, TokenAccount,
    },
};
declare_program!(dynamic_amm);
#[event_cpi]
#[derive(Accounts)]
pub struct Create<'info> {
    #[account(
        init,
        payer = creator,
        mint::decimals = DEFAULT_DECIMALS as u8,
        mint::authority = mint_authority,
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
        CurveLaunchpadError::NotInitialized
    );

    msg!("create::BondingCurve::get_lamports: {:?}", &ctx.accounts.bonding_curve.get_lamports());

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
    /*let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        token::SetAuthority {
            current_authority: ctx.accounts.mint_authority.to_account_info(),
            account_or_mint: ctx.accounts.mint.to_account_info(),
        },
        &signer,
    );
    token::set_authority(cpi_context, AuthorityType::MintTokens, None)?;
    */
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
pub fn create_permissionless_constant_product_pool_with_config(ctx: Context<CreatePermissionlessConstantProductPoolWithConfig>) -> Result<()> {
    let accounts = dynamic_amm::cpi::accounts::InitializePermissionlessConstantProductPoolWithConfig {
        pool: ctx.accounts.pool.to_account_info(),
        config: ctx.accounts.config.to_account_info(),
        lp_mint: ctx.accounts.lp_mint.to_account_info(),
        token_a_mint: ctx.accounts.token_a_mint.to_account_info(),
        token_b_mint: ctx.accounts.token_b_mint.to_account_info(),
        a_vault: ctx.accounts.a_vault.to_account_info(),
        b_vault: ctx.accounts.b_vault.to_account_info(),
        a_token_vault: ctx.accounts.a_token_vault.to_account_info(),
        b_token_vault: ctx.accounts.b_token_vault.to_account_info(),
        a_vault_lp_mint: ctx.accounts.a_vault_lp_mint.to_account_info(),
        b_vault_lp_mint: ctx.accounts.b_vault_lp_mint.to_account_info(),
        a_vault_lp: ctx.accounts.a_vault_lp.to_account_info(),
        b_vault_lp: ctx.accounts.b_vault_lp.to_account_info(),
        payer_token_a: ctx.accounts.payer_token_a.to_account_info(),
        payer_token_b: ctx.accounts.payer_token_b.to_account_info(),
        payer_pool_lp: ctx.accounts.payer_pool_lp.to_account_info(),
        protocol_token_a_fee: ctx.accounts.protocol_token_a_fee.to_account_info(),
        protocol_token_b_fee: ctx.accounts.protocol_token_b_fee.to_account_info(),
        payer: ctx.accounts.payer.to_account_info(),
        rent: ctx.accounts.rent.to_account_info(),
        mint_metadata: ctx.accounts.mint_metadata.to_account_info(),
        metadata_program: ctx.accounts.metadata_program.to_account_info(),
        vault_program: ctx.accounts.vault_program.to_account_info(),
        token_program: ctx.accounts.token_program.to_account_info(),
        associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
    };
    let seeds = [BondingCurve::SEED_PREFIX, ctx.accounts.token_b_mint.to_account_info().key.as_ref(), &[ctx.bumps.bonding_curve]];
    let signer = [&seeds[..]];

    let cpi_context = CpiContext::new_with_signer(ctx.accounts.dynamic_amm_program.to_account_info(), accounts, &signer);

    dynamic_amm::cpi::initialize_permissionless_constant_product_pool_with_config(cpi_context, ctx.accounts.payer_token_a.amount, ctx.accounts.payer_token_b.amount)?;
    let accounts = dynamic_amm::cpi::accounts::CreateLockEscrow {
        lock_escrow: ctx.accounts.lock_escrow.to_account_info(),
        payer: ctx.accounts.payer.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        pool: ctx.accounts.pool.to_account_info(),
        owner: ctx.accounts.payer.to_account_info(),
        lp_mint: ctx.accounts.lp_mint.to_account_info(),
    };
    let cpi_context = CpiContext::new_with_signer(ctx.accounts.vault_program.to_account_info(), accounts, &signer);
    dynamic_amm::cpi::create_lock_escrow(cpi_context)?;
    let accounts = create_associated_token_account(ctx.accounts.payer.to_account_info().key, ctx.accounts.lock_escrow.to_account_info().key, ctx.accounts.lp_mint.to_account_info().key, ctx.accounts.token_program.to_account_info().key);
    
    invoke(&accounts, &
    [
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.lock_escrow.to_account_info(),
        ctx.accounts.lp_mint.to_account_info(),
        ctx.accounts.lock_escrow_token_account.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
    ]
    )?;

    let accounts = dynamic_amm::cpi::accounts::Lock {
        pool: ctx.accounts.pool.to_account_info(),
        lp_mint: ctx.accounts.lp_mint.to_account_info(),
        lock_escrow: ctx.accounts.lock_escrow.to_account_info(),
        owner: ctx.accounts.payer.to_account_info(),
        source_tokens: ctx.accounts.payer_token_a.to_account_info(),
        escrow_vault: ctx.accounts.lock_escrow_token_account.to_account_info(),
        token_program: ctx.accounts.token_program.to_account_info(),
        a_vault: ctx.accounts.a_vault.to_account_info(),
        b_vault: ctx.accounts.b_vault.to_account_info(),
            a_vault_lp: ctx.accounts.a_vault_lp.to_account_info(),
        b_vault_lp: ctx.accounts.b_vault_lp.to_account_info(),
        a_vault_lp_mint: ctx.accounts.a_vault_lp_mint.to_account_info(),
        b_vault_lp_mint: ctx.accounts.b_vault_lp_mint.to_account_info(),
    };
    let cpi_context = CpiContext::new_with_signer(ctx.accounts.vault_program.to_account_info(), accounts, &signer);
    dynamic_amm::cpi::lock(cpi_context, ctx.accounts.payer_token_a.amount)?;
    Ok(())
}
#[derive(Accounts)]
pub struct CreatePermissionlessConstantProductPoolWithConfig<'info> {
    #[account(mut)]
    pub pool: UncheckedAccount<'info>,

    pub config: UncheckedAccount<'info>,

    /// LP token mint of the pool
    #[account(
       mut
    )]
    pub lp_mint: UncheckedAccount<'info>,

    /// Token A mint of the pool. Eg: USDT
    pub token_a_mint: Box<Account<'info, Mint>>,
    /// Token B mint of the pool. Eg: USDC
    pub token_b_mint: Box<Account<'info, Mint>>,

    #[account(mut)]
    /// CHECK: Vault account for token A. Token A of the pool will be deposit / withdraw from this vault account.
    pub a_vault: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Vault account for token B. Token B of the pool will be deposit / withdraw from this vault account.
    pub b_vault: UncheckedAccount<'info>,

    #[account(mut)]
    /// Token vault account of vault A
    pub a_token_vault: UncheckedAccount<'info>,
    #[account(mut)]
    /// Token vault account of vault B
    pub b_token_vault: UncheckedAccount<'info>,

    #[account(mut)]
    /// LP token mint of vault A
    pub a_vault_lp_mint: UncheckedAccount<'info>,

    #[account(mut)]
    /// LP token mint of vault B
    pub b_vault_lp_mint: UncheckedAccount<'info>,
    /// LP token account of vault A. Used to receive/burn the vault LP upon deposit/withdraw from the vault.
    #[account(
        mut
    )]
    pub a_vault_lp: UncheckedAccount<'info>,
    /// LP token account of vault B. Used to receive/burn vault LP upon deposit/withdraw from the vault.
    #[account(
        mut
    )]
    pub b_vault_lp: UncheckedAccount<'info>,

    #[account(mut)]
    /// Payer token account for pool token A mint. Used to bootstrap the pool with initial liquidity.
    pub payer_token_a: Account<'info, TokenAccount>,

    #[account(mut)]
    /// Admin token account for pool token B mint. Used to bootstrap the pool with initial liquidity.
    pub payer_token_b: Account<'info, TokenAccount>,

    /// CHECK: Payer pool LP token account. Used to receive LP during first deposit (initialize pool)
    #[account(
        mut
    )]
    pub payer_pool_lp: UncheckedAccount<'info>,

    #[account(
        mut
    )]
    /// Protocol fee token account for token A. Used to receive trading fee.
    pub protocol_token_a_fee: UncheckedAccount<'info>,

    /// Protocol fee token account for token B. Used to receive trading fee.
    #[account(
        mut
    )]
    pub protocol_token_b_fee: UncheckedAccount<'info>,

    /// Admin account. This account will be the admin of the pool, and the payer for PDA during initialize pool.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Rent account.
    pub rent: Sysvar<'info, Rent>,

    /// CHECK: LP mint metadata PDA. Metaplex do the checking.
    #[account(mut)]
    pub mint_metadata: UncheckedAccount<'info>,

    /// CHECK: Metadata program
    pub metadata_program: UncheckedAccount<'info>,

    /// CHECK: Vault program. The pool will deposit/withdraw liquidity from the vault.
    pub vault_program: UncheckedAccount<'info>,
    /// Token program.
    pub token_program: Program<'info, Token>,
    /// Associated token program.
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// System program.
    pub system_program: Program<'info, System>,

    pub dynamic_amm_program: UncheckedAccount<'info>,

    #[account(
        seeds = [BondingCurve::SEED_PREFIX, token_b_mint.to_account_info().key.as_ref()],
        bump,
    )]
    pub bonding_curve: Box<Account<'info, BondingCurve>>,
    #[account(
        mut
    )]
    pub lock_escrow: UncheckedAccount<'info>,
    #[account(
        mut
    )]
    pub lock_escrow_token_account: UncheckedAccount<'info>,
}