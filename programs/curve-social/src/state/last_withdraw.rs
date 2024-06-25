use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct LastWithdraw {
    pub last_withdraw_timestamp: i64,
}

impl LastWithdraw {
    pub const SEED_PREFIX: &'static [u8; 13] = b"last-withdraw";
}