import { ModifyVoteParams } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import { modifyVoteMutation } from "@/data";
import { requireKnownSnapshotNetwork } from "@/lib/snapshotNetwork";
import { useMutation } from "@tanstack/react-query";
import { track } from "@vercel/analytics";

export function useModifyVote() {
  const { endpointUrl: endpoint, network } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();

  return useMutation({
    mutationKey: ["modify-vote"],
    mutationFn: (params: ModifyVoteParams) =>
      modifyVoteMutation(params, {
        endpoint,
        network: requireKnownSnapshotNetwork(network),
        ncnApiUrl,
      }),
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
