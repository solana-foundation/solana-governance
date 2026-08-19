import type { LegacyBn, LegacyPublicKey } from "@/lib/governance/legacyAdapters";

export interface VoteOverrideAccountData {
  publicKey: LegacyPublicKey;
  stakeAccount: LegacyPublicKey;
  delegator: LegacyPublicKey;
  validator: LegacyPublicKey;
  proposal: LegacyPublicKey;
  voteAccountValidator: LegacyPublicKey;
  forVotesBp: LegacyBn;
  againstVotesBp: LegacyBn;
  abstainVotesBp: LegacyBn;
  forVotesLamports: LegacyBn;
  againstVotesLamports: LegacyBn;
  abstainVotesLamports: LegacyBn;
  stakeAmount: LegacyBn;
  voteOverrideTimestamp: LegacyBn;
  bump: number;
}
