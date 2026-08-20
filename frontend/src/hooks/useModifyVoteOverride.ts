import { CastVoteOverrideParams } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import { modifyVoteOverrideMutation } from "@/data";
import { requireKnownSnapshotNetwork } from "@/lib/snapshotNetwork";
import { useMutation } from "@tanstack/react-query";
import { track } from "@vercel/analytics";

export function useModifyVoteOverride() {
  const { endpointUrl: endpoint, network } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();

  return useMutation({
    mutationKey: ["modify-vote-override"],
    mutationFn: (params: CastVoteOverrideParams) =>
      modifyVoteOverrideMutation(params, {
        endpoint,
        network: requireKnownSnapshotNetwork(network),
        ncnApiUrl,
      }),
    onMutate: (params) => {
      track("Modify Vote Override init", { proposalId: params.proposalId });
    },
    onSuccess: (_data: unknown, params) => {
      track("Modify Vote Override success", { proposalId: params.proposalId });
    },
    onError: (error: Error, params) => {
      track("Modify Vote Override error", {
        proposalId: params.proposalId,
        error: error.name,
      });
    },
  });
}
