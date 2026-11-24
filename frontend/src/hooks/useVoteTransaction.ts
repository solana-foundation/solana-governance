import { address, createSolanaRpc, type Address } from "@solana/kit";
import {
  fetchMaybeMetaMerkleProof,
  findMetaMerkleProofPda,
} from "@solana/ncn-snapshot";
import { fetchProposal } from "@solana/svmgov";
import { useMutation } from "@tanstack/react-query";
import { track } from "@vercel/analytics";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import {
  computeProofCloseTimestamp,
  ensureMetaMerkleProofInitialized,
} from "@/lib/metaMerkleProofSetup";
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
import { useResolveSignerVoteAccount } from "./useResolveSignerVoteAccount";
import { withReadableErrors } from "@/lib/transactionErrors";
import { useSignTransaction } from "./useSignTransaction";

type VoteInput = VoteDistribution & { proposalId: string; consensusResult: Address; publicKey?: string };
type OverrideInput = VoteInput & { stakeAccount: string };

/** Funnel events for one vote action, named `<action> init|success|error`. */
function voteAnalytics(action: string) {
  return {
    onMutate: ({ proposalId }: VoteInput) => {
      track(`${action} init`, { proposalId });
    },
    onSuccess: (_data: unknown, { proposalId }: VoteInput) => {
      track(`${action} success`, { proposalId });
    },
    onError: (error: Error, { proposalId }: VoteInput) => {
      track(`${action} error`, { proposalId, error: error.name });
    },
  };
}

type MetaMerkleProofInitialization = {
  consensusResult: Address;
  endEpoch: bigint;
  proof: Awaited<ReturnType<typeof getVoteAccountProof>>;
  signer: Parameters<typeof buildCastVoteInstruction>[0]["signer"];
};

async function buildMetaMerkleProofInitialization(
  rpc: ReturnType<typeof createSolanaRpc>,
  input: MetaMerkleProofInitialization,
) {
  return buildInitializeMetaMerkleProofInstruction({
    closeTimestamp: await computeProofCloseTimestamp(rpc, input.endEpoch),
    consensusResult: input.consensusResult,
    proof: input.proof,
    signer: input.signer,
  });
}

/** The proof initialization, or nothing when the proof account already exists. */
async function buildMissingMetaMerkleProofInitialization(
  rpc: ReturnType<typeof createSolanaRpc>,
  input: MetaMerkleProofInitialization,
) {
  const [merkleProof] = await findMetaMerkleProofPda({
    consensusResult: input.consensusResult,
    voteAccount: address(input.proof.meta_merkle_leaf.vote_account),
  });
  const existingProof = await fetchMaybeMetaMerkleProof(rpc, merkleProof);
  return existingProof.exists
    ? []
    : [await buildMetaMerkleProofInitialization(rpc, input)];
}

function useValidatorVoteTransaction(modify: boolean) {
  const { endpointUrl, network } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();
  const { signAndSend } = useSignTransaction();
  const resolveSignerVoteAccount = useResolveSignerVoteAccount();
  return useMutation({
    mutationKey: [modify ? "modify-vote" : "cast-vote"],
    ...voteAnalytics(modify ? "Modify Vote" : "Cast Vote"),
    mutationFn: withReadableErrors(async (input: VoteInput) => {
      if (!input.publicKey) throw new Error("Wallet not connected");
      const rpc = createSolanaRpc(endpointUrl);
      const [proposal, splVoteAccount] = await Promise.all([
        fetchProposal(rpc, address(input.proposalId)),
        // The wallet is either the validator identity or its accepted vote delegate.
        resolveSignerVoteAccount(input.publicKey),
      ]);
      const snapshotSlot = requireProposalSnapshotSlot(proposal.data.snapshotSlot);
      const proof = await getVoteAccountProof(splVoteAccount, network, snapshotSlot, ncnApiUrl);
      const builder = modify ? buildModifyVoteInstruction : buildCastVoteInstruction;
      const signature = await signAndSend(async ({ signer }) => {
        const consensusResult = input.consensusResult;
        const [initProofInstruction, voteInstruction] = await Promise.all([
          buildMissingMetaMerkleProofInitialization(rpc, {
            consensusResult,
            endEpoch: proposal.data.endEpoch,
            proof,
            signer,
          }),
          builder({ consensusResult, distribution: input, proposal: address(input.proposalId), signer, splVoteAccount, voteAccount: address(proof.meta_merkle_leaf.vote_account) }),
        ]);
        return [...initProofInstruction, voteInstruction];
      });
      return { signature, success: true };
    }),
  });
}

function useOverrideVoteTransaction(modify: boolean) {
  const { endpointUrl, network } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();
  const { signAndSend } = useSignTransaction();
  return useMutation({
    mutationKey: [modify ? "modify-vote-override" : "cast-vote-override"],
    ...voteAnalytics(modify ? "Modify Vote Override" : "Cast Vote Override"),
    mutationFn: withReadableErrors(async (input: OverrideInput) => {
      if (!input.publicKey) throw new Error("Wallet not connected");
      const rpc = createSolanaRpc(endpointUrl);
      const proposal = await fetchProposal(rpc, address(input.proposalId));
      const snapshotSlot = requireProposalSnapshotSlot(proposal.data.snapshotSlot);
      const stakeProof = await getStakeAccountProof(input.stakeAccount, network, snapshotSlot, ncnApiUrl);
      if (!stakeProof.vote_account) throw new Error("Stake account proof is missing the snapshot vote_account");
      const metaProof = await getVoteAccountProof(stakeProof.vote_account, network, snapshotSlot, ncnApiUrl);
      assertOverrideProofLineage(stakeProof, metaProof);
      const builder = modify ? buildModifyVoteOverrideInstruction : buildCastVoteOverrideInstruction;
      const consensusResult = input.consensusResult;

      // The meta proof and the stake proof together exceed the 1,232-byte
      // transaction limit, so the shared meta proof is confirmed on its own
      // first. Its close timestamp is the proposal's vote expiry, so it cannot
      // be closed between the two transactions.
      const [metaMerkleProof] = await findMetaMerkleProofPda({
        consensusResult,
        voteAccount: address(metaProof.meta_merkle_leaf.vote_account),
      });
      await ensureMetaMerkleProofInitialized({
        isInitialized: async () =>
          (await fetchMaybeMetaMerkleProof(rpc, metaMerkleProof)).exists,
        initialize: () =>
          signAndSend(async ({ signer }) => [
            await buildMetaMerkleProofInitialization(rpc, {
              consensusResult,
              endEpoch: proposal.data.endEpoch,
              proof: metaProof,
              signer,
            }),
          ]),
      });

      const signature = await signAndSend(async ({ signer }) => [
        await builder({
          consensusResult, distribution: input, proposal: address(input.proposalId), signer,
          stakeAccount: address(input.stakeAccount),
          stakeMerkleLeaf: { activeStake: BigInt(stakeProof.stake_merkle_leaf.active_stake), stakeAccount: address(stakeProof.stake_merkle_leaf.stake_account), votingWallet: address(stakeProof.stake_merkle_leaf.voting_wallet) },
          stakeMerkleProof: stakeProof.stake_merkle_proof.map(address), voteAccount: address(stakeProof.vote_account),
        }),
      ]);
      return { signature, success: true };
    }),
  });
}

export const useCastVoteTransaction = () => useValidatorVoteTransaction(false);
export const useModifyVoteTransaction = () => useValidatorVoteTransaction(true);
export const useCastVoteOverrideTransaction = () => useOverrideVoteTransaction(false);
export const useModifyVoteOverrideTransaction = () => useOverrideVoteTransaction(true);
