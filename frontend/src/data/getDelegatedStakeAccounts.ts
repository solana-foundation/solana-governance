import { createSolanaRpc } from "@solana/kit";
import {
  fetchDelegatedStakeAccounts,
  type WalletStakeAccount,
} from "@/lib/stakeAccounts";

interface ParsedStakeAccount {
  stakeAccount: string;
  voter: string | undefined;
  amountLamports: bigint;
  staker: string | undefined;
  withdrawer: string | undefined;
  state: string;
}

export const getDelegatedStakeAccounts = async (
  endpoint: string,
  validatorIdentityPubKey: string | undefined,
): Promise<ParsedStakeAccount[]> => {
  if (!validatorIdentityPubKey) {
    throw new Error("Validator identity public key is required");
  }

  const rpc = createSolanaRpc(endpoint);
  const voteAccounts = await rpc.getVoteAccounts().send();
  const validatorVotes = [...voteAccounts.current, ...voteAccounts.delinquent]
    .filter((vote) => vote.nodePubkey === validatorIdentityPubKey);

  const accounts = await Promise.all(
    validatorVotes.map((vote) => fetchDelegatedStakeAccounts(rpc, vote.votePubkey)),
  );
  return accounts.flat().map(mapDelegatedStakeAccountDto);
};

function mapDelegatedStakeAccountDto(raw: WalletStakeAccount): ParsedStakeAccount {
  return {
    stakeAccount: raw.address,
    voter: raw.voter ?? undefined,
    amountLamports: raw.activeStakeLamports,
    staker: raw.staker,
    withdrawer: raw.withdrawer,
    state: raw.state,
  };
}
