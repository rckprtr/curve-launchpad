use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Global {
    pub authority: Pubkey,
    pub fee_recipient: Pubkey,
    pub fee_basis_points: u64,
    pub withdraw_authority: Pubkey,
    pub initial_virtual_token_reserves: u64,
}

impl Global {
    pub const SEED_PREFIX: &'static [u8; 6] = b"global";
}
