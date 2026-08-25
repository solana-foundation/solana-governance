import { createSolanaRpc } from "@solana/kit";

export interface RpcVoteAccountData {
  activatedStake: bigint;
  commission: number;
  epochCredits: readonly [bigint, bigint, bigint][];
  lastVote: bigint;
  nodePubkey: string;
  votePubkey: string;
}

export interface RawVoteAccountsData {
  current: RpcVoteAccountData[];
  delinquent: RpcVoteAccountData[];
}

export interface ChainVoteAccountData {
  activeStake: bigint;
  voteAccount: string;
  nodePubkey: string;
}

function mapVoteAccount(account: {
  activatedStake: bigint;
  commission: number;
  epochCredits: readonly (readonly [bigint, bigint, bigint])[];
  lastVote: bigint;
  nodePubkey: string;
  votePubkey: string;
}): RpcVoteAccountData {
  return {
    activatedStake: account.activatedStake,
    commission: account.commission,
    epochCredits: account.epochCredits.map(([epoch, credits, previous]) => [
      epoch,
      credits,
      previous,
    ]),
    lastVote: account.lastVote,
    nodePubkey: account.nodePubkey,
    votePubkey: account.votePubkey,
  };
}

export async function fetchRawVoteAccounts(endpoint: string): Promise<RawVoteAccountsData> {
  const accounts = await createSolanaRpc(endpoint).getVoteAccounts().send();
  return {
    current: accounts.current.map(mapVoteAccount),
    delinquent: accounts.delinquent.map(mapVoteAccount),
  };
}
