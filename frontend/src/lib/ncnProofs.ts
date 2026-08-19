import { DEFAULT_NCN_API_URL, fetchNcnJson } from "@/lib/ncnApi";

export type VoteAccountProof = {
  meta_merkle_leaf: { active_stake: number; stake_merkle_root: string; vote_account: string; voting_wallet: string };
  meta_merkle_proof: string[];
};
export type StakeAccountProof = {
  stake_merkle_leaf: { active_stake: number; stake_account: string; voting_wallet: string };
  stake_merkle_proof: string[];
  vote_account: string;
};

export function getVoteAccountProof(voteAccount: string, network: string, slot: bigint, ncnApiUrl?: string) {
  return fetchNcnJson<VoteAccountProof>(`${ncnApiUrl ?? DEFAULT_NCN_API_URL}/proof/vote_account/${voteAccount}?network=${network}&slot=${slot}`, { label: "vote account proof" });
}

export function getStakeAccountProof(stakeAccount: string, network: string, slot: bigint, ncnApiUrl?: string) {
  return fetchNcnJson<StakeAccountProof>(`${ncnApiUrl ?? DEFAULT_NCN_API_URL}/proof/stake_account/${stakeAccount}?network=${network}&slot=${slot}`, { label: "stake account proof" });
}
