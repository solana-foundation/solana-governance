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
  voteAccount: string;
  consensusResult: PublicKey;
}

export interface ModifyVoteOverrideParams {
  proposalId: string;
  forVotesBp: number;
  againstVotesBp: number;
  abstainVotesBp: number;
  stakeAccount: string;
  wallet: AnchorWallet | undefined;
  voteAccount: string;
  consensusResult: PublicKey;
}

export interface SupportProposalParams {
  proposalId: string;
  wallet: AnchorWallet | undefined;
  voteAccount?: PublicKey;
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
