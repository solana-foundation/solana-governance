import type { Address } from "@solana/kit";

export interface VoteOverrideAccountData {
  publicKey: Address;
  stakeAccount: Address;
  delegator: Address;
  validator: Address;
  proposal: Address;
  voteAccountValidator: Address;
  forVotesBp: bigint;
  againstVotesBp: bigint;
  abstainVotesBp: bigint;
  forVotesLamports: bigint;
  againstVotesLamports: bigint;
  abstainVotesLamports: bigint;
  stakeAmount: bigint;
  voteOverrideTimestamp: bigint;
  bump: number;
}
