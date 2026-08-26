import { address, createSolanaRpc, type Address } from "@solana/kit";
import {
  fetchMaybeMetaMerkleProof,
  findMetaMerkleProofPda,
} from "@solana/ncn-snapshot";
import { fetchProposal } from "@solana/svmgov";
import { useMutation } from "@tanstack/react-query";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import { requireKnownSnapshotNetwork } from "@/lib/snapshotNetwork";
import {
  assertOverrideProofLineage,
  getStakeAccountProof,
  getVoteAccountProof,
  requireProposalSnapshotSlot,
} from "@/lib/ncnProofs";
import {
  buildCastVoteInstruction,
  buildCastVoteOverrideInstruction,
  buildInitializeMetaMerkleProofInstruction,
  buildModifyVoteInstruction,
  buildModifyVoteOverrideInstruction,
  type VoteDistribution,
} from "@/lib/transactions";
import { useSignTransaction } from "./useSignTransaction";

type VoteInput = VoteDistribution & { proposalId: string; consensusResult: Address; publicKey?: string };
type OverrideInput = VoteInput & { stakeAccount: string };

async function computeProofCloseTimestamp(
  rpc: ReturnType<typeof createSolanaRpc>,
  endEpoch: bigint,
): Promise<bigint> {
  const epochInfo = await rpc.getEpochInfo().send();
  const epochStartSlot = epochInfo.absoluteSlot - epochInfo.slotIndex;
  const targetSlot = epochStartSlot +
    (endEpoch - epochInfo.epoch) * epochInfo.slotsInEpoch;

  let blockTime: bigint | null = null;
  let referenceSlot = epochInfo.absoluteSlot;
  for (let attempt = 0; attempt < 8 && referenceSlot >= 0n; attempt += 1) {
    try {
      blockTime = await rpc.getBlockTime(referenceSlot).send();
      if (blockTime !== null) break;
    } catch {
      // Skipped slots and pruned RPC history can both lack a block time.
    }
    referenceSlot -= 1n;
  }
  if (blockTime === null) {
    throw new Error("Failed to fetch a recent block time for the proof expiry");
  }

  const projectedSeconds = ((targetSlot - referenceSlot) * 400n) / 1_000n;
  const bufferSeconds = projectedSeconds > 0n
    ? (projectedSeconds * 20n) / 100n > 3_600n
      ? (projectedSeconds * 20n) / 100n
      : 3_600n
    : 0n;
  return blockTime + projectedSeconds + bufferSeconds;
}

async function buildMetaMerkleProofInitialization(
  rpc: ReturnType<typeof createSolanaRpc>,
  input: {
    consensusResult: ReturnType<typeof address>;
    endEpoch: bigint;
    proof: Awaited<ReturnType<typeof getVoteAccountProof>>;
    signer: Parameters<typeof buildCastVoteInstruction>[0]["signer"];
  },
) {
  const [merkleProof] = await findMetaMerkleProofPda({
    consensusResult: input.consensusResult,
    voteAccount: address(input.proof.meta_merkle_leaf.vote_account),
  });
  const existingProof = await fetchMaybeMetaMerkleProof(rpc, merkleProof);
  if (existingProof.exists) return [];

  return [await buildInitializeMetaMerkleProofInstruction({
    closeTimestamp: await computeProofCloseTimestamp(rpc, input.endEpoch),
    consensusResult: input.consensusResult,
    proof: input.proof,
    signer: input.signer,
  })];
}

