import type { BN, Program, ProgramAccount } from "@coral-xyz/anchor";

import { PublicKey } from "@solana/web3.js";
import { SvmgovProgramOLD } from "./OLD.svmgov_program";

/**
 * @deprecated old code
 */
export type GovProgram = Program<SvmgovProgramOLD>;
/**
 * @deprecated old code
 */
export interface ProposalOld {
  author: PublicKey;
  title: string;
  description: string;
  creation_epoch: BN;
  startEpoch: BN;
  endEpoch: BN;
  proposerStakeWeightBp: BN;
  clusterSupportBp: BN;
  forVotesBp: BN;
  againstVotesBp: BN;
  abstainVotesBp: BN;
  voting: boolean;
  finalized: boolean;
  proposalBump: number;
}
/**
 * @deprecated old code
 */
export interface SupportOld {
  proposal: PublicKey;
  validator: PublicKey;
  bump: number;
}
/**
 * @deprecated old code
 */
interface RawVoteOld {
  proposal: PublicKey;
  forVotesBp: BN;
  againstVotesBp: BN;
  abstainVotesBp: BN;
  voteTimestamp: BN;
  bump: number;
}
/**
 * @deprecated old code
 */
export type VoteOld = ProgramAccount<RawVoteOld>;
/**
 * @deprecated old code
 */
export type VotesOld = VoteOld[];
