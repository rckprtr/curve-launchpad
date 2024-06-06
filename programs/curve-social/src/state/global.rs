use anchor_lang::prelude::*;



#[account]
#[derive(InitSpace)]
pub struct Global {
    pub authority: Pubkey,
    pub initialized: bool,
    pub initial_token_supply: u64,
}

impl Global {
   pub const SEED_PREFIX: &'static [u8; 6] = b"global";
}