use std::fmt;
use crate::state::BondingCurve;

#[derive(Debug)]
pub struct AMM {
    virtual_sol_reserves: u64,
    virtual_token_reserves: u64,
    real_sol_reserves: u64,
    real_token_reserves: u64,
    inital_virtual_token_reserves: u64,
}

impl AMM {
    pub fn new(
        virtual_sol_reserves: u64,
        virtual_token_reserves: u64,
        real_sol_reserves: u64,
        real_token_reserves: u64,
        inital_virtual_token_reserves: u64,
    ) -> Self {
        AMM {
            virtual_sol_reserves,
            virtual_token_reserves,
            real_sol_reserves,
            real_token_reserves,
            inital_virtual_token_reserves
        }
    }

    pub fn get_buy_price(&self, tokens: u64) -> u64 {
        if tokens <= 0 {
            return 0;
        }
    
        let product_of_reserves = self.virtual_sol_reserves * self.virtual_token_reserves;
        let new_virtual_token_reserves = self.virtual_token_reserves - tokens;
        let new_virtual_sol_reserves = product_of_reserves / new_virtual_token_reserves + 1;
        let amount_needed = new_virtual_sol_reserves.saturating_sub(self.virtual_sol_reserves);
    
        if amount_needed > 0 { amount_needed } else { 0 }
    }

    pub fn apply_buy(&mut self, token_amount: u64) {
        let sol_amount = self.get_buy_price(token_amount);

        self.virtual_token_reserves = self.virtual_token_reserves.saturating_sub(token_amount);
        self.real_token_reserves = self.real_token_reserves.saturating_sub(token_amount);

        self.virtual_sol_reserves = self.virtual_sol_reserves.saturating_add(sol_amount);
        self.real_sol_reserves = self.real_sol_reserves.saturating_add(sol_amount);
    }

    pub fn apply_sell(&mut self, token_amount: u64) {
        self.virtual_token_reserves = self.virtual_token_reserves.saturating_add(token_amount);
        self.real_token_reserves = self.real_token_reserves.saturating_add(token_amount);

        let sell_price = self.get_sell_price(token_amount);

        self.virtual_sol_reserves = self.virtual_sol_reserves.saturating_sub(sell_price);
        self.real_sol_reserves = self.real_sol_reserves.saturating_sub(sell_price);
    }

    pub fn get_sell_price(&self, tokens: u64) -> u64 {
        if tokens <= 0 {
            return 0;
        }

        let scaling_factor = self.inital_virtual_token_reserves;

        let token_sell_proportion = (tokens * scaling_factor) / self.virtual_token_reserves;
        let sol_received = (self.virtual_sol_reserves * token_sell_proportion) / scaling_factor;

        if sol_received < self.real_sol_reserves {
            sol_received
        } else {
            self.real_sol_reserves
        }
    }
}

impl fmt::Display for AMM {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "AMM {{ virtual_sol_reserves: {}, virtual_token_reserves: {}, real_sol_reserves: {}, real_token_reserves: {}, inital_virtual_token_reserves: {} }}",
            self.virtual_sol_reserves, self.virtual_token_reserves, self.real_sol_reserves, self.real_token_reserves, self.inital_virtual_token_reserves
        )
    }
}
