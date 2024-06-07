use anchor_lang::prelude::*;
use std::result::Result;

#[account]
#[derive(InitSpace)]
pub struct BondingCurve {
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub token_total_supply: u64,
    pub complete: bool,
}

impl BondingCurve {
    pub const SEED_PREFIX: &'static [u8; 13] = b"bonding-curve";
    //calculate buy

    //calculate sell with fees
    pub fn get_buy_price(&self, token_amount: u64) -> Result<u64, String> {
        // Calculate the product of virtual reserves
        // Calculate the product of virtual reserves
        let n = self
            .virtual_sol_reserves
            .checked_mul(self.virtual_token_reserves)
            .ok_or_else(|| "Overflow in multiplication".to_string())?;

        // Calculate the new virtual token reserves after the purchase
        let r = self
            .virtual_token_reserves
            .checked_sub(token_amount)
            .ok_or_else(|| "Insufficient tokens".to_string())?;

        // Calculate the new virtual sol reserves after the purchase
        let i = n
            .checked_div(r)
            .ok_or_else(|| "Division error".to_string())?
            .checked_add(1)
            .ok_or_else(|| "Overflow error".to_string())?;

        // Calculate the amount of SOL required
        let amount = i
            .checked_sub(self.virtual_sol_reserves)
            .ok_or_else(|| "Underflow error".to_string())?;

        // Return the required amount of SOL
        Ok(amount)
    }
}
