use anchor_lang::prelude::*;
use instructions::*;
use anchor_lang::solana_program::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
};

pub mod instructions;
pub mod state;
pub mod amm;

use anchor_lang::{
    prelude::*,
    system_program::{create_account, CreateAccount},
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};
use spl_tlv_account_resolution::{
    account::ExtraAccountMeta, seeds::Seed, state::ExtraAccountMetaList,
};
use spl_transfer_hook_interface::instruction::{ExecuteInstruction, TransferHookInstruction};

#[derive(Accounts)]
pub struct InitializeExtraAccountMetaList<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    /// CHECK: ExtraAccountMetaList Account, must use these seeds
    #[account(
        mut,
        seeds = [b"extra-account-metas", mint.key().as_ref()], 
        bump
    )]
    pub extra_account_meta_list: AccountInfo<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub wsol_mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

// Order of accounts matters for this struct.
// The first 4 accounts are the accounts required for token transfer (source, mint, destination, owner)
// Remaining accounts are the extra accounts required from the ExtraAccountMetaList account
// These accounts are provided via CPI to this program from the token2022 program
#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(
        token::mint = mint, 
        token::authority = owner,
    )]
    pub source_token: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        token::mint = mint,
    )]
    pub destination_token: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: source token account owner, can be SystemAccount or PDA owned by another program
    pub owner: UncheckedAccount<'info>,
    /// CHECK: ExtraAccountMetaList Account,
    #[account(
        seeds = [b"extra-account-metas", mint.key().as_ref()], 
        bump
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"delegate"], 
        bump
    )]
    pub delegate: SystemAccount<'info>,
    #[account(mut)]
    pub user: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

declare_id!("Uz9Fn2tCcMJ2VmZE4tSHwh61pKVRepJVoHDXQDRhUej");

#[program]
pub mod curve_launchpad {


    use super::*;

