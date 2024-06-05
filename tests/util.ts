import * as anchor from "@coral-xyz/anchor";

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
