import { address, createSolanaRpc } from "@solana/kit";
import { useMutation } from "@tanstack/react-query";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useGovernanceConfigContext } from "@/contexts/GovernanceConfigContext";
import { buildSupportProposalInstruction } from "@/lib/transactions";
import { useSignTransaction } from "./useSignTransaction";

function firstSlotOfEpoch(schedule: { firstNormalSlot: bigint; firstNormalEpoch: bigint; slotsPerEpoch: bigint }, epoch: bigint) {
  return schedule.firstNormalSlot + (epoch - schedule.firstNormalEpoch) * schedule.slotsPerEpoch;
}

export function useSupportProposal(_userPubKey: string | undefined) {
  const { endpointUrl } = useEndpoint();
  const configQuery = useGovernanceConfigContext();
  const { signAndSend } = useSignTransaction();
  return useMutation({
    mutationKey: ["support-proposal", configQuery.dataUpdatedAt],
    mutationFn: async ({ proposalId, publicKey }: { proposalId: string; publicKey?: string }) => {
      if (!publicKey) throw new Error("Wallet not connected");
      const config = configQuery.data;
      if (!config) throw new Error("Governance config not loaded");
      const rpc = createSolanaRpc(endpointUrl);
      const [votes, epochInfo, epochSchedule] = await Promise.all([
        rpc.getVoteAccounts().send(), rpc.getEpochInfo().send(), rpc.getEpochSchedule().send(),
      ]);
      const vote = [...votes.current, ...votes.delinquent].find((account) => account.nodePubkey === publicKey);
      if (!vote) throw new Error(`No SPL vote account found for validator identity ${publicKey}`);
      const targetEpoch = epochInfo.epoch + BigInt(config.discussionEpochs) + BigInt(config.snapshotEpochExtension);
      const snapshotSlot = firstSlotOfEpoch(epochSchedule, targetEpoch) + BigInt(config.snapshotSlotOffset);
      const signature = await signAndSend(({ signer }) => Promise.all([
        buildSupportProposalInstruction({ proposal: address(proposalId), signer, snapshotSlot, splVoteAccount: vote.votePubkey }),
      ]));
      return { signature, success: true };
    },
  });
}
