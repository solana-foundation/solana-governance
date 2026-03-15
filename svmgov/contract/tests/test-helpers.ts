import * as anchor from "@coral-xyz/anchor";
import { Govcontract } from "../target/types/govcontract";
import { MockGovV1 } from "../target/types/mock_gov_v1";
import { BALLOT_ID, MERKLE_ROOT_HASH } from "./test-constants";

// Account derivation helpers
export function deriveProposalIndexAccount(program: anchor.Program<Govcontract>): anchor.web3.PublicKey {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("index")],
    program.programId
  )[0];
}

export function deriveProposalAccount(
  program: anchor.Program<Govcontract>,
  seed: anchor.BN,
  splVoteAccount: anchor.web3.PublicKey
): anchor.web3.PublicKey {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("proposal"),
      seed.toArrayLike(Buffer, "le", 8),
      splVoteAccount.toBuffer(),
    ],
    program.programId
  )[0];
}

export function deriveSupportAccount(
  program: anchor.Program<Govcontract>,
  proposalAccount: anchor.web3.PublicKey,
  splVoteAccount: anchor.web3.PublicKey
): anchor.web3.PublicKey {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("support"),
      proposalAccount.toBuffer(),
      splVoteAccount.toBuffer(),
    ],
    program.programId
  )[0];
}

export function deriveVoteAccount(
  program: anchor.Program<Govcontract>,
  proposalAccount: anchor.web3.PublicKey,
  splVoteAccount: anchor.web3.PublicKey
): anchor.web3.PublicKey {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("vote"),
      proposalAccount.toBuffer(),
      splVoteAccount.toBuffer(),
    ],
    program.programId
  )[0];
}

export function deriveConsensusResultAccount(mockProgram: anchor.Program<MockGovV1>): anchor.web3.PublicKey {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("ConsensusResult"),
      BALLOT_ID.toArrayLike(Buffer, "le", 8),
    ],
    mockProgram.programId
  )[0];
}

export function deriveMetaMerkleProofAccount(
  mockProgram: anchor.Program<MockGovV1>,
  consensusResult: anchor.web3.PublicKey,
  splVoteAccount: anchor.web3.PublicKey
): anchor.web3.PublicKey {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("MetaMerkleProof"),
      consensusResult.toBuffer(),
      splVoteAccount.toBuffer(),
    ],
    mockProgram.programId
  )[0];
}

export function deriveVoteOverrideAccount(
  program: anchor.Program<Govcontract>,
  proposalAccount: anchor.web3.PublicKey,
  stakeAccount: anchor.web3.PublicKey,
  validatorVote: anchor.web3.PublicKey
): anchor.web3.PublicKey {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("vote_override"),
      proposalAccount.toBuffer(),
      stakeAccount.toBuffer(),
      validatorVote.toBuffer(),
    ],
    program.programId
  )[0];
}

export function deriveVoteOverrideCacheAccount(
  program: anchor.Program<Govcontract>,
  proposalAccount: anchor.web3.PublicKey,
  validatorVote: anchor.web3.PublicKey
): anchor.web3.PublicKey {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("vote_override_cache"),
      proposalAccount.toBuffer(),
      validatorVote.toBuffer(),
    ],
    program.programId
  )[0];
}

// Event listener helpers
export function createEventListener<T>(
  program: anchor.Program<Govcontract>,
  eventName: string,
  callback: (event: T, slot: number) => void
): number {
  return program.addEventListener(eventName, callback);
}

export function removeEventListener(
  program: anchor.Program<Govcontract>,
  listenerId: number
): void {
  program.removeEventListener(listenerId);
}

// Proposal display helper
export function logProposalState(proposal: any, prefix = ""): void {
  console.log(`${prefix}Proposal State:`);
  console.log(`- Cluster support lamports: ${proposal.clusterSupportLamports.toString()} (${Number(proposal.clusterSupportLamports) / anchor.web3.LAMPORTS_PER_SOL} SOL)`);
  console.log(`- For votes lamports: ${proposal.forVotesLamports.toString()} (${Number(proposal.forVotesLamports) / anchor.web3.LAMPORTS_PER_SOL} SOL)`);
  console.log(`- Against votes lamports: ${proposal.againstVotesLamports.toString()} (${Number(proposal.againstVotesLamports) / anchor.web3.LAMPORTS_PER_SOL} SOL)`);
  console.log(`- Abstain votes lamports: ${proposal.abstainVotesLamports.toString()} (${Number(proposal.abstainVotesLamports) / anchor.web3.LAMPORTS_PER_SOL} SOL)`);
  console.log(`- Total vote count: ${proposal.voteCount.toString()}`);
  console.log(`- Voting active: ${proposal.voting}`);
}

// Vote account display helper
export function logVoteState(vote: any, prefix = ""): void {
  console.log(`${prefix}Vote Account State:`);
  console.log(`- For votes lamports: ${vote.forVotesLamports.toString()} (${Number(vote.forVotesLamports) / anchor.web3.LAMPORTS_PER_SOL} SOL)`);
  console.log(`- Against votes lamports: ${vote.againstVotesLamports.toString()} (${Number(vote.againstVotesLamports) / anchor.web3.LAMPORTS_PER_SOL} SOL)`);
  console.log(`- Abstain votes lamports: ${vote.abstainVotesLamports.toString()} (${Number(vote.abstainVotesLamports) / anchor.web3.LAMPORTS_PER_SOL} SOL)`);
  console.log(`- Override lamports: ${vote.overrideLamports.toString()} (${Number(vote.overrideLamports) / anchor.web3.LAMPORTS_PER_SOL} SOL)`);
  console.log(`- Total stake: ${vote.stake.toString()} (${Number(vote.stake) / anchor.web3.LAMPORTS_PER_SOL} SOL)`);
}

// Event validation helper (inlined in tests now)
