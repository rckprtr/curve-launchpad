use anchor_lang::prelude::*;



#[account]
#[derive(InitSpace)]
pub struct BondingCurve {
    pub virtual_sol_reserve: u64,
    pub virtual_token_reserve: u64,
    pub real_sol_reserve: u64,
    pub real_token_reserve: u64,
    pub token_total_supply: u64,
    pub complete: bool,
}

impl BondingCurve {
   pub const SEED_PREFIX: &'static [u8; 13] = b"bonding-curve";
   //calculate buy


   //calculate sell with fees

}