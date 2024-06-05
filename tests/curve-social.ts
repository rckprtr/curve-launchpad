import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { CurveSocial } from "../target/types/curve_social";
import {
  LAMPORTS_PER_SOL,
  PublicKey,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import { getTxDetails } from "./util";

const GLOBAL_SEED = "global";

describe("curve-social", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.CurveSocial as Program<CurveSocial>;

  const connection = provider.connection;
  const authority = anchor.web3.Keypair.generate();

  const [globalPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from(GLOBAL_SEED)],
    program.programId
  );

  before(async () => {
    let fundSig  = await connection.requestAirdrop(
      authority.publicKey,
      LAMPORTS_PER_SOL
    );

    await getTxDetails(connection, fundSig);
  });

  it("Is initialized!", async () => {
    // Add your test here.
    console.log("authority", globalPDA);
    const initializeTx = await program.methods.initialize().accounts(
      {
        authority: authority.publicKey,
        global: globalPDA,
      }
    ).signers([authority]).rpc();

    console.log("Your transaction signature", initializeTx);
  });
});