function useValidatorVoteTransaction(modify: boolean) {
  const { endpointUrl, network } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();
  const { signAndSend } = useSignTransaction();
  return useMutation({
    mutationKey: [modify ? "modify-vote" : "cast-vote"],
    mutationFn: async (input: VoteInput) => {
      if (!input.publicKey) throw new Error("Wallet not connected");
      const rpc = createSolanaRpc(endpointUrl);
      const [proposal, voteAccounts] = await Promise.all([
        fetchProposal(rpc, address(input.proposalId)), rpc.getVoteAccounts().send(),
      ]);
      const snapshotSlot = requireProposalSnapshotSlot(proposal.data.snapshotSlot);
      const vote = [...voteAccounts.current, ...voteAccounts.delinquent].find((account) => account.nodePubkey === input.publicKey);
      if (!vote) throw new Error(`No SPL vote account found for validator identity ${input.publicKey}`);
      const proof = await getVoteAccountProof(vote.votePubkey, requireKnownSnapshotNetwork(network), snapshotSlot, ncnApiUrl);
      const builder = modify ? buildModifyVoteInstruction : buildCastVoteInstruction;
      const signature = await signAndSend(async ({ signer }) => {
        const consensusResult = input.consensusResult;
        const [initProofInstruction, voteInstruction] = await Promise.all([
          buildMetaMerkleProofInitialization(rpc, {
            consensusResult,
            endEpoch: proposal.data.endEpoch,
            proof,
            signer,
          }),
          builder({ consensusResult, distribution: input, proposal: address(input.proposalId), signer, splVoteAccount: vote.votePubkey, voteAccount: address(proof.meta_merkle_leaf.vote_account) }),
        ]);
        return [...initProofInstruction, voteInstruction];
      });
      return { signature, success: true };
    },
  });
}

function useOverrideVoteTransaction(modify: boolean) {
  const { endpointUrl, network } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();
  const { signAndSend } = useSignTransaction();
  return useMutation({
    mutationKey: [modify ? "modify-vote-override" : "cast-vote-override"],
    mutationFn: async (input: OverrideInput) => {
      if (!input.publicKey) throw new Error("Wallet not connected");
      const proposal = await fetchProposal(createSolanaRpc(endpointUrl), address(input.proposalId));
      const snapshotSlot = requireProposalSnapshotSlot(proposal.data.snapshotSlot);
      const resolvedNetwork = requireKnownSnapshotNetwork(network);
      const stakeProof = await getStakeAccountProof(input.stakeAccount, resolvedNetwork, snapshotSlot, ncnApiUrl);
      if (!stakeProof.vote_account) throw new Error("Stake account proof is missing the snapshot vote_account");
      const metaProof = await getVoteAccountProof(stakeProof.vote_account, resolvedNetwork, snapshotSlot, ncnApiUrl);
      assertOverrideProofLineage(stakeProof, metaProof);
      const builder = modify ? buildModifyVoteOverrideInstruction : buildCastVoteOverrideInstruction;
      const signature = await signAndSend(async ({ signer }) => {
        const consensusResult = input.consensusResult;
        const rpc = createSolanaRpc(endpointUrl);
        const [initProofInstruction, voteInstruction] = await Promise.all([
          buildMetaMerkleProofInitialization(rpc, {
            consensusResult,
            endEpoch: proposal.data.endEpoch,
            proof: metaProof,
            signer,
          }),
          builder({
            consensusResult, distribution: input, proposal: address(input.proposalId), signer,
            stakeAccount: address(input.stakeAccount),
            stakeMerkleLeaf: { activeStake: BigInt(stakeProof.stake_merkle_leaf.active_stake), stakeAccount: address(stakeProof.stake_merkle_leaf.stake_account), votingWallet: address(stakeProof.stake_merkle_leaf.voting_wallet) },
            stakeMerkleProof: stakeProof.stake_merkle_proof.map(address), voteAccount: address(stakeProof.vote_account),
          }),
        ]);
        return [...initProofInstruction, voteInstruction];
      });
      return { signature, success: true };
    },
  });
}

export const useCastVoteTransaction = () => useValidatorVoteTransaction(false);
export const useModifyVoteTransaction = () => useValidatorVoteTransaction(true);
export const useCastVoteOverrideTransaction = () => useOverrideVoteTransaction(false);
export const useModifyVoteOverrideTransaction = () => useOverrideVoteTransaction(true);
