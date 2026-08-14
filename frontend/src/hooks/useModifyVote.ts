import { ModifyVoteParams } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import { modifyVoteMutation } from "@/data";
import { resolveSnapshotNetwork } from "@/lib/snapshotNetwork";
import { useMutation } from "@tanstack/react-query";
import { track } from "@vercel/analytics";

export function useModifyVote() {
  const { endpointUrl: endpoint, endpointType } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();

  return useMutation({
    mutationKey: ["modify-vote"],
    mutationFn: async (params: ModifyVoteParams) => {
      const network = await resolveSnapshotNetwork(endpointType, endpoint);

      return modifyVoteMutation(params, {
        endpoint,
        network,
        ncnApiUrl,
      });
    },
    onMutate: (params) => {
      track("Modify Vote init", { proposalId: params.proposalId });
    },
    onSuccess: (_data: unknown, params) => {
      track("Modify Vote success", { proposalId: params.proposalId });
    },
    onError: (error: Error, params) => {
      track("Modify Vote error", {
        proposalId: params.proposalId,
        error: error.name,
      });
    },
  });
}
