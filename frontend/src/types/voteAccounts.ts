import type { LegacyBn as BN, LegacyPublicKey as PublicKey } from "@/lib/governance/legacyAdapters";

export interface VoteAccountData {
  validator: PublicKey;
  proposal: PublicKey;
  forVotesBp: BN;
  againstVotesBp: BN;
  abstainVotesBp: BN;
  forVotesLamports: BN;
  againstVotesLamports: BN;
  abstainVotesLamports: BN;
  stake: BN;
  overrideLamports: BN;
  voteTimestamp: BN;
  bump: number;
}

export interface OldVoteAccountData {
  voteAccount: PublicKey;
  proposal: PublicKey;
  activeStake: number;
  identity?: PublicKey;
  name?: string;
  commission?: number;
  lastVote?: number;
  credits?: number;
  epochCredits?: number;
  activatedStake?: number;
  forVotesBp: BN;
  againstVotesBp: BN;
  abstainVotesBp: BN;
  forVotesLamports: BN;
  againstVotesLamports: BN;
  abstainVotesLamports: BN;
  stake: BN;
  overrideLamports: BN;
  voteTimestamp: BN;
  bump: number;
}
