use anchor_lang::prelude::*;

use instructions::*;

pub mod instructions;
pub mod state;

declare_id!("GVapdHoG4xjJZpvGPd8EUBaUJKR5Txpf6VHnVwBVCY69");

#[program]
pub mod curve_social {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::initialize(ctx)
    }
}