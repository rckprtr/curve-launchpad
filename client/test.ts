

import { CurveLaunchpad } from "../target/types/curve_launchpad";
import * as IDL from "../target/idl/curve_launchpad.json";

import { Connection, Keypair, Commitment, PublicKey, ComputeBudgetProgram } from "@solana/web3.js";
import { AnchorProvider, Program, Wallet } from "@coral-xyz/anchor";
import BN from "bn.js";

import AUTHORITY_KEY from "./keypairs/authority.json";
import FEE_RECIPIENT_KEY from "./keypairs/feeRecipient.json";
import WITHDRAW_AUTH_KEY from "./keypairs/withdrawAuthority.json";
import CREATOR_KEY from "./keypairs/creator.json";
import USER_KEY from "./keypairs/user.json";
import { getAssociatedTokenAddressSync, getOrCreateAssociatedTokenAccount, NATIVE_MINT } from "@solana/spl-token";
import { AMM } from "./amm";
import { calculateFee } from "./utils";
import { ammProgramId, createPoolFee, getAmmConfigAddress, getAuthAddress, getOrcleAccountAddress, getPoolAddress, getPoolLpMintAddress, getPoolVaultAddress } from "./raydium";

const RPC_URL = "https://api.devnet.solana.com";

const GLOBAL_SEED = "global";
const BONDING_CURVE_SEED = "bonding-curve";

const DEFAULT_DECIMALS = 6n;
const DEFAULT_TOKEN_RESERVES = 100000000000000n;
const DEFAULT_VIRTUAL_SOL_RESERVE = 30000000000n;
const DEFUALT_VIRTUAL_TOKEN_RESERVE = 1900000000000000n;
const DEFUALT_INITIAL_VIRTUAL_TOKEN_RESERVE = 1000000000000000n;
const DEFAULT_FEE_BASIS_POINTS = 50n;

const sendOptions = {
    commitment: 'confirmed' as Commitment,
    preflightCommitment: 'confirmed' as Commitment,
    maxRetries: 10,
};

const connection = new Connection(RPC_URL);
const authority = Keypair.fromSecretKey(new Uint8Array(AUTHORITY_KEY));
const feeRecipient = Keypair.fromSecretKey(new Uint8Array(FEE_RECIPIENT_KEY));
const withdrawAuthority = Keypair.fromSecretKey(new Uint8Array(WITHDRAW_AUTH_KEY));
const creator = Keypair.fromSecretKey(new Uint8Array(CREATOR_KEY));
const user = Keypair.fromSecretKey(new Uint8Array(USER_KEY));

const wallet = new Wallet(authority);
const provider = new AnchorProvider(connection, wallet);
const program = new Program<CurveLaunchpad>(IDL as CurveLaunchpad, provider);

(async () => {

    // await initialize();
    // await setParams();

    // const mint = Keypair.generate();
    // await create("Test #1", "T1", "https://test.com", creator, mint);

    const mint = new PublicKey("41XAvTgsKfJ1bpwDzJwXL9ouvfo2eQnNF6GyK8DUEeHn");

    // for (let i = 0; i < 12; i++) {

    //     let currentAMM = await getAmmFromBondingCurve(mint);

    //     let tokenAmount = 1_000_000_000n;

    //     if (i % 2 == 1) {
    //         let maxSolAmount = currentAMM.getBuyPrice(tokenAmount);
    //         let fee = calculateFee(maxSolAmount, Number(DEFAULT_FEE_BASIS_POINTS));
    //         maxSolAmount = maxSolAmount + fee;

    //         await buy(user, mint, new BN(tokenAmount.toString()), new BN(maxSolAmount.toString()));
    //     }
    //     else {
    //         let minSolAmount = currentAMM.getSellPrice(tokenAmount);
    //         let fee = calculateFee(minSolAmount, Number(DEFAULT_FEE_BASIS_POINTS));
    //         minSolAmount = minSolAmount - fee;

    //         await sell(user, mint, new BN(tokenAmount.toString()), new BN(minSolAmount.toString()));
    //     }
    // }

    // {
    //     // buy all
    //     let currentAMM = await getAmmFromBondingCurve(mint);
    //     let tokenAmount = currentAMM.realTokenReserves;
    //     let maxSolAmount = currentAMM.getBuyPrice(tokenAmount);
    //     let fee = calculateFee(maxSolAmount, Number(DEFAULT_FEE_BASIS_POINTS));
    //     maxSolAmount = maxSolAmount + fee;

    //     await buy(user, mint, new BN(tokenAmount.toString()), new BN(maxSolAmount.toString()));
    // }

    // await withdraw(mint);

    await migrate(withdrawAuthority, mint);

})()

async function initialize() {
    const tx = await program.methods
        .initialize()
        .accounts({
            authority: authority.publicKey,
        })
        .signers([authority])
        .rpc(sendOptions);
    console.log(`Initialize:`, tx);
}

