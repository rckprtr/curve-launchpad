use anchor_lang::prelude::*;

declare_id!("GVapdHoG4xjJZpvGPd8EUBaUJKR5Txpf6VHnVwBVCY69");

#[program]
pub mod curve_social {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
