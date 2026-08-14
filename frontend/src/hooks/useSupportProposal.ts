import { SupportProposalParams } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useGovernanceConfigContext } from "@/contexts/GovernanceConfigContext";
import { supportProposalMutation } from "@/data";
import { useMutation } from "@tanstack/react-query";
import {
  SNAPSHOT_UNAVAILABLE_MESSAGE,
  useSnapshotMeta,
} from "./useSnapshotMeta";
import { useChainVoteAccount } from "./useChainVoteAccount";

export function useSupportProposal(userPubKey: string | undefined) {
  const { endpointUrl: endpoint, endpointType } = useEndpoint();
  const governanceConfigQuery = useGovernanceConfigContext();
  const { data: meta } = useSnapshotMeta();
  const { data: chainVoteAccount } = useChainVoteAccount(userPubKey);

  return useMutation({
    mutationKey: [
      "support-proposal",
      chainVoteAccount,
      governanceConfigQuery.dataUpdatedAt,
    ],
    mutationFn: (params: SupportProposalParams) => {
      const governanceConfig = governanceConfigQuery.data;
      if (!governanceConfig) {
        throw new Error("Governance config not loaded");
      }
      if (meta?.slot === undefined) {
        throw new Error(SNAPSHOT_UNAVAILABLE_MESSAGE);
      }
      return supportProposalMutation(
        params,
        {
          endpoint,
          network: endpointType,
        },
        meta.slot,
        chainVoteAccount || undefined,
        governanceConfig,
      );
    },
  });
}
