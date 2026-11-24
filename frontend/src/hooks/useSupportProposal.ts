import { address, createSolanaRpc } from "@solana/kit";
import { fetchProposal } from "@solana/svmgov";
import { useMutation } from "@tanstack/react-query";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useGovernanceConfigContext } from "@/contexts/GovernanceConfigContext";
import { resolveSignerVoteAccount } from "@/lib/governance/voteDelegations";
import {
  buildSetComputeUnitLimitInstruction,
  MAX_SUPPORTERS,
  supportComputeUnitLimit,
} from "@/lib/supportComputeBudget";
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
      const [splVoteAccount, epochInfo, epochSchedule, numSupporters] = await Promise.all([
        // The wallet is either the validator identity or its accepted vote delegate.
        resolveSignerVoteAccount(rpc, address(publicKey)), rpc.getEpochInfo().send(), rpc.getEpochSchedule().send(),
        // The handler re-tallies every existing supporter, so the compute
        // budget is sized from the current list length. A failed read must not
        // block a support that could still succeed, so fall back to the
        // program's supporter cap.
        fetchProposal(rpc, address(proposalId)).then(
          (proposal) => proposal.data.numSupporters,
          (error) => {
            console.warn(
              `Could not read the supporter count for ${proposalId}; requesting the compute budget for a full ${MAX_SUPPORTERS}-supporter list`,
              error,
            );
            return MAX_SUPPORTERS;
          },
        ),
      ]);
      const targetEpoch = epochInfo.epoch + BigInt(config.discussionEpochs) + BigInt(config.snapshotEpochExtension);
      const snapshotSlot = firstSlotOfEpoch(epochSchedule, targetEpoch) + BigInt(config.snapshotSlotOffset);
      const signature = await signAndSend(async ({ signer }) => [
        buildSetComputeUnitLimitInstruction(supportComputeUnitLimit(numSupporters)),
        await buildSupportProposalInstruction({ proposal: address(proposalId), signer, snapshotSlot, splVoteAccount }),
      ]);
      return { signature, success: true };
    },
  });
}
