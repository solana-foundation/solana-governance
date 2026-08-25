import { createSolanaRpc, type Address } from "@solana/kit";
import { fetchVotes, type Vote } from "@/lib/governance/programAccounts";
import { OldVoteAccountData } from "@/types";

/**
 * @deprecated cant fetch ALL vote accounts at once.
 */
export const getVoteAccounts = async (
  endpoint: string,
): Promise<OldVoteAccountData[]> => {
  const voteAccs = await fetchVotes(createSolanaRpc(endpoint), {});
  return voteAccs.map(({ address, data }) => mapVoteAccountDto(data, address));
};

/**
 * Maps raw on-chain vote account to internal type.
 */
export function mapVoteAccountDto(
  raw: Vote,
  address: Address,
): OldVoteAccountData {
  return {
    voteAccount: address,
    proposal: raw.proposal,
    // validator data
    activeStake: raw.stake,
    identity: raw.validator,
    commission: 0,
    lastVote: 0n,
    credits: 0,
    epochCredits: 0n,
    activatedStake: 0n,
    // vote data
    forVotesBp: raw.forVotesBp, againstVotesBp: raw.againstVotesBp, abstainVotesBp: raw.abstainVotesBp,
    forVotesLamports: raw.forVotesLamports, againstVotesLamports: raw.againstVotesLamports, abstainVotesLamports: raw.abstainVotesLamports,
    stake: raw.stake, overrideLamports: raw.overrideLamports, voteTimestamp: raw.voteTimestamp,
    bump: raw.bump,
  };
}
