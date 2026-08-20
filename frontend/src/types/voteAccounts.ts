import type { Address } from "@solana/kit";

export interface VoteAccountData {
  validator: Address;
  proposal: Address;
  forVotesBp: bigint;
  againstVotesBp: bigint;
  abstainVotesBp: bigint;
  forVotesLamports: bigint;
  againstVotesLamports: bigint;
  abstainVotesLamports: bigint;
  stake: bigint;
  overrideLamports: bigint;
  voteTimestamp: bigint;
  bump: number;
}

export interface OldVoteAccountData {
  voteAccount: Address;
  proposal: Address;
  activeStake: bigint;
  identity?: Address;
  name?: string;
  commission?: number;
  lastVote?: bigint;
  credits?: number;
  epochCredits?: bigint;
  activatedStake?: bigint;
  forVotesBp: bigint;
  againstVotesBp: bigint;
  abstainVotesBp: bigint;
  forVotesLamports: bigint;
  againstVotesLamports: bigint;
  abstainVotesLamports: bigint;
  stake: bigint;
  overrideLamports: bigint;
  voteTimestamp: bigint;
  bump: number;
}
