import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { CurveSocial } from "../target/types/curve_social";
import { LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import { getTxDetails } from "./util";
import {
  getAssociatedTokenAddress,
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";
import { BN } from "bn.js";
import { assert } from "chai";

const GLOBAL_SEED = "global";
const METADATA_SEED = "metadata";
const MINT_AUTHORITY_SEED = "mint-authority";
const BONDING_CURVE_SEED = "bonding-curve";
const TOKEN_METADATA_PROGRAM_ID = new PublicKey(
  "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
);

describe("curve-social", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.CurveSocial as Program<CurveSocial>;

  const connection = provider.connection;
  const authority = anchor.web3.Keypair.generate();
  const tokenCreator = anchor.web3.Keypair.generate();

  const mint = anchor.web3.Keypair.generate();

  const [globalPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from(GLOBAL_SEED)],
    program.programId
  );

  const [mintAuthorityPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from(MINT_AUTHORITY_SEED)],
    program.programId
  );

  before(async () => {
    let fundSig = await connection.requestAirdrop(
      authority.publicKey,
      5 * LAMPORTS_PER_SOL
    );

    await getTxDetails(connection, fundSig);

    let fundSigtokenCreator = await connection.requestAirdrop(
      tokenCreator.publicKey,
      5 * LAMPORTS_PER_SOL
    );

    await getTxDetails(connection, fundSigtokenCreator);
  });

  it("Is initialized!", async () => {
    // Add your test here.
    const initializeTx = await program.methods
      .initialize()
      .accounts({
        authority: authority.publicKey,
        global: globalPDA,
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

    const createTx = await program.methods
      .create("test", "tst", "https://www.test.com")
      .accounts({
        mint: mint.publicKey,
        creator: tokenCreator.publicKey,
        mintAuthority: mintAuthorityPDA,
        bondingCurve: bondingCurvePDA,
        bondingCurveTokenAccount: bondingCurveTokenAccount,
        global: globalPDA,
        metadata: metadataPDA,
      })
      .signers([mint, tokenCreator])
      .rpc();
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

    const buyTx = await program.methods
      .buy(new BN(1000), new BN(555555))
      .accounts({
        user: tokenCreator.publicKey,
        global: globalPDA,
        mint: mint.publicKey,
        bondingCurve: bondingCurvePDA,
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

    const sellTx = await program.methods
      .sell(new BN(1000), new BN(1))
      .accounts({
        user: tokenCreator.publicKey,
        global: globalPDA,
        mint: mint.publicKey,
        bondingCurve: bondingCurvePDA,
        bondingCurveTokenAccount: bondingCurveTokenAccount,
        userTokenAccount: userTokenAccount.address,
      })
      .signers([tokenCreator])
      .rpc();
  });

  it("can set params", async () => {
    const setParamsTx = await program.methods
      .setParams(
        TOKEN_METADATA_PROGRAM_ID,
        new BN(1000),
        new BN(1000),
        new BN(1000),
        new BN(1000),
        new BN(100)
      )
      .accounts({
        user: authority.publicKey,
        global: globalPDA,
      })
      .signers([authority])
      .rpc();

    let global = await program.account.global.fetch(globalPDA);

    assert.equal(global.feeRecipient.toBase58(), TOKEN_METADATA_PROGRAM_ID.toBase58());
    assert.equal(global.initialVirtualTokenReserves.toString(), new BN(1000).toString());
    assert.equal(global.initialVirtualSolReserves.toString(), new BN(1000).toString());
    assert.equal(global.initialRealTokenReserves.toString(), new BN(1000).toString());
    assert.equal(global.initialTokenSupply.toString(), new BN(1000).toString());
    assert.equal(global.feeBasisPoints.toString(), new BN(100).toString());

  });

});
