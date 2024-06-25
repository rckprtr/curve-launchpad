import * as anchor from "@coral-xyz/anchor";
import {
  Connection,
  LAMPORTS_PER_SOL,
  PublicKey,
  SendTransactionError,
  Transaction,
  TransactionMessage,
  VersionedTransaction,
} from "@solana/web3.js";
import { CurveLaunchpad } from "../target/types/curve_launchpad";
import * as client from "../client/";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";


type EventKeys = keyof anchor.IdlEvents<CurveLaunchpad>;

const validEventNames: Array<keyof anchor.IdlEvents<CurveLaunchpad>> = [
  "completeEvent",
  "createEvent",
  "setParamsEvent",
  "tradeEvent",
];

export const getTransactionEvents = (
  program: anchor.Program<CurveLaunchpad>,
  txResponse: anchor.web3.VersionedTransactionResponse | null
) => {
  if (!txResponse) {
    return [];
  }

  let [eventPDA] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("__event_authority")],
    program.programId
  );

  let indexOfEventPDA =
    txResponse.transaction.message.staticAccountKeys.findIndex((key) =>
      key.equals(eventPDA)
    );

  if (indexOfEventPDA === -1) {
    return [];
  }

  const matchingInstructions = txResponse.meta?.innerInstructions
    ?.flatMap((ix) => ix.instructions)
    .filter(
      (instruction) =>
        instruction.accounts.length === 1 &&
        instruction.accounts[0] === indexOfEventPDA
    );

  if (matchingInstructions) {
    let events = matchingInstructions.map((instruction) => {
      const ixData = anchor.utils.bytes.bs58.decode(instruction.data);
      const eventData = anchor.utils.bytes.base64.encode(ixData.slice(8));
      const event = program.coder.events.decode(eventData);
      return event;
    });
    const isNotNull = <T>(value: T | null): value is T => {
      return value !== null;
    };
    return events.filter(isNotNull);
  } else {
    return [];
  }
};

const isEventName = (
  eventName: string
): eventName is keyof anchor.IdlEvents<CurveLaunchpad> => {
  return validEventNames.includes(
    eventName as keyof anchor.IdlEvents<CurveLaunchpad>
  );
};

export const toEvent = <E extends EventKeys>(
  eventName: E,
  event: any
): anchor.IdlEvents<CurveLaunchpad>[E] | null => {
  if (isEventName(eventName)) {
    return getEvent(eventName, event.data);
  }
  return null;
};

const getEvent = <E extends EventKeys>(
  eventName: E,
  event: anchor.IdlEvents<CurveLaunchpad>[E]
): anchor.IdlEvents<CurveLaunchpad>[E] => {
  return event;
};

export const buildVersionedTx = async (
  connection: anchor.web3.Connection,
  payer: PublicKey,
  tx: Transaction
) => {
  const blockHash = (await connection.getLatestBlockhash("processed"))
    .blockhash;

  let messageV0 = new TransactionMessage({
    payerKey: payer,
    recentBlockhash: blockHash,
    instructions: tx.instructions,
  }).compileToV0Message();

  return new VersionedTransaction(messageV0);
};

export const getTxDetails = async (connection: anchor.web3.Connection, sig) => {
  const latestBlockHash = await connection.getLatestBlockhash("processed");

  await connection.confirmTransaction(
    {
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: sig,
    },
    "confirmed"
  );

  return await connection.getTransaction(sig, {
    maxSupportedTransactionVersion: 0,
    commitment: "confirmed",
  });
};

export const sendTransaction = async (
  program: anchor.Program<CurveLaunchpad>,
  tx: Transaction,
  signers: anchor.web3.Signer[],
  payer: PublicKey
) => {
  const versionedTx = await buildVersionedTx(
    program.provider.connection,
    payer,
    tx
  );
  versionedTx.sign(signers);

  let sig = await program.provider.connection.sendTransaction(versionedTx);
  let response = await getTxDetails(program.provider.connection, sig);
  let events = getTransactionEvents(program, response);
  return {
    response,
    events,
  };
};

export const getAnchorError = (error: any) => {
  if (error instanceof anchor.AnchorError) {
    return error;
  } else if (error instanceof SendTransactionError) {
    return anchor.AnchorError.parse(error.logs || []);
  }
  return null;
};

export const fundAccountSOL = async (
  connection: anchor.web3.Connection,
  publicKey: anchor.web3.PublicKey,
  amount: number
) => {
  let fundSig = await connection.requestAirdrop(publicKey, amount);

  return getTxDetails(connection, fundSig);
};

export const ammFromBondingCurve = (
  bondingCurveAccount: anchor.IdlAccounts<CurveLaunchpad>["bondingCurve"] | null,
  initialVirtualTokenReserves: bigint
) => {
  if(!bondingCurveAccount) throw new Error("Bonding curve account not found");
  return new client.AMM(
    BigInt(bondingCurveAccount.virtualSolReserves.toString()),
    BigInt(bondingCurveAccount.virtualTokenReserves.toString()),
    BigInt(bondingCurveAccount.realSolReserves.toString()),
    BigInt(bondingCurveAccount.realTokenReserves.toString()),
    initialVirtualTokenReserves
  );
};

export const bigIntToSOL = (amount: bigint) => {
  return amount / BigInt(LAMPORTS_PER_SOL);
}

export const getSPLBalance = async (
  connection: Connection,
  mintAddress: PublicKey,
  pubKey: PublicKey,
  allowOffCurve: boolean = false
) => {
  try {
    let ata = getAssociatedTokenAddressSync(mintAddress, pubKey, allowOffCurve);
    const balance = await connection.getTokenAccountBalance(ata, "processed");
    return balance.value.amount;
  } catch (e) {
    console.error(e);
  }
  return '0';
};