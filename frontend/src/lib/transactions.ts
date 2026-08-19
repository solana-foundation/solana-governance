import { getAddressEncoder, type Address, type TransactionModifyingSigner } from "@solana/kit";
import {
  findBallotBoxPda,
  findMetaMerkleProofPda,
  NCN_SNAPSHOT_PROGRAM_ADDRESS,
  type StakeMerkleLeaf,
} from "@solana/ncn-snapshot";
import {
  getCastVoteInstructionAsync,
  getCastVoteOverrideInstructionAsync,
  getCreateProposalInstructionAsync,
  getFinalizeProposalInstruction,
  getModifyVoteInstructionAsync,
  getModifyVoteOverrideInstructionAsync,
  getSupportProposalInstructionAsync,
} from "@solana/svmgov";

export type VoteDistribution = { forVotesBp: number; againstVotesBp: number; abstainVotesBp: number };
const basisPoints = (distribution: VoteDistribution) => ({
  forVotesBp: BigInt(distribution.forVotesBp),
  againstVotesBp: BigInt(distribution.againstVotesBp),
  abstainVotesBp: BigInt(distribution.abstainVotesBp),
});

export async function buildCreateProposalInstruction(input: {
  description: string; signer: TransactionModifyingSigner; splVoteAccount: Address; title: string;
}) {
  return getCreateProposalInstructionAsync({ ...input, seed: crypto.getRandomValues(new BigUint64Array(1))[0] });
}

export function buildFinalizeProposalInstruction(input: { proposal: Address; signer: TransactionModifyingSigner }) {
  return getFinalizeProposalInstruction(input);
}

export async function buildSupportProposalInstruction(input: {
  proposal: Address; signer: TransactionModifyingSigner; snapshotSlot: bigint; splVoteAccount: Address;
}) {
  const [ballotBox] = await findBallotBoxPda({ snapshotSlot: input.snapshotSlot });
  return getSupportProposalInstructionAsync({
    ballotBox,
    ballotProgram: NCN_SNAPSHOT_PROGRAM_ADDRESS,
    proposal: input.proposal,
    signer: input.signer,
    splVoteAccount: input.splVoteAccount,
  });
}

export async function buildCastVoteInstruction(input: {
  consensusResult: Address; distribution: VoteDistribution; proposal: Address; signer: TransactionModifyingSigner;
  splVoteAccount: Address; voteAccount: Address;
}) {
  const [metaMerkleProof] = await findMetaMerkleProofPda({ consensusResult: input.consensusResult, voteAccount: input.voteAccount });
  return getCastVoteInstructionAsync({
    ...basisPoints(input.distribution), consensusResult: input.consensusResult, metaMerkleProof,
    proposal: input.proposal, signer: input.signer, snapshotProgram: NCN_SNAPSHOT_PROGRAM_ADDRESS,
    splVoteAccount: input.splVoteAccount,
  });
}

export async function buildModifyVoteInstruction(input: {
  consensusResult: Address; distribution: VoteDistribution; proposal: Address; signer: TransactionModifyingSigner;
  splVoteAccount: Address; voteAccount: Address;
}) {
  const [metaMerkleProof] = await findMetaMerkleProofPda({ consensusResult: input.consensusResult, voteAccount: input.voteAccount });
  return getModifyVoteInstructionAsync({
    ...basisPoints(input.distribution), consensusResult: input.consensusResult, metaMerkleProof,
    proposal: input.proposal, signer: input.signer, snapshotProgram: NCN_SNAPSHOT_PROGRAM_ADDRESS,
    splVoteAccount: input.splVoteAccount,
  });
}

async function buildOverride(
  modify: boolean,
  input: {
    consensusResult: Address; distribution: VoteDistribution; proposal: Address; signer: TransactionModifyingSigner;
    stakeAccount: Address; stakeMerkleLeaf: StakeMerkleLeaf; stakeMerkleProof: readonly Address[]; voteAccount: Address;
  },
) {
  const [metaMerkleProof] = await findMetaMerkleProofPda({ consensusResult: input.consensusResult, voteAccount: input.voteAccount });
  const args = {
    ...basisPoints(input.distribution), consensusResult: input.consensusResult, metaMerkleProof,
    proposal: input.proposal, signer: input.signer, snapshotProgram: NCN_SNAPSHOT_PROGRAM_ADDRESS,
    splStakeAccount: input.stakeAccount, splVoteAccount: input.voteAccount,
    stakeMerkleLeaf: input.stakeMerkleLeaf,
    stakeMerkleProof: input.stakeMerkleProof.map((node) => getAddressEncoder().encode(node)),
  };
  return modify ? getModifyVoteOverrideInstructionAsync(args) : getCastVoteOverrideInstructionAsync(args);
}

export function buildCastVoteOverrideInstruction(input: Parameters<typeof buildOverride>[1]) { return buildOverride(false, input); }
export function buildModifyVoteOverrideInstruction(input: Parameters<typeof buildOverride>[1]) { return buildOverride(true, input); }
