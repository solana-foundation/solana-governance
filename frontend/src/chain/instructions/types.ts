import { AnchorWallet } from "@solana/wallet-adapter-react";
import { PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";

import svmgovProgramIdl from "@/chain/idl/svmgov_program.json";
import govV1idl from "@/chain/idl/gov-v1.json";
import { RPCEndpoint } from "@/types";

// Common types
export interface TransactionResult {
  signature: string;
  success: boolean;
  error?: string;
}

export interface BlockchainParams {
  network: RPCEndpoint;
  endpoint: string;
  ncnApiUrl?: string;
}

// Instruction parameter types
export interface CreateProposalParams {
  title: string;
  description: string;
  seed?: number;
  wallet: AnchorWallet | undefined;
  voteAccount?: PublicKey;
}

export interface CastVoteParams {
  proposalId: string;
  forVotesBp: number;
  againstVotesBp: number;
  abstainVotesBp: number;
  wallet: AnchorWallet | undefined;
  voteAccount?: PublicKey;
  consensusResult: PublicKey;
}

export interface ModifyVoteParams {
  proposalId: string;
  forVotesBp: number;
  againstVotesBp: number;
  abstainVotesBp: number;
  wallet: AnchorWallet | undefined;
  voteAccount?: PublicKey;
  consensusResult: PublicKey;
}

export interface CastVoteOverrideParams {
  proposalId: string;
  forVotesBp: number;
  againstVotesBp: number;
  abstainVotesBp: number;
  stakeAccount: string;
  wallet: AnchorWallet | undefined;
  consensusResult: PublicKey;
}

export interface ModifyVoteOverrideParams {
  proposalId: string;
  forVotesBp: number;
  againstVotesBp: number;
  abstainVotesBp: number;
  stakeAccount: string;
  wallet: AnchorWallet | undefined;
  consensusResult: PublicKey;
}

export interface SupportProposalParams {
  proposalId: string;
  wallet: AnchorWallet | undefined;
  voteAccount?: PublicKey;
}

/** GlobalConfig fields needed to derive snapshot ballot PDAs when supporting a proposal (from governance config / hook). */
export interface SupportProposalGlobalConfigInput {
  discussionEpochs: number;
  snapshotEpochExtension: number;
  snapshotSlotOffset: number;
}

export interface AddMerkleRootParams {
  proposalId: string;
  merkleRootHash: string;
  wallet: AnchorWallet | undefined;
}

export interface FinalizeProposalParams {
  proposalId: string;
  wallet: AnchorWallet | undefined;
}

// API response types (based on solgov.online API)
export interface VoteAccountProofResponse {
  meta_merkle_leaf: {
    active_stake: number;
    stake_merkle_root: string;
    vote_account: string;
    voting_wallet: string;
  };
  meta_merkle_proof: string[];
  network: string;
  snapshot_slot: number;
}

export interface StakeMerkleLeafRaw {
  active_stake: number;
  stake_account: string;
  voting_wallet: string;
}

export interface StakeMerkleLeafConverted {
  activeStake: BN;
  stakeAccount: PublicKey;
  votingWallet: PublicKey;
}

export interface StakeAccountProofResponse {
  stake_merkle_leaf: StakeMerkleLeafRaw;
  stake_merkle_proof: string[];
  network: string;
  snapshot_slot: number;
  /**
   * The validator vote account this stake was delegated to AT SNAPSHOT TIME. This is the
   * authoritative vote account for an override vote: pairing the stake proof with the live
   * on-chain delegation instead breaks for redelegated stake. Sourced from the meta leaf at
   * snapshot upload time by the verifier service.
   */
  vote_account: string;
}

export interface ChainVoteAccountData {
  activeStake: number;
  voteAccount: string;
  nodePubkey: string;
}

export interface VoterSummaryResponse {
  network: string;
  snapshot_slot: number;
  voting_wallet: string;
  stake_accounts: {
    active_stake: number;
    stake_account: string;
    vote_account: string;
  }[];
  vote_accounts: {
    activeStake: number;
    voteAccount: string;
  }[];
}

export interface NetworkMetaResponse {
  network: string;
  slot: number;
  merkle_root: string;
  snapshot_hash: string;
  created_at: string;
}

// Constants
export const BASIS_POINTS_TOTAL = 10000;
export const SVMGOV_PROGRAM_ID = new PublicKey(svmgovProgramIdl.address);
export const SNAPSHOT_PROGRAM_ID = new PublicKey(govV1idl.address);

// --- Compute budget for support_proposal -----------------------------------
//
// The handler re-tallies the whole supporter list on every call, so its cost is
// linear in `num_supporters`. Requesting a flat worst case would overshoot a
// typical call by ~10x, and priority fees price the *requested* limit rather
// than what is consumed, so the request is modelled instead.
//
// Keep in sync with the mirror in `svmgov/cli/src/constants.rs`. The program's
// `tests/support_compute_budget.rs` asserts the model covers measured cost.

/** Fixed cost before the per-supporter re-tally. Measured at 22,434 CU. */
const SUPPORT_CU_BASE = 22_500;
/** Per existing supporter: a `sol_get_epoch_stake` syscall plus loop overhead. */
const SUPPORT_CU_PER_SUPPORTER = 132;
/**
 * Extra units for the call that crosses the threshold: it activates voting and,
 * unless the ballot box exists, creates it via the `init_ballot_box` CPI.
 * Measured at ~26.8k above a non-activating call, at the 64-operator whitelist
 * maximum (init_ballot_box clones the whitelist, so a longer list costs more).
 * Always included — a caller cannot know whether its own call will cross.
 */
const SUPPORT_CU_ACTIVATION = 28_000;
/** Covers supporters landing between reading the count and executing. */
const SUPPORT_CU_HEADROOM_PERCENT = 15;
/** Per-transaction maximum a client may request. */
const MAX_COMPUTE_UNIT_LIMIT = 1_400_000;
/**
 * The program's `MAX_SUPPORTERS_LIMIT`. Used as the supporter count when the
 * real one cannot be read, so the request still covers the largest list the
 * program permits.
 */
export const MAX_SUPPORTERS = 2_000;

/**
 * Compute-unit limit to request for a support against a proposal that currently
 * has `numSupporters` supporters.
 */
export function supportComputeUnitLimit(numSupporters: number): number {
  const modelled =
    SUPPORT_CU_BASE +
    SUPPORT_CU_PER_SUPPORTER * Math.max(0, numSupporters) +
    SUPPORT_CU_ACTIVATION;
  const withHeadroom = Math.ceil(
    (modelled * (100 + SUPPORT_CU_HEADROOM_PERCENT)) / 100,
  );
  return Math.min(withHeadroom, MAX_COMPUTE_UNIT_LIMIT);
}
