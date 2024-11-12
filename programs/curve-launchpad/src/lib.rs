use anchor_lang::prelude::*;

pub mod amm;
pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;
pub mod utils;

use instructions::*;
use state::*;

declare_id!("Cpm3iVenngWyh3YQUXtjR1PudXBXfJJqLhxMGrDiVSkW");

#[program]
pub mod curve_launchpad {

    use super::*;

    // Admin functions
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::handle(ctx)
    }

    pub fn set_params(
        ctx: Context<SetParams>,
        initial_virtual_token_reserves: Option<u64>,
        fee_basis_points: Option<u64>,
    ) -> Result<()> {
        set_params::handle(ctx, initial_virtual_token_reserves, fee_basis_points)
    }

    // Creator functions
    pub fn create(ctx: Context<Create>, param: CreateLaunchpadParam) -> Result<()> {
        create::handle(ctx, param)
    }

    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        withdraw::handle(ctx)
    }

    pub fn migrate(ctx: Context<Migrate>) -> Result<()> {
        migrate::handle(ctx)
    }

    // User functions
    pub fn buy(ctx: Context<Buy>, token_amount: u64, max_sol_cost: u64) -> Result<()> {
        buy::handle(ctx, token_amount, max_sol_cost)
    }

    pub fn sell(ctx: Context<Sell>, token_amount: u64, min_sol_output: u64) -> Result<()> {
        sell::handle(ctx, token_amount, min_sol_output)
    }
}
