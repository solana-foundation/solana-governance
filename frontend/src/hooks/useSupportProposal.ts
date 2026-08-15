import { SupportProposalParams } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useGovernanceConfigContext } from "@/contexts/GovernanceConfigContext";
import { supportProposalMutation } from "@/data";
import {
  SNAPSHOT_UNAVAILABLE_MESSAGE,
  requireKnownSnapshotNetwork,
} from "@/lib/snapshotNetwork";
import { useMutation } from "@tanstack/react-query";
import { useSnapshotMeta } from "./useSnapshotMeta";
import { useChainVoteAccount } from "./useChainVoteAccount";

export function useSupportProposal(userPubKey: string | undefined) {
  const { endpointUrl: endpoint, network } = useEndpoint();
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
          network: requireKnownSnapshotNetwork(network),
        },
        meta.slot,
        chainVoteAccount || undefined,
        governanceConfig,
      );
    },
  });
}
