use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke, system_instruction},
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{
        self,
        spl_token::{instruction::sync_native, native_mint},
        Burn, Mint, Token, TokenAccount,
    },
};

#[derive(borsh::BorshSerialize, borsh::BorshDeserialize)]
pub struct InitializePayload {
    init_amount_0: u64,
    init_amount_1: u64,
    open_time: u64,
}

const INITIALIZE_DISCRIMINANT: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];

#[derive(Accounts)]
pub struct Migrate<'info> {
    /// Address paying to create the pool. Can be anyone
    #[account(mut)]
    pub creator: Signer<'info>,

    /// CHECK: Which config the pool belongs to.
    pub amm_config: UncheckedAccount<'info>,

    /// CHECK: pool vault and lp mint authority
    pub authority: UncheckedAccount<'info>,

    /// CHECK: Initialize an account to store the pool state, init by cp-swap
    #[account(mut)]
    pub pool_state: UncheckedAccount<'info>,

    /// WSOL mint
    #[account(
        address = native_mint::id()
    )]
    pub wsol_mint: Box<Account<'info, Mint>>,

    /// Token mint, the key must grater then token_0 mint.
    pub token_mint: Box<Account<'info, Mint>>,

    /// CHECK: pool lp mint, init by cp-swap
    #[account(mut)]
    pub lp_mint: UncheckedAccount<'info>,

    /// payer token0 account
    #[account(
        init_if_needed,
        payer = creator,
        associated_token::mint = wsol_mint,
        associated_token::authority = creator,
    )]
    pub creator_wsol_account: Box<Account<'info, TokenAccount>>,

    /// creator token1 account
    #[account(
        mut,
        token::mint = token_mint,
        token::authority = creator,
    )]
    pub creator_token_account: Box<Account<'info, TokenAccount>>,

    /// CHECK: creator lp ATA token account, init by cp-swap
    #[account(mut)]
    pub creator_lp_token: UncheckedAccount<'info>,

    /// CHECK: Token_0 vault for the pool, init by cp-swap
    #[account(mut)]
    pub token_0_vault: UncheckedAccount<'info>,

    /// CHECK: Token_1 vault for the pool, init by cp-swap
    #[account(mut)]
    pub token_1_vault: UncheckedAccount<'info>,

    /// create pool fee account
    #[account(mut)]
    pub create_pool_fee: Box<Account<'info, TokenAccount>>,

    /// CHECK: an account to store oracle observations, init by cp-swap
    #[account(mut)]
    pub observation_state: UncheckedAccount<'info>,

    /// CHECK: cp-swap programId
    pub cp_swap_program: UncheckedAccount<'info>,

    /// Program to create mint account and mint tokens
    pub token_program: Program<'info, Token>,
    /// Program to create an ATA for receiving position NFT
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// To create a new program account
    pub system_program: Program<'info, System>,
    /// Sysvar for program account
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle(ctx: Context<Migrate>) -> Result<()> {
    // Wrap SOL into WSOL from creator
    // let minimum_rent_fee = 200_000_000;
    let minimum_rent_fee = 1_100_000_000;

    let lamports = ctx
        .accounts
        .creator
        .lamports()
        .checked_sub(minimum_rent_fee)
        .unwrap();

    let transfer_ix = system_instruction::transfer(
        &ctx.accounts.creator.key(),
        &ctx.accounts.creator_wsol_account.key(),
        lamports,
    );
    invoke(
        &transfer_ix,
        &[
            ctx.accounts.creator.to_account_info(),
            ctx.accounts.creator_wsol_account.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    let wrap_ix = sync_native(&token::ID, &ctx.accounts.creator_wsol_account.key())?;
    invoke(
        &wrap_ix,
        &[ctx.accounts.creator_wsol_account.to_account_info()],
    )?;
    ctx.accounts.creator_wsol_account.reload()?;

    // Setup pool amounts & open_time
    let init_amount_0: u64 = ctx.accounts.creator_wsol_account.amount;
    let init_amount_1: u64 = ctx.accounts.creator_token_account.amount;
    let open_time: u64 = 0;

    // Prepare initalize instruction for Raydium CPMM
    let account_metas = vec![
        AccountMeta::new(ctx.accounts.creator.key(), true),
        AccountMeta::new_readonly(ctx.accounts.amm_config.key(), false),
        AccountMeta::new_readonly(ctx.accounts.authority.key(), false),
        AccountMeta::new(ctx.accounts.pool_state.key(), false),
        AccountMeta::new_readonly(ctx.accounts.wsol_mint.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_mint.key(), false),
        AccountMeta::new(ctx.accounts.lp_mint.key(), false),
        AccountMeta::new(ctx.accounts.creator_wsol_account.key(), false),
        AccountMeta::new(ctx.accounts.creator_token_account.key(), false),
        AccountMeta::new(ctx.accounts.creator_lp_token.key(), false),
        AccountMeta::new(ctx.accounts.token_0_vault.key(), false),
        AccountMeta::new(ctx.accounts.token_1_vault.key(), false),
        AccountMeta::new(ctx.accounts.create_pool_fee.key(), false),
        AccountMeta::new(ctx.accounts.observation_state.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.associated_token_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.rent.key(), false),
    ];

    let payload = InitializePayload {
        init_amount_0,
        init_amount_1,
        open_time,
    };
    let mut serialized_data = Vec::new();
    payload.serialize(&mut serialized_data)?;
    let mut data = INITIALIZE_DISCRIMINANT.to_vec();
    data.append(&mut serialized_data);

    let initialize_ix =
        Instruction::new_with_bytes(ctx.accounts.cp_swap_program.key(), &data, account_metas);
    invoke(
        &initialize_ix,
        &[
            ctx.accounts.creator.to_account_info(),
            ctx.accounts.amm_config.to_account_info(),
            ctx.accounts.authority.to_account_info(),
            ctx.accounts.pool_state.to_account_info(),
            ctx.accounts.wsol_mint.to_account_info(),
            ctx.accounts.token_mint.to_account_info(),
            ctx.accounts.lp_mint.to_account_info(),
            ctx.accounts.creator_wsol_account.to_account_info(),
            ctx.accounts.creator_token_account.to_account_info(),
            ctx.accounts.creator_lp_token.to_account_info(),
            ctx.accounts.token_0_vault.to_account_info(),
            ctx.accounts.token_1_vault.to_account_info(),
            ctx.accounts.create_pool_fee.to_account_info(),
            ctx.accounts.observation_state.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.associated_token_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.rent.to_account_info(),
        ],
    )?;

    // Burn LP
    let lp_token =
        TokenAccount::try_deserialize(&mut &ctx.accounts.creator_lp_token.data.borrow_mut()[..])?;
    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Burn {
            mint: ctx.accounts.lp_mint.to_account_info(),
            from: ctx.accounts.creator_lp_token.to_account_info(),
            authority: ctx.accounts.creator.to_account_info(),
        },
    );
    token::burn(cpi_ctx, lp_token.amount)?;

    Ok(())
}