    pub fn initialize_extra_account_meta_list(
        ctx: Context<InitializeExtraAccountMetaList>,
    ) -> Result<()> {
        // index 0-3 are the accounts required for token transfer (source, mint, destination, owner)
        // index 4 is address of ExtraAccountMetaList account
        // The `addExtraAccountsToInstruction` JS helper function resolving incorrectly
        let account_metas = vec![
            // index 5, wrapped SOL mint
            ExtraAccountMeta::new_with_pubkey(&ctx.accounts.wsol_mint.key(), false, false)?,
            // index 6, token program
            ExtraAccountMeta::new_with_pubkey(&ctx.accounts.token_program.key(), false, false)?,
            // index 7, associated token program
            ExtraAccountMeta::new_with_pubkey(
                &ctx.accounts.associated_token_program.key(),
                false,
                false,
            )?,
            // index 8, delegate PDA
            ExtraAccountMeta::new_with_seeds(
                &[Seed::Literal {
                    bytes: "delegate".as_bytes().to_vec(),
                }],
                false, // is_signer
                true,  // is_writable
            )?,
            // index 9, delegate wrapped SOL token account
            ExtraAccountMeta::new_external_pda_with_seeds(
                7, // associated token program index
                &[
                    Seed::AccountKey { index: 8 }, // owner index (delegate PDA)
                    Seed::AccountKey { index: 6 }, // token program index
                    Seed::AccountKey { index: 5 }, // wsol mint index
                ],
                false, // is_signer
                true,  // is_writable
            )?,
            // index 10, sender wrapped SOL token account
            ExtraAccountMeta::new_external_pda_with_seeds(
                7, // associated token program index
                &[
                    Seed::AccountKey { index: 3 }, // owner index
                    Seed::AccountKey { index: 6 }, // token program index
                    Seed::AccountKey { index: 5 }, // wsol mint index
                ],
                false, // is_signer
                true,  // is_writable
            )?
        ];

        // calculate account size
        let account_size = ExtraAccountMetaList::size_of(account_metas.len())? as u64;
        // calculate minimum required lamports
        let lamports = Rent::get()?.minimum_balance(account_size as usize);

        let mint = ctx.accounts.mint.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"extra-account-metas",
            &mint.as_ref(),
            &[ctx.bumps.extra_account_meta_list],
        ]];

        // create ExtraAccountMetaList account
        create_account(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                CreateAccount {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.extra_account_meta_list.to_account_info(),
                },
            )
            .with_signer(signer_seeds),
            lamports,
            account_size,
            ctx.program_id,
        )?;

        // initialize ExtraAccountMetaList account with extra accounts
        ExtraAccountMetaList::init::<ExecuteInstruction>(
            &mut ctx.accounts.extra_account_meta_list.try_borrow_mut_data()?,
            &account_metas,
        )?;

        Ok(())
    }
    #[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
    pub struct UserTransferData {
        pub last_transfer_timestamp: i64,
        pub transfers_in_last_hour: u8,
        pub hour_start_timestamp: i64,
    }
    impl UserTransferData {
        pub fn try_deserialize(data: &mut &[u8]) -> Result<Self> {
            let user_transfer_data: UserTransferData = AnchorDeserialize::deserialize(data)?;
            Ok(user_transfer_data)
        }
        pub fn get_or_init<'info>(
            user_account: &AccountInfo<'info>,
            system_program: &AccountInfo<'info>,
            signer_seeds: &[&[&[u8]]],
        ) -> Result<Self> {
            let account_data = user_account.try_borrow_data()?;
            if account_data.len() >= 8 + std::mem::size_of::<UserTransferData>() {
                // Account already initialized, deserialize and return
                Ok(Self::try_deserialize(&mut &account_data[8..])?)
            } else {
                // Account not initialized, create and return default
                let rent = Rent::get()?;
                let space = 8 + std::mem::size_of::<UserTransferData>();
                let lamports = rent.minimum_balance(space);

                create_account(
                    CpiContext::new(
                        system_program.clone(),
                        CreateAccount {
                            from: user_account.clone(),
                            to: user_account.clone(),
                        },
                    )
                    .with_signer(signer_seeds),
                    lamports,
                    space as u64,
                    &crate::ID,
                )?;

                Ok(Self::default())
            }
        }

        pub fn save<'info>(
            &self,
            user_account: &AccountInfo<'info>,
            system_program: &AccountInfo<'info>,
            signer_seeds: &[&[&[u8]]],
        ) -> Result<()> {
            let mut account_data = user_account.try_borrow_mut_data()?;
            if account_data.len() < 8 + std::mem::size_of::<UserTransferData>() {
                let rent = Rent::get()?;
                let space = 8 + std::mem::size_of::<UserTransferData>();
                let lamports = rent.minimum_balance(space);

                create_account(
                    CpiContext::new(
                        system_program.clone(),
                        CreateAccount {
                            from: user_account.clone(),
                            to: user_account.clone(),
                        },
                    )
                    .with_signer(signer_seeds),
                    lamports,
                    space as u64,
                    &crate::ID,
                )?;
            }

            self.serialize(&mut &mut account_data[8..])?;
            Ok(())
        }
    }
    pub fn transfer_hook(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
       let signer_seeds: &[&[&[u8]]] = &[&[b"delegate", &[ctx.bumps.delegate]]];
        // magick. you can be creative here but not too creative - cannot reenter the token22 program. You can however transfer ownership of a plain ol 'token program ATA, without the account info, i.e. wsol
        /* btw, a bonding curve is -so- much cooler if it wraps the sol in/out of LSTs. do it with luts and pray to the stack size gods
        cuz I like monye
        ---
 let signer_seeds: &[&[&[u8]]] = &[&[
                b"extra-account-metas",
                &ctx.accounts.mint.to_account_info().key.as_ref(),
                &[ctx.bumps.extra_account_meta_list],
            ]];
            invoke_signed(
                &spl_stake_pool::instruction::withdraw_sol(
                    &spl_stake_pool::id(),
                    &ctx.accounts.stake_pool.key(),
                    &ctx.accounts.stake_pool_withdraw_authority.key(),
                    &ctx.accounts.extra_account_meta_list.key(),
                    &ctx.accounts.pool_token_receiver_account.key(),
                    &ctx.accounts.reserve_stake_account.key(),
                    &ctx.accounts.delegate.key(),
                    &ctx.accounts.manager_fee_account.key(),
                    &ctx.accounts.pool_mint.key(),
                    &anchor_spl::token::ID,
                    ctx.accounts.pool_token_receiver_account.amount,
                ),
                &[
                    ctx.accounts.extra_account_meta_list.to_account_info(),
                    ctx.accounts.delegate.to_account_info(),
                    ctx.accounts.stake_pool.to_account_info(),
                    ctx.accounts.stake_pool_withdraw_authority.to_account_info(),
                    ctx.accounts.pool_token_receiver_account.to_account_info(),
                    ctx.accounts.stake_pool_program.to_account_info(),
                    ctx.accounts.reserve_stake_account.to_account_info(),
                    ctx.accounts.manager_fee_account.to_account_info(),
                    ctx.accounts.pool_mint.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                    ctx.accounts.token_program.to_account_info(),
                    ctx.accounts.clock.to_account_info(),
                    ctx.accounts.stake_history.to_account_info(),
                    ctx.accounts.stake_program.to_account_info(),
                    ctx.accounts.rent.to_account_info(),
                ],
                signer_seeds,
            )?;
            let signer_seeds: &[&[&[u8]]] = &[&[
                b"delegate",
                ctx.accounts.mint.to_account_info().key.as_ref(),
                &[ctx.bumps.delegate],
            ]];

            anchor_lang::system_program::transfer(
                CpiContext::new(
                    ctx.accounts.system_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.delegate.to_account_info(),
                        to: ctx.accounts.wsol.to_account_info(),
                    },
                )
                .with_signer(signer_seeds),
                ctx.accounts.delegate.lamports(),
            )?;
            // Wrap the SOL by calling sync_native
            invoke_signed(
                &spl_token::instruction::sync_native(&spl_token::id(), &ctx.accounts.wsol.key())?,
                &[
                    ctx.accounts.wsol.to_account_info(),
                    ctx.accounts.token_program.to_account_info(),
                ],
                signer_seeds,
            )?;
            // Change the account owner authority to the winner_winner
            invoke_signed(
                &spl_token::instruction::set_authority(
                    &spl_token::id(),
                    &ctx.accounts.wsol.key(),
                    Some(&ctx.accounts.source_token.owner.key()),
                    spl_token::instruction::AuthorityType::AccountOwner,
                    &ctx.accounts.delegate.key(),
                    &[],
                )?,
                &[
                    ctx.accounts.wsol.to_account_info(),
                    ctx.accounts.delegate.to_account_info(),
                    ctx.accounts.token_program.to_account_info(),
                ],
                signer_seeds,
            )?;
        ---
         */
        // Anti-bot measures
        let clock = Clock::get()?;
        let current_timestamp = clock.unix_timestamp;

        // Get or initialize the user's transfer data
        let mut user_transfer_data = UserTransferData::get_or_init(
            &ctx.accounts.user.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            signer_seeds,
        )?;

        // Check cooldown period (e.g., 5 minutes)
        const COOLDOWN_PERIOD: i64 = 300; // 5 minutes in seconds
        if current_timestamp - user_transfer_data.last_transfer_timestamp < COOLDOWN_PERIOD {
            return Err(CurveLaunchpadError::TransferCooldownNotMet.into());
        }

        // Check transfer limit (e.g., 5 transfers per hour)
        const MAX_TRANSFERS_PER_HOUR: u8 = 5;
        const ONE_HOUR: i64 = 3600; // 1 hour in seconds
        if user_transfer_data.transfers_in_last_hour >= MAX_TRANSFERS_PER_HOUR {
            if current_timestamp - user_transfer_data.hour_start_timestamp >= ONE_HOUR {
                // Reset the counter if an hour has passed
                user_transfer_data.transfers_in_last_hour = 0;
                user_transfer_data.hour_start_timestamp = current_timestamp;
            } else {
                return Err(CurveLaunchpadError::TransferLimitExceeded.into());
            }
        }

        // Update user's transfer data
        user_transfer_data.last_transfer_timestamp = current_timestamp;
        user_transfer_data.transfers_in_last_hour += 1;
        if user_transfer_data.transfers_in_last_hour == 1 {
            user_transfer_data.hour_start_timestamp = current_timestamp;
        }
        user_transfer_data.save(
            &ctx.accounts.user.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            signer_seeds,
        )?;

        // Proceed with the transfer if all checks pass
        Ok(())
    }

    // fallback instruction handler as workaround to anchor instruction discriminator check
    pub fn fallback<'info>(
        program_id: &Pubkey,
        accounts: &'info [AccountInfo<'info>],
        data: &[u8],
    ) -> Result<()> {
        let instruction = TransferHookInstruction::unpack(data)?;

        // match instruction discriminator to transfer hook interface execute instruction  
        // token2022 program CPIs this instruction on token transfer
        match instruction {
            TransferHookInstruction::Execute { amount } => {
                let amount_bytes = amount.to_le_bytes();

                // invoke custom transfer hook instruction on our program
                __private::__global::transfer_hook(program_id, accounts, &amount_bytes)
            }
            _ => return Err(ProgramError::InvalidInstructionData.into()),
        }
    }

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::initialize(ctx)
    }

    pub fn create(ctx: Context<Create>, name: String, symbol: String, uri: String) -> Result<()> {
        create::create(ctx, name, symbol, uri)
    }

    pub fn buy(ctx: Context<Buy>, token_amount: u64, max_sol_cost: u64) -> Result<()> {
        buy::buy(ctx, token_amount, max_sol_cost)
    }

    pub fn sell(ctx: Context<Sell>, token_amount: u64, min_sol_output: u64) -> Result<()> {
        sell::sell(ctx, token_amount, min_sol_output)
    }

    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        withdraw::withdraw(ctx)
    }

    pub fn set_params(
        ctx: Context<SetParams>,
        fee_recipient: Pubkey,
        withdraw_authority: Pubkey,
        initial_virtual_token_reserves: u64,
        initial_virtual_sol_reserves: u64,
        initial_real_token_reserves: u64,
        inital_token_supply: u64,
        fee_basis_points: u64,
    ) -> Result<()> {
        set_params::set_params(
            ctx,
            fee_recipient,
            withdraw_authority,
            initial_virtual_token_reserves,
            initial_virtual_sol_reserves,
            initial_real_token_reserves,
            inital_token_supply,
            fee_basis_points,
        )
    }
    /*
    pub fn initialize_group(ctx: Context<InitializeGroupContext>, 
        update_authority: Pubkey,
        max_size: u32) -> Result<()> {
// Assumes one has already created a mint for the group.

let accounts = ctx.accounts.to_account_infos();
let account_info_iter = &mut accounts.iter();
// Accounts expected by this instruction:
//
//   0. `[w]`   Group
//   1. `[]`    Mint
//   2. `[s]`   Mint authority
let group_info = next_account_info(account_info_iter)?;
let mint_info = next_account_info(account_info_iter)?;
let mint_authority_info = next_account_info(account_info_iter)?;

{
    // IMPORTANT: this example program is designed to work with any
    // program that implements the SPL token interface, so there is no
    // ownership check on the mint account.
    let mint_data = mint_info.try_borrow_data()?;
    let mint = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&mint_data)?;

    if !mint_authority_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature.into());
    }
    if mint.base.mint_authority.as_ref() != COption::Some(mint_authority_info.key) {
        return Err(ProgramError::MissingRequiredSignature.into());
    }
}

// Allocate a TLV entry for the space and write it in
let mut buffer = group_info.try_borrow_mut_data()?;
let mut state = TlvStateMut::unpack(&mut buffer)?;
let (group, _) = state.init_value::<TokenGroup>(false)?;
*group = TokenGroup::new(mint_info.key, spl_pod::optional_keys::OptionalNonZeroPubkey(update_authority), max_size.into());

Ok(())
        }

    pub fn update_group_max_size(ctx: Context<UpdateGroupMaxSizeContext>, max_size: u32) -> Result<()> {

        let accounts = ctx.accounts.to_account_infos();
        let account_info_iter = &mut accounts.iter();
        // Accounts expected by this instruction:
        //
        //   0. `[w]`   Group
        //   1. `[s]`   Update authority
        let group_info = next_account_info(account_info_iter)?;
        let update_authority_info = next_account_info(account_info_iter)?;
    
        let mut buffer = group_info.try_borrow_mut_data()?;
        let mut state = TlvStateMut::unpack(&mut buffer)?;
        let group = state.get_first_value_mut::<TokenGroup>()?;
    
        check_update_authority(update_authority_info, &group.update_authority)?;
    
        // Update the max size (zero-copy)
        group.update_max_size(max_size)?;
        Ok(())

    }

    pub fn update_group_authority(ctx: Context<UpdateGroupAuthorityContext>, new_authority: Pubkey) -> Result<()> {

        let accounts = ctx.accounts.to_account_infos();
        let account_info_iter = &mut accounts.iter();
        // Accounts expected by this instruction:
        //
        //   0. `[w]`   Group
        //   1. `[s]`   Current update authority
        let group_info = next_account_info(account_info_iter)?;
        let update_authority_info = next_account_info(account_info_iter)?;
    
        let mut buffer = group_info.try_borrow_mut_data()?;
        let mut state = TlvStateMut::unpack(&mut buffer)?;
        let group = state.get_first_value_mut::<TokenGroup>()?;
    
        check_update_authority(update_authority_info, &group.update_authority)?;
    
        // Update the authority (zero-copy)
        group.update_authority = spl_pod::optional_keys::OptionalNonZeroPubkey(new_authority);
    
        Ok(())
        }

    pub fn initialize_member(ctx: Context<InitializeMemberContext>) -> Result<()> {
// For this group, we are going to assume the group has been
    // initialized, and we're also assuming a mint has been created for the
    // member.
    // Group members in this example can have their own separate
    // metadata that differs from the metadata of the group, since
    // metadata is not involved here.
    let accounts = ctx.accounts.to_account_infos();
    let account_info_iter = &mut accounts.iter();

    // Accounts expected by this instruction:
    //
    //   0. `[w]`   Member
    //   1. `[]`    Member Mint
    //   2. `[s]`   Member Mint authority
    //   3. `[w]`   Group
    //   4. `[s]`   Group update authority
    let member_info = next_account_info(account_info_iter)?;
    let member_mint_info = next_account_info(account_info_iter)?;
    let member_mint_authority_info = next_account_info(account_info_iter)?;
    let group_info = next_account_info(account_info_iter)?;
    let group_update_authority_info = next_account_info(account_info_iter)?;

    // Mint checks on the member
    {
        // IMPORTANT: this example program is designed to work with any
        // program that implements the SPL token interface, so there is no
        // ownership check on the mint account.
        let member_mint_data = member_mint_info.try_borrow_data()?;
        let member_mint = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&member_mint_data)?;

        if !member_mint_authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature.into());
        }
        if member_mint.base.mint_authority.as_ref() != COption::Some(member_mint_authority_info.key)
        {
            return Err(ProgramError::MissingRequiredSignature.into());
        }
    }

    // Make sure the member account is not the same as the group account
    if member_info.key == group_info.key {
        return Err(ProgramError::MissingRequiredSignature.into());
    }

    // Increment the size of the group
    let mut buffer = group_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    let group = state.get_first_value_mut::<TokenGroup>()?;

    check_update_authority(group_update_authority_info, &group.update_authority)?;
    let member_number = group.increment_size()?;

    // Allocate a TLV entry for the space and write it in
    let mut buffer = member_info.try_borrow_mut_data()?;
    let mut state = TlvStateMut::unpack(&mut buffer)?;
    // Note if `allow_repetition: true` is instead used here, one can initialize
    // the same token as a member of multiple groups!
    let (member, _) = state.init_value::<TokenGroupMember>(false)?;
    *member = TokenGroupMember::new(member_mint_info.key, group_info.key, member_number);

        Ok(())
} */
}
/*
fn check_update_authority(
    update_authority_info: &AccountInfo,
    expected_update_authority: &OptionalNonZeroPubkey,
) -> Result<()> {
    if !update_authority_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature.into());
    }
    let update_authority = Option::<Pubkey>::from(*expected_update_authority)
        .ok_or(TokenGroupError::ImmutableGroup).unwrap();
    if update_authority != *update_authority_info.key {
        return Err(ProgramError::MissingRequiredSignature.into());
    }
    Ok(())
}
#[event_cpi]
#[derive(Accounts)]
pub struct InitializeGroupContext<'info> {
    
    pub group: Signer<'info>,
    pub mint: AccountInfo<'info>,
    pub mint_authority: AccountInfo<'info>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct UpdateGroupMaxSizeContext<'info> {
    pub group: AccountInfo<'info>,
    pub update_authority: Signer<'info>,
}


#[event_cpi]
#[derive(Accounts)]
pub struct UpdateGroupAuthorityContext<'info> {
    pub group: AccountInfo<'info>,
    pub update_authority: Signer<'info>,
}


#[event_cpi]
#[derive(Accounts)]
pub struct InitializeMemberContext<'info> {
    /// The member account to be initialized
    #[account(mut)]
    pub member: AccountInfo<'info>,
    /// CHECK:
    /// The mint of the member token
    pub member_mint: AccountInfo<'info>,
    /// The mint authority of the member token
    pub member_mint_authority: Signer<'info>,
    /// The group account that the member will be added to
    #[account(mut)]
    /// CHECK:
    pub group: AccountInfo<'info>,
    /// The update authority of the group
    pub group_update_authority: Signer<'info>,
} */