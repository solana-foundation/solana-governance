import { createSolanaRpc } from "@solana/kit";
import { fetchVotes, type Vote } from "@/lib/governance/programAccounts";
import { toLegacyBn, toLegacyPublicKey } from "@/lib/governance/legacyAdapters";
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
  address: string,
): OldVoteAccountData {
  return {
    voteAccount: toLegacyPublicKey(address),
    proposal: toLegacyPublicKey(raw.proposal),
    // validator data
    activeStake: Number(raw.stake),
    identity: toLegacyPublicKey(raw.validator),
    commission: 0,
    lastVote: 0,
    credits: 0,
    epochCredits: 0,
    activatedStake: 0,
    // vote data
    forVotesBp: toLegacyBn(raw.forVotesBp), againstVotesBp: toLegacyBn(raw.againstVotesBp), abstainVotesBp: toLegacyBn(raw.abstainVotesBp),
    forVotesLamports: toLegacyBn(raw.forVotesLamports), againstVotesLamports: toLegacyBn(raw.againstVotesLamports), abstainVotesLamports: toLegacyBn(raw.abstainVotesLamports),
    stake: toLegacyBn(raw.stake), overrideLamports: toLegacyBn(raw.overrideLamports), voteTimestamp: toLegacyBn(raw.voteTimestamp),
    bump: raw.bump,
  };
}