async function setParams() {
    const tx = await program.methods
        .setParams(
            feeRecipient.publicKey,
            withdrawAuthority.publicKey,
            new BN(DEFUALT_VIRTUAL_TOKEN_RESERVE.toString()),
            new BN(DEFAULT_VIRTUAL_SOL_RESERVE.toString()),
            new BN(DEFAULT_TOKEN_RESERVES.toString()),
            new BN(DEFUALT_INITIAL_VIRTUAL_TOKEN_RESERVE.toString()),
            new BN(DEFAULT_FEE_BASIS_POINTS.toString())
        )
        .accounts({
            user: authority.publicKey,
        })
        .signers([authority])
        .rpc(sendOptions);
    console.log(`Set params:`, tx);
}

async function create(name: string, symbol: string, uri: string, creator: Keypair, mint: Keypair) {

    const tx = await program.methods
        .create(name, symbol, uri)
        .accounts({
            mint: mint.publicKey,
            creator: creator.publicKey,
            program: program.programId,
        })
        .preInstructions([
            ComputeBudgetProgram.setComputeUnitLimit({
                units: 200_000
            }),
        ])
        .signers([creator, mint])
        .rpc(sendOptions);
    console.log(`Mint:`, mint.publicKey.toBase58());
    console.log(`Create token:`, tx);
}

async function buy(user: Keypair, mint: PublicKey, tokenAmount: BN, maxSolAmount: BN) {

    await getOrCreateAssociatedTokenAccount(
        connection,
        user,
        mint,
        user.publicKey,
        true,
        'confirmed'
    );

    let tx = await program.methods
        .buy(tokenAmount, maxSolAmount)
        .accounts({
            user: user.publicKey,
            mint: mint,
            feeRecipient: feeRecipient.publicKey,
            program: program.programId,
        })
        .signers([user])
        .rpc(sendOptions);
    console.log(`Buy token:`, tx);
}

async function sell(user: Keypair, mint: PublicKey, tokenAmount: BN, minSolAmount: BN) {

    await getOrCreateAssociatedTokenAccount(
        connection,
        user,
        mint,
        user.publicKey,
        true,
        'confirmed'
    );

    let tx = await program.methods
        .sell(tokenAmount, minSolAmount)
        .accounts({
            user: user.publicKey,
            mint: mint,
            feeRecipient: feeRecipient.publicKey,
            program: program.programId,
        })
        .signers([user])
        .rpc(sendOptions);
    console.log(`Sell token:`, tx);
}

async function withdraw(mint: PublicKey) {

    let tx = await program.methods
        .withdraw()
        .accounts({
            user: withdrawAuthority.publicKey,
            mint,
        })
        .signers([withdrawAuthority])
        .rpc(sendOptions);
    console.log(`Withdraw token:`, tx);
}

async function migrate(creator: Keypair, mint: PublicKey) {

    const ammConfig = getAmmConfigAddress(0, ammProgramId)[0];

    const wsolMint = NATIVE_MINT;
    const tokenMint = mint;

    const creatorTokenAccount = getAssociatedTokenAddressSync(tokenMint, creator.publicKey);
    const poolState = getPoolAddress(ammConfig, wsolMint, tokenMint, ammProgramId)[0];
    const ammAuthority = getAuthAddress(ammProgramId)[0];
    const token0Vault = getPoolVaultAddress(poolState, wsolMint, ammProgramId)[0];
    const token1Vault = getPoolVaultAddress(poolState, tokenMint, ammProgramId)[0];
    const observationState = getOrcleAccountAddress(poolState, ammProgramId)[0];
    const lpMint = getPoolLpMintAddress(poolState, ammProgramId)[0];
    const creatorLpToken = getAssociatedTokenAddressSync(lpMint, creator.publicKey);

    let tx = await program.methods
        .migrate()
        .accounts({
            creator: creator.publicKey,
            ammConfig,
            authority: ammAuthority,
            poolState,
            tokenMint,
            token0Vault,
            token1Vault,
            lpMint,
            creatorTokenAccount,
            createPoolFee,
            creatorLpToken,
            observationState,
            cpSwapProgram: ammProgramId,
        })
        .signers([creator])
        .preInstructions([
            ComputeBudgetProgram.setComputeUnitLimit({
                units: 1_000_000
            })
        ])
        .rpc(sendOptions);
    console.log(`Migrate token:`, tx);
}

async function getAmmFromBondingCurve(mint: PublicKey) {
    const bondingCurvePDA = PublicKey.findProgramAddressSync(
        [Buffer.from(BONDING_CURVE_SEED), mint.toBuffer()],
        program.programId
    )[0];

    let bondingCurveAccount = await program.account.bondingCurve.fetch(
        bondingCurvePDA, 'confirmed'
    );

    // console.log(`Price:`, bondingCurveAccount.virtualSolReserves.div(bondingCurveAccount.virtualTokenReserves).toNumber());

    return new AMM(
        BigInt(bondingCurveAccount.virtualSolReserves.toString()),
        BigInt(bondingCurveAccount.virtualTokenReserves.toString()),
        BigInt(bondingCurveAccount.realSolReserves.toString()),
        BigInt(bondingCurveAccount.realTokenReserves.toString()),
        BigInt(DEFUALT_INITIAL_VIRTUAL_TOKEN_RESERVE.toString()),
    );
};

async function delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}