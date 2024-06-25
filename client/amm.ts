export type BuyResult = {
    token_amount: bigint;
    sol_amount: bigint;
};

export type SellResult = {
    token_amount: bigint;
    sol_amount: bigint;
};

export class AMM {
    constructor(
        public virtualSolReserves: bigint,
        public virtualTokenReserves: bigint,
        public realSolReserves: bigint,
        public realTokenReserves: bigint,
        public initialVirtualTokenReserves: bigint
    ) {}

    getBuyPrice(tokens: bigint): bigint {
        const productOfReserves = this.virtualSolReserves * this.virtualTokenReserves;
        const newVirtualTokenReserves = this.virtualTokenReserves - tokens;
        const newVirtualSolReserves = (productOfReserves / newVirtualTokenReserves) + 1n;
        const amountNeeded = newVirtualSolReserves - this.virtualSolReserves;

        return amountNeeded;
    }

    applyBuy(token_amount: bigint): BuyResult {
        const final_token_amount = token_amount > this.realTokenReserves ? this.realTokenReserves : token_amount;
        const sol_amount = this.getBuyPrice(final_token_amount);

        this.virtualTokenReserves = this.virtualTokenReserves - final_token_amount;
        this.realTokenReserves = this.realTokenReserves - final_token_amount;

        this.virtualSolReserves = this.virtualSolReserves + sol_amount;
        this.realSolReserves = this.realSolReserves + sol_amount;

        return {
            token_amount: final_token_amount,
            sol_amount: sol_amount
        }
    }

    applySell(token_amount: bigint): SellResult {
        this.virtualTokenReserves = this.virtualTokenReserves + token_amount;
        this.realTokenReserves = this.realTokenReserves + token_amount;

        const sell_price = this.getSellPrice(token_amount);

        this.virtualSolReserves = this.virtualSolReserves - sell_price;
        this.realSolReserves = this.realSolReserves - sell_price;

        return {
            token_amount: token_amount,
            sol_amount: sell_price
        }
    }

    getSellPrice(tokens: bigint): bigint {
        const scaling_factor = this.initialVirtualTokenReserves;
        const token_sell_proportion = (tokens * scaling_factor) / this.virtualTokenReserves;
        const sol_received = (this.virtualSolReserves * token_sell_proportion) / scaling_factor;
        return sol_received < this.realSolReserves ? sol_received : this.realSolReserves;
    }
}