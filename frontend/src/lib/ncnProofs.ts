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

export type NetworkMetaResponse = {
  network: string;
  slot: bigint;
  merkle_root: string;
  snapshot_hash: string;
  created_at: string;
  total_active_stake: bigint | null;
};

export type NetworkMetaWireResponse = Omit<
  NetworkMetaResponse,
  "slot" | "total_active_stake"
> & {
  slot: string;
  total_active_stake: string | null;
};

export function deserializeNetworkMeta(
  meta: NetworkMetaWireResponse,
): NetworkMetaResponse {
  return {
    ...meta,
    slot: BigInt(meta.slot),
    total_active_stake:
      meta.total_active_stake === null
        ? null
        : BigInt(meta.total_active_stake),
  };
}
export type VoterSummaryResponse = {
  network: string; snapshot_slot: number; voting_wallet: string;
  stake_accounts: { active_stake: number; stake_account: string; vote_account: string }[];
  vote_accounts: { activeStake: number; voteAccount: string }[];
};

export function getVoteAccountProof(voteAccount: string, network: string, slot: bigint, ncnApiUrl?: string) {
  return fetchNcnJson<VoteAccountProof>(`${ncnApiUrl ?? DEFAULT_NCN_API_URL}/proof/vote_account/${voteAccount}?network=${network}&slot=${slot}`, { label: "vote account proof" });
}

export function getStakeAccountProof(stakeAccount: string, network: string, slot: bigint, ncnApiUrl?: string) {
  return fetchNcnJson<StakeAccountProof>(`${ncnApiUrl ?? DEFAULT_NCN_API_URL}/proof/stake_account/${stakeAccount}?network=${network}&slot=${slot}`, { label: "stake account proof" });
}

/**
 * Proofs must be read from the proposal's committed snapshot, never from the
 * NCN API's current `/meta` slot. The latter can move between proposal creation
 * and a voter's transaction.
 */
export function requireProposalSnapshotSlot(snapshotSlot: bigint): bigint {
  if (snapshotSlot <= 0n) {
    throw new Error(
      "Proposal has no snapshot slot; voting may not have been activated yet",
    );
  }
  return snapshotSlot;
}

/**
 * Ensure the two proofs refer to the same validator vote account.
 *
 * Do not compare `stake_merkle_leaf.voting_wallet` with
 * `meta_merkle_leaf.voting_wallet`: delegator and validator wallet fields are
 * intentionally allowed to differ for an override vote.
 */
export function assertOverrideProofLineage(
  stakeProof: Pick<StakeAccountProof, "vote_account">,
  metaProof: Pick<VoteAccountProof, "meta_merkle_leaf">,
): void {
  if (metaProof.meta_merkle_leaf.vote_account !== stakeProof.vote_account) {
    throw new Error("Stake and vote proofs are from different snapshots");
  }
}
