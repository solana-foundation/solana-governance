import { address, createSolanaRpc } from "@solana/kit";
import { fetchProposal } from "@solana/svmgov";
import { useMutation } from "@tanstack/react-query";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import { requireKnownSnapshotNetwork } from "@/lib/snapshotNetwork";
import { getStakeAccountProof, getVoteAccountProof } from "@/lib/ncnProofs";
import {
  buildCastVoteInstruction,
  buildCastVoteOverrideInstruction,
  buildModifyVoteInstruction,
  buildModifyVoteOverrideInstruction,
  type VoteDistribution,
} from "@/lib/transactions";
import { useSignTransaction } from "./useSignTransaction";

type PublicKeyLike = string | { toBase58(): string };
type VoteInput = VoteDistribution & { proposalId: string; consensusResult: PublicKeyLike; publicKey?: string };
type OverrideInput = VoteInput & { stakeAccount: string };
const toAddress = (value: PublicKeyLike) => address(typeof value === "string" ? value : value.toBase58());

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
      const snapshotSlot = proposal.data.snapshotSlot;
      if (snapshotSlot <= 0n) throw new Error("Proposal has no snapshot slot; voting may not have been activated yet");
      const vote = [...voteAccounts.current, ...voteAccounts.delinquent].find((account) => account.nodePubkey === input.publicKey);
      if (!vote) throw new Error(`No SPL vote account found for validator identity ${input.publicKey}`);
      const proof = await getVoteAccountProof(vote.votePubkey, requireKnownSnapshotNetwork(network), snapshotSlot, ncnApiUrl);
      const builder = modify ? buildModifyVoteInstruction : buildCastVoteInstruction;
      const signature = await signAndSend(({ signer }) => Promise.all([
        builder({ consensusResult: toAddress(input.consensusResult), distribution: input, proposal: address(input.proposalId), signer, splVoteAccount: vote.votePubkey, voteAccount: address(proof.meta_merkle_leaf.vote_account) }),
      ]));
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
      const snapshotSlot = proposal.data.snapshotSlot;
      if (snapshotSlot <= 0n) throw new Error("Proposal has no snapshot slot; voting may not have been activated yet");
      const resolvedNetwork = requireKnownSnapshotNetwork(network);
      const stakeProof = await getStakeAccountProof(input.stakeAccount, resolvedNetwork, snapshotSlot, ncnApiUrl);
      if (!stakeProof.vote_account) throw new Error("Stake account proof is missing the snapshot vote_account");
      const metaProof = await getVoteAccountProof(stakeProof.vote_account, resolvedNetwork, snapshotSlot, ncnApiUrl);
      if (metaProof.meta_merkle_leaf.vote_account !== stakeProof.vote_account) throw new Error("Stake and vote proofs are from different snapshots");
      const builder = modify ? buildModifyVoteOverrideInstruction : buildCastVoteOverrideInstruction;
      const signature = await signAndSend(({ signer }) => Promise.all([
        builder({
          consensusResult: toAddress(input.consensusResult), distribution: input, proposal: address(input.proposalId), signer,
          stakeAccount: address(input.stakeAccount),
          stakeMerkleLeaf: { activeStake: BigInt(stakeProof.stake_merkle_leaf.active_stake), stakeAccount: address(stakeProof.stake_merkle_leaf.stake_account), votingWallet: address(stakeProof.stake_merkle_leaf.voting_wallet) },
          stakeMerkleProof: stakeProof.stake_merkle_proof.map(address), voteAccount: address(stakeProof.vote_account),
        }),
      ]));
      return { signature, success: true };
    },
  });
}

export const useCastVoteTransaction = () => useValidatorVoteTransaction(false);
export const useModifyVoteTransaction = () => useValidatorVoteTransaction(true);
export const useCastVoteOverrideTransaction = () => useOverrideVoteTransaction(false);
export const useModifyVoteOverrideTransaction = () => useOverrideVoteTransaction(true);
