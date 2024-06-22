export const calculateFee = (amount: bigint, fee: number): bigint => {
  return (amount * BigInt(fee)) / 10000n;
};
