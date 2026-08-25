import { createSolanaRpc, type Address } from "@solana/kit";
import { fetchStakeAccounts, type WalletStakeAccount } from "@/lib/stakeAccounts";
import { StakeAccountData } from "@/types/stakeAccounts";

export const getWalletStakeAccounts = async (
  endpoint: string,
  userPubkey: string | undefined,
): Promise<StakeAccountData[]> => {
  if (!userPubkey) throw new Error("User public key is required");

  const accounts = await fetchStakeAccounts(
    createSolanaRpc(endpoint),
    userPubkey as Address,
  );
  return accounts.map(mapStakeAccountDto);
};

export function mapStakeAccountDto(
  raw: WalletStakeAccount,
  index: number,
): StakeAccountData {
  return {
    id: index.toString(),
    voteAccount: raw.voter ?? undefined,
    activeStake: raw.activeStakeLamports,
    stakeAccount: raw.address,
    state: raw.state,
  };
}
