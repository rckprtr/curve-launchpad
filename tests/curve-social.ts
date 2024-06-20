import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { CurveSocial } from "../target/types/curve_social";
import {
  LAMPORTS_PER_SOL,
  PublicKey,
  SendTransactionError,
} from "@solana/web3.js";
import {
  ammFromBondingCurve,
  bigIntToSOL,
  fundAccountSOL,
  getAnchorError,
  getSPLBalance,
  getTxDetails,
  sendTransaction,
  toEvent,
} from "./util";
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
import fs from "fs";

const GLOBAL_SEED = "global";
const METADATA_SEED = "metadata";
const BONDING_CURVE_SEED = "bonding-curve";
const TOKEN_METADATA_PROGRAM_ID = new PublicKey(
  "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
);

import IDL from "../target/idl/curve_social.json";

//TODO: Unit test order is essential, need to refactor to make it so its not.

describe("curve-social", () => {
  const DEFAULT_DECIMALS = 6n;
  const DEFAULT_TOKEN_BALANCE =
    1_000_000_000n * BigInt(10 ** Number(DEFAULT_DECIMALS));
  const DEFAULT_INITIAL_TOKEN_RESERVES = 793_100_000_000_000n;
  const DEFAULT_INITIAL_VIRTUAL_SOL_RESERVE = 30_000_000_000n;
  const DEFUALT_INITIAL_VIRTUAL_TOKEN_RESERVE = 1_073_000_000_000_000n;
  const DEFAULT_FEE_BASIS_POINTS = 100n;

  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  let newProgram = new Program<CurveSocial>(IDL as any);

  //newProgram.account.

  const program = anchor.workspace.CurveSocial as Program<CurveSocial>;

  const connection = provider.connection;
  const authority = anchor.web3.Keypair.generate();
  const tokenCreator = anchor.web3.Keypair.generate();
  const feeRecipient = anchor.web3.Keypair.generate();

  const mint = anchor.web3.Keypair.generate();

  const [globalPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from(GLOBAL_SEED)],
    program.programId
  );

  const [bondingCurvePDA] = PublicKey.findProgramAddressSync(
    [Buffer.from(BONDING_CURVE_SEED), mint.publicKey.toBuffer()],
    program.programId
  );

  const getAmmFromBondingCurve = async () => {
    let bondingCurveAccount = await program.account.bondingCurve.fetch(
      bondingCurvePDA
    );
    return ammFromBondingCurve(
      bondingCurveAccount,
      DEFUALT_INITIAL_VIRTUAL_TOKEN_RESERVE
    );
  };

  const assertBondingCurve = (
    amm: any,
    bondingCurveAccount: any,
    complete: boolean = false
  ) => {
    assert.equal(
      bondingCurveAccount.virtualTokenReserves.toString(),
      amm.virtualTokenReserves.toString()
    );
    assert.equal(
      bondingCurveAccount.virtualSolReserves.toString(),
      amm.virtualSolReserves.toString()
    );
    assert.equal(
      bondingCurveAccount.realTokenReserves.toString(),
      amm.realTokenReserves.toString()
    );
    assert.equal(
      bondingCurveAccount.realSolReserves.toString(),
      amm.realSolReserves.toString()
    );
    assert.equal(
      bondingCurveAccount.tokenTotalSupply.toString(),
      DEFAULT_TOKEN_BALANCE.toString()
    );
    assert.equal(bondingCurveAccount.complete, complete);
  };

  const simpleBuy = async (
    user: anchor.web3.Keypair,
    tokenAmount: bigint,
    maxSolAmount: bigint
  ) => {
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

    let tx = await program.methods
      .buy(new BN(tokenAmount.toString()), new BN(maxSolAmount.toString()))
      .accounts({
        user: user.publicKey,
        mint: mint.publicKey,
        bondingCurveTokenAccount: bondingCurveTokenAccount,
        userTokenAccount: userTokenAccount.address,
        program: program.programId,
      })
      .transaction();

    let txResults = await sendTransaction(program, tx, [user], user.publicKey);

    return {
      tx: txResults,
      userTokenAccount,
      bondingCurveTokenAccount,
      bondingCurvePDA,
    };
  };

  const simpleSell = async (
    user: anchor.web3.Keypair,
    tokenAmount: bigint,
    minSolAmount: bigint
  ) => {
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

    let tx = await program.methods
      .sell(new BN(tokenAmount.toString()), new BN(minSolAmount.toString()))
      .accounts({
        user: user.publicKey,
        mint: mint.publicKey,
        bondingCurveTokenAccount: bondingCurveTokenAccount,
        userTokenAccount: userTokenAccount.address,
        program: program.programId,
      })
      .transaction();

    let txResults = await sendTransaction(program, tx, [user], user.publicKey);

    return {
      tx: txResults,
      userTokenAccount,
      bondingCurveTokenAccount,
      bondingCurvePDA,
    };
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

    await program.methods
      .setParams(
        feeRecipient.publicKey,
        new BN(DEFUALT_INITIAL_VIRTUAL_TOKEN_RESERVE.toString()),
        new BN(DEFAULT_INITIAL_VIRTUAL_SOL_RESERVE.toString()),
        new BN(DEFAULT_INITIAL_TOKEN_RESERVES.toString()),
        new BN(DEFAULT_TOKEN_BALANCE.toString()),
        new BN(DEFAULT_FEE_BASIS_POINTS.toString())
      )
      .accounts({
        user: authority.publicKey,
        program: program.programId,
      })
      .signers([authority])
      .rpc();
  });

  it("can mint a token", async () => {
    const bondingCurveTokenAccount = await getAssociatedTokenAddress(
      mint.publicKey,
      bondingCurvePDA,
      true
    );

    let name = "test";
    let symbol = "tst";
    let uri = "https://www.test.com";

    const tx = await program.methods
      .create(name, symbol, uri)
      .accounts({
        mint: mint.publicKey,
        creator: tokenCreator.publicKey,
        bondingCurveTokenAccount: bondingCurveTokenAccount,
        program: program.programId,
      })
      .transaction();

    let txResult = await sendTransaction(
      program,
      tx,
      [mint, tokenCreator],
      tokenCreator.publicKey
    );

    let createEvents = txResult.events.filter((event) => {
      return event.name === "createEvent";
    });

    assert.equal(createEvents.length, 1);

    let createEvent = toEvent("createEvent", createEvents[0]);
    assert.notEqual(createEvent, null);
    if (createEvent != null) {
      assert.equal(createEvent.name, name);
      assert.equal(createEvent.symbol, symbol);
      assert.equal(createEvent.uri, uri);
      assert.equal(createEvent.mint.toBase58(), mint.publicKey.toBase58());
      assert.equal(
        createEvent.bondingCurve.toBase58(),
        bondingCurvePDA.toBase58()
      );
      assert.equal(
        createEvent.creator.toBase58(),
        tokenCreator.publicKey.toBase58()
      );
    }

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

    let bondingCurveTokenAccountInfo = await connection.getTokenAccountBalance(
      bondingCurveTokenAccount
    );

    assert.equal(
      bondingCurveTokenAccountInfo.value.amount,
      DEFAULT_TOKEN_BALANCE.toString()
    );

    let bondingCurveAccount = await program.account.bondingCurve.fetch(
      bondingCurvePDA
    );

    assert.equal(
      bondingCurveAccount.virtualTokenReserves.toString(),
      DEFUALT_INITIAL_VIRTUAL_TOKEN_RESERVE.toString()
    );
    assert.equal(
      bondingCurveAccount.virtualSolReserves.toString(),
      DEFAULT_INITIAL_VIRTUAL_SOL_RESERVE.toString()
    );
    assert.equal(
      bondingCurveAccount.realTokenReserves.toString(),
      DEFAULT_INITIAL_TOKEN_RESERVES.toString()
    );
    assert.equal(bondingCurveAccount.realSolReserves.toString(), "0");
    assert.equal(
      bondingCurveAccount.tokenTotalSupply.toString(),
      DEFAULT_TOKEN_BALANCE.toString()
    );
    assert.equal(bondingCurveAccount.complete, false);
  });

  it("can buy a token", async () => {
    let currentAMM = await getAmmFromBondingCurve();

    let buyMaxSOLAmount = BigInt(10 * LAMPORTS_PER_SOL);
    let buyTokenAmount = DEFAULT_TOKEN_BALANCE / 100n;

    let buyResult = currentAMM.applyBuy(buyTokenAmount);

    let txResult = await simpleBuy(
      tokenCreator,
      buyTokenAmount,
      buyMaxSOLAmount
    );

    let targetCurrentSupply = (
      DEFAULT_TOKEN_BALANCE - buyTokenAmount
    ).toString();

    let tradeEvents = txResult.tx.events.filter((event) => {
      return event.name === "tradeEvent";
    });
    assert.equal(tradeEvents.length, 1);

    let tradeEvent = toEvent("tradeEvent", tradeEvents[0]);
    assert.notEqual(tradeEvent, null);
    if (tradeEvent != null) {
      assert.equal(
        tradeEvent.tokenAmount.toString(),
        buyTokenAmount.toString()
      );

      assert.equal(tradeEvent.isBuy, true);
      assert.equal(
        tradeEvent.solAmount.toString(),
        buyResult.sol_amount.toString()
      );
    }

    const tokenAmount = await connection.getTokenAccountBalance(
      txResult.userTokenAccount.address
    );
    assert.equal(tokenAmount.value.amount, buyTokenAmount.toString());

    let bondingCurveTokenAccountInfo = await connection.getTokenAccountBalance(
      txResult.bondingCurveTokenAccount
    );

    assert.equal(
      bondingCurveTokenAccountInfo.value.amount,
      targetCurrentSupply
    );

    let bondingCurveAccount = await program.account.bondingCurve.fetch(
      bondingCurvePDA
    );

    assertBondingCurve(currentAMM, bondingCurveAccount);
  });

  it("can sell a token", async () => {
    let currentAMM = await getAmmFromBondingCurve();

    let tokenAmount = 10000000n;
    let minSolAmount = 0n;

    let sellResults = currentAMM.applySell(tokenAmount);

    let userPreSaleBalance = await getSPLBalance(
      connection,
      mint.publicKey,
      tokenCreator.publicKey
    );

    let curvePreSaleBalance = await getSPLBalance(
      connection,
      mint.publicKey,
      bondingCurvePDA,
      true
    );

    let txResult = await simpleSell(tokenCreator, tokenAmount, minSolAmount);

    let tradeEvents = txResult.tx.events.filter((event) => {
      return event.name === "tradeEvent";
    });

    let userPostSaleBalance = await getSPLBalance(
      connection,
      mint.publicKey,
      tokenCreator.publicKey
    );

    assert.equal(
      userPostSaleBalance,
      (BigInt(userPreSaleBalance) - tokenAmount).toString()
    );
    assert.equal(tradeEvents.length, 1);

    let tradeEvent = toEvent("tradeEvent", tradeEvents[0]);
    assert.notEqual(tradeEvent, null);
    if (tradeEvent != null) {
      assert.equal(tradeEvent.tokenAmount.toString(), tokenAmount.toString());
      assert.equal(tradeEvent.isBuy, false);
      assert.equal(
        tradeEvent.solAmount.toString(),
        sellResults.sol_amount.toString()
      );
    }

    let bondingCurveTokenAccountInfo = await connection.getTokenAccountBalance(
      txResult.bondingCurveTokenAccount
    );

    let curvePostSaleBalance = await getSPLBalance(
      connection,
      mint.publicKey,
      bondingCurvePDA,
      true
    );

    assert.equal(
      bondingCurveTokenAccountInfo.value.amount,
      (BigInt(curvePreSaleBalance) + tokenAmount).toString()
    );

    let bondingCurveAccount = await program.account.bondingCurve.fetch(
      bondingCurvePDA
    );
    assertBondingCurve(currentAMM, bondingCurveAccount);
  });

  //excpetion unit tests

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
      let anchorError = getAnchorError(err);
      if (anchorError) {
        errorCode = anchorError.error.errorCode.code;
      }
    }
    assert.equal(errorCode, "InsufficientSOL");
  });

  it("can't buy a token, exceed max sol", async () => {
    let errorCode = "";
    try {
      await simpleBuy(tokenCreator, DEFAULT_TOKEN_BALANCE / 100n, 1n);
    } catch (err) {
      let anchorError = getAnchorError(err);
      if (anchorError) {
        errorCode = anchorError.error.errorCode.code;
      }
    }
    assert.equal(errorCode, "MaxSOLCostExceeded");
  });

  it("can't buy 0 tokens", async () => {
    let errorCode = "";
    try {
      await simpleBuy(tokenCreator, 0n, 1n);
    } catch (err) {
      let anchorError = getAnchorError(err);
      if (anchorError) {
        errorCode = anchorError.error.errorCode.code;
      }
    }
    assert.equal(errorCode, "MinBuy");
  });

  it("can't sell a token, not enough tokens", async () => {
    let errorCode = "";
    try {
      await simpleSell(tokenCreator, DEFAULT_TOKEN_BALANCE, 0n);
    } catch (err) {
      let anchorError = getAnchorError(err);
      if (anchorError) {
        errorCode = anchorError.error.errorCode.code;
      }
    }
    assert.equal(errorCode, "InsufficientTokens");
  });

  it("can't sell 0 tokens", async () => {
    let errorCode = "";
    try {
      await simpleSell(tokenCreator, 0n, 0n);
    } catch (err) {
      let anchorError = getAnchorError(err);
      if (anchorError) {
        errorCode = anchorError.error.errorCode.code;
      }
    }
    assert.equal(errorCode, "MinSell");
  });

  it("can't sell a token, exceed mint sol sell", async () => {
    let errorCode = "";
    try {
      await simpleSell(tokenCreator, 1n, DEFAULT_TOKEN_BALANCE);
    } catch (err) {
      let anchorError = getAnchorError(err);
      if (anchorError) {
        errorCode = anchorError.error.errorCode.code;
      }
    }
    assert.equal(errorCode, "MinSOLOutputExceeded");
  });

  //curve complete unit tests
  it("can complete the curve", async () => {
    let currentAMM = await getAmmFromBondingCurve();

    let buyTokenAmount = currentAMM.realTokenReserves;
    let maxSolAmount = currentAMM.getBuyPrice(buyTokenAmount);

    let buyResult = currentAMM.applyBuy(buyTokenAmount);

    let userPrePurchaseBalance = await getSPLBalance(
      connection,
      mint.publicKey,
      tokenCreator.publicKey
    );

    let txResult = await simpleBuy(tokenCreator, buyTokenAmount, maxSolAmount);

    let tradeEvents = txResult.tx.events.filter((event) => {
      return event.name === "tradeEvent";
    });
    assert.equal(tradeEvents.length, 1);

    let tradeEvent = toEvent("tradeEvent", tradeEvents[0]);
    assert.notEqual(tradeEvent, null);
    if (tradeEvent != null) {
      assert.equal(tradeEvent.isBuy, true);
      assert.equal(
        tradeEvent.solAmount.toString(),
        buyResult.sol_amount.toString()
      );
    }

    let userPostPurchaseBalance = await getSPLBalance(
      connection,
      mint.publicKey,
      tokenCreator.publicKey
    );

    assert.equal(
      userPostPurchaseBalance,
      (BigInt(userPrePurchaseBalance) + buyTokenAmount).toString()
    );

    let bondingCurveTokenAccountInfo = await connection.getTokenAccountBalance(
      txResult.bondingCurveTokenAccount
    );

    assert.equal(
      (
        BigInt(bondingCurveTokenAccountInfo.value.amount) +
        DEFAULT_INITIAL_TOKEN_RESERVES
      ).toString(),
      DEFAULT_TOKEN_BALANCE.toString()
    );

    let bondingCurveAccount = await program.account.bondingCurve.fetch(
      bondingCurvePDA
    );

    assertBondingCurve(currentAMM, bondingCurveAccount, true);
  });

  it("can't buy a token, curve complete", async () => {
    let currentAMM = await getAmmFromBondingCurve();

    let buyTokenAmount = 100n;
    let maxSolAmount = currentAMM.getBuyPrice(buyTokenAmount);

    let errorCode = "";
    try {
      await simpleBuy(tokenCreator, buyTokenAmount, maxSolAmount);
    } catch (err) {
      let anchorError = getAnchorError(err);
      if (anchorError) {
        errorCode = anchorError.error.errorCode.code;
      }
    }
    assert.equal(errorCode, "BondingCurveComplete");
  });

  it("can't sell a token, curve complete", async () => {
    let tokenAmount = 100n;
    let minSolAmount = 0n;

    let errorCode = "";
    try {
      await simpleSell(tokenCreator, tokenAmount, minSolAmount);
    } catch (err) {
      let anchorError = getAnchorError(err);
      if (anchorError) {
        errorCode = anchorError.error.errorCode.code;
      }
    }
    assert.equal(errorCode, "BondingCurveComplete");
  });

  //param unit tests
  it("can set params", async () => {
    let tx = await program.methods
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
        program: program.programId,
      })
      .transaction();

    let txResult = await sendTransaction(
      program,
      tx,
      [authority],
      authority.publicKey
    );

    let global = await program.account.global.fetch(globalPDA);

    let setParamsEvents = txResult.events.filter((event) => {
      return event.name === "setParamsEvent";
    });

    assert.equal(setParamsEvents.length, 1);

    let setParamsEvent = toEvent("setParamsEvent", setParamsEvents[0]);
    assert.notEqual(setParamsEvent, null);
    if (setParamsEvent != null) {
      assert.equal(
        setParamsEvent.feeRecipient.toBase58(),
        TOKEN_METADATA_PROGRAM_ID.toBase58()
      );
      assert.equal(
        setParamsEvent.initialVirtualTokenReserves.toString(),
        new BN(1000).toString()
      );
      assert.equal(
        setParamsEvent.initialVirtualSolReserves.toString(),
        new BN(2000).toString()
      );
      assert.equal(
        setParamsEvent.initialRealTokenReserves.toString(),
        new BN(3000).toString()
      );
      assert.equal(
        setParamsEvent.initialTokenSupply.toString(),
        new BN(4000).toString()
      );
      assert.equal(
        setParamsEvent.feeBasisPoints.toString(),
        new BN(100).toString()
      );
    }

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

  it("can't set params as non-authority", async () => {
    let errorCode = "";
    try {
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
          user: tokenCreator.publicKey,
          program: program.programId,
        })
        .signers([tokenCreator])
        .rpc();
    } catch (err) {
      let anchorError = getAnchorError(err);
      if (anchorError) {
        errorCode = anchorError.error.errorCode.code;
      }
    }
    assert.equal(errorCode, "InvalidAuthority");
  });
});

//TODO: Tests
// test buy
// complete curve
// test sell whole curve
