import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { CurveSocial } from "../target/types/curve_social";
import { LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import { fundAccountSOL, getTxDetails } from "./util";
import {
  getAssociatedTokenAddress,
  getAssociatedTokenAddressSync,
  getMint,
  getOrCreateAssociatedTokenAccount,
  getTokenMetadata,
} from "@solana/spl-token";
import { BN } from "bn.js";
import { assert } from "chai";
import { Metaplex, token } from "@metaplex-foundation/js";

const GLOBAL_SEED = "global";
const METADATA_SEED = "metadata";
const MINT_AUTHORITY_SEED = "mint-authority";
const BONDING_CURVE_SEED = "bonding-curve";
const TOKEN_METADATA_PROGRAM_ID = new PublicKey(
  "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
);

import IDL from "../target/idl/curve_social.json";

describe("curve-social", () => {
  const DEFAULT_DECIMALS = 6n;
  const DEFAULT_TOKEN_BALANCE =
    1_000_000_000n * BigInt(10 ** Number(DEFAULT_DECIMALS));

  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  let newProgram = new Program<CurveSocial>(IDL as any);

  //newProgram.account.

  const program = anchor.workspace.CurveSocial as Program<CurveSocial>;

  const connection = provider.connection;
  const authority = anchor.web3.Keypair.generate();
  const tokenCreator = anchor.web3.Keypair.generate();

  const mint = anchor.web3.Keypair.generate();

  const [globalPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from(GLOBAL_SEED)],
    program.programId
  );

  const simpleBuy = async (
    user: anchor.web3.Keypair,
    tokenAmount: bigint,
    maxSolAmount: bigint
  ) => {
    const [bondingCurvePDA] = PublicKey.findProgramAddressSync(
      [Buffer.from(BONDING_CURVE_SEED), mint.publicKey.toBuffer()],
      program.programId
    );

    const bondingCurveTokenAccount = await getAssociatedTokenAddress(
      mint.publicKey,
      bondingCurvePDA,
      true
    );

    const userTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      user,
      mint.publicKey,
      user.publicKey
    );

    return program.methods
      .buy(new BN(tokenAmount.toString()), new BN(maxSolAmount.toString()))
      .accounts({
        user: user.publicKey,
        mint: mint.publicKey,
        bondingCurveTokenAccount: bondingCurveTokenAccount,
        userTokenAccount: userTokenAccount.address,
      })
      .signers([user])
      .rpc();
  };

  const simpleSell = async (
    user: anchor.web3.Keypair,
    tokenAmount: bigint,
    minSolAmount: bigint
  ) => {
    const [bondingCurvePDA] = PublicKey.findProgramAddressSync(
      [Buffer.from(BONDING_CURVE_SEED), mint.publicKey.toBuffer()],
      program.programId
    );

    const bondingCurveTokenAccount = await getAssociatedTokenAddress(
      mint.publicKey,
      bondingCurvePDA,
      true
    );

    const userTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      user,
      mint.publicKey,
      user.publicKey
    );

    return program.methods
      .sell(new BN(tokenAmount.toString()), new BN(minSolAmount.toString()))
      .accounts({
        user: user.publicKey,
        mint: mint.publicKey,
        bondingCurveTokenAccount: bondingCurveTokenAccount,
        userTokenAccount: userTokenAccount.address,
      })
      .signers([user])
      .rpc();
  };

  before(async () => {
    await fundAccountSOL(connection, authority.publicKey, 5 * LAMPORTS_PER_SOL);

    await fundAccountSOL(
      connection,
      tokenCreator.publicKey,
      200 * LAMPORTS_PER_SOL
    );
  });

  it("Is initialized!", async () => {
    await program.methods
      .initialize()
      .accounts({
        authority: authority.publicKey,
      })
      .signers([authority])
      .rpc();

    let global = await program.account.global.fetch(globalPDA);

    assert.equal(global.authority.toBase58(), authority.publicKey.toBase58());
    assert.equal(global.initialized, true);
  });

  it("can mint a token", async () => {
    const [metadataPDA] = PublicKey.findProgramAddressSync(
      [
        Buffer.from(METADATA_SEED),
        TOKEN_METADATA_PROGRAM_ID.toBuffer(),
        mint.publicKey.toBuffer(),
      ],
      TOKEN_METADATA_PROGRAM_ID
    );

    const [bondingCurvePDA] = PublicKey.findProgramAddressSync(
      [Buffer.from(BONDING_CURVE_SEED), mint.publicKey.toBuffer()],
      program.programId
    );

    const bondingCurveTokenAccount = await getAssociatedTokenAddress(
      mint.publicKey,
      bondingCurvePDA,
      true
    );

    let name = "test";
    let symbol = "tst";
    let uri = "https://www.test.com";

    await program.methods
      .create(name, symbol, uri)
      .accounts({
        mint: mint.publicKey,
        creator: tokenCreator.publicKey,
        bondingCurveTokenAccount: bondingCurveTokenAccount,
        metadata: metadataPDA,
        program: program.programId,
      })
      .signers([mint, tokenCreator])
      .rpc();

    const tokenAmount = await connection.getTokenAccountBalance(
      bondingCurveTokenAccount
    );
    assert.equal(tokenAmount.value.amount, DEFAULT_TOKEN_BALANCE.toString());

    const createdMint = await getMint(connection, mint.publicKey);
    assert.equal(createdMint.isInitialized, true);
    assert.equal(createdMint.decimals, Number(DEFAULT_DECIMALS));
    assert.equal(createdMint.supply, DEFAULT_TOKEN_BALANCE);
    assert.equal(createdMint.mintAuthority, null);

    const metaplex = Metaplex.make(connection);
    const token = await metaplex
      .nfts()
      .findByMint({ mintAddress: mint.publicKey });
    assert.equal(token.name, name);
    assert.equal(token.symbol, symbol);
    assert.equal(token.uri, uri);
  });

  it("can buy a token", async () => {
    const [bondingCurvePDA] = PublicKey.findProgramAddressSync(
      [Buffer.from(BONDING_CURVE_SEED), mint.publicKey.toBuffer()],
      program.programId
    );

    const bondingCurveTokenAccount = await getAssociatedTokenAddressSync(
      mint.publicKey,
      bondingCurvePDA,
      true
    );

    const userTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      tokenCreator,
      mint.publicKey,
      tokenCreator.publicKey
    );

    let buyMaxSOLAmount = new BN(85 * LAMPORTS_PER_SOL);
    let buyTokenAmount = new BN((DEFAULT_TOKEN_BALANCE / 100n).toString());

    await program.methods
      .buy(new BN(buyTokenAmount), new BN(buyMaxSOLAmount))
      .accounts({
        user: tokenCreator.publicKey,
        mint: mint.publicKey,
        bondingCurveTokenAccount: bondingCurveTokenAccount,
        userTokenAccount: userTokenAccount.address,
      })
      .signers([tokenCreator])
      .rpc();
  });

  it("can sell a token", async () => {
    const [bondingCurvePDA] = PublicKey.findProgramAddressSync(
      [Buffer.from(BONDING_CURVE_SEED), mint.publicKey.toBuffer()],
      program.programId
    );

    const bondingCurveTokenAccount = await getAssociatedTokenAddress(
      mint.publicKey,
      bondingCurvePDA,
      true
    );

    const userTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      tokenCreator,
      mint.publicKey,
      tokenCreator.publicKey
    );

    await program.methods
      .sell(new BN(10000000), new BN(0))
      .accounts({
        user: tokenCreator.publicKey,
        mint: mint.publicKey,
        bondingCurveTokenAccount: bondingCurveTokenAccount,
        userTokenAccount: userTokenAccount.address,
      })
      .signers([tokenCreator])
      .rpc();

      await simpleSell(tokenCreator, 10000000n, 0n);
  });

  it("can't buy a token, not enough SOL", async () => {
    const notEnoughSolUser = anchor.web3.Keypair.generate();

    await fundAccountSOL(
      connection,
      notEnoughSolUser.publicKey,
      0.021 * LAMPORTS_PER_SOL
    );

    let errorCode = "";
    try {
      await simpleBuy(
        notEnoughSolUser,
        5_000_000_000_000n,
        BigInt(5 * LAMPORTS_PER_SOL)
      );
    } catch (err) {
      if (err instanceof anchor.AnchorError) {
        errorCode = err.error.errorCode.code;
      }
    }
    assert.equal(errorCode, "InsufficientSOL");
  });

  it("can't buy a token, exceed max sol", async () => {
    let errorCode = "";
    try {
      await simpleBuy(tokenCreator, DEFAULT_TOKEN_BALANCE / 100n, 1n);
    } catch (err) {
      if (err instanceof anchor.AnchorError) {
        errorCode = err.error.errorCode.code;
      }
    }
    assert.equal(errorCode, "MaxSOLCostExceeded");
  });

  it("can't buy 0 tokens", async () => {
    let errorCode = "";
    try {
      await simpleBuy(tokenCreator, 0n, 1n);
    } catch (err) {
      if (err instanceof anchor.AnchorError) {
        errorCode = err.error.errorCode.code;
      }
    }
    assert.equal(errorCode, "MinBuy");
  });

  it("can't sell a token, not enough tokens", async () => {
    let errorCode = "";
    try {
      await simpleSell(tokenCreator, DEFAULT_TOKEN_BALANCE, 0n);
    } catch (err) {
      if (err instanceof anchor.AnchorError) {
        errorCode = err.error.errorCode.code;
      }
    }
    assert.equal(errorCode, "InsufficientTokens");
  });

  it("can't sell 0 tokens", async () => {
    let errorCode = "";
    try {
      await simpleSell(tokenCreator, 0n, 0n);
    } catch (err) {
      if (err instanceof anchor.AnchorError) {
        errorCode = err.error.errorCode.code;
      }
    }
    assert.equal(errorCode, "MinSell");
  });

  it("can't sell a token, exceed mint sol sell", async () => {
    let errorCode = "";
    try {
      await simpleSell(tokenCreator, 1n, DEFAULT_TOKEN_BALANCE);
    } catch (err) {
      if (err instanceof anchor.AnchorError) {
        errorCode = err.error.errorCode.code;
      }
    }
    assert.equal(errorCode, "MinSOLOutputExceeded");
  });

  it("can set params", async () => {
    await program.methods
      .setParams(
        TOKEN_METADATA_PROGRAM_ID,
        new BN(1000),
        new BN(2000),
        new BN(3000),
        new BN(4000),
        new BN(100)
      )
      .accounts({
        user: authority.publicKey,
      })
      .signers([authority])
      .rpc();

    let global = await program.account.global.fetch(globalPDA);

    assert.equal(
      global.feeRecipient.toBase58(),
      TOKEN_METADATA_PROGRAM_ID.toBase58()
    );
    assert.equal(
      global.initialVirtualTokenReserves.toString(),
      new BN(1000).toString()
    );
    assert.equal(
      global.initialVirtualSolReserves.toString(),
      new BN(2000).toString()
    );
    assert.equal(
      global.initialRealTokenReserves.toString(),
      new BN(3000).toString()
    );
    assert.equal(global.initialTokenSupply.toString(), new BN(4000).toString());
    assert.equal(global.feeBasisPoints.toString(), new BN(100).toString());
  });
});

//TODO: Tests
// test buy whole curve
// test sell whole curve

// test set params errors
// set params error: not authority
