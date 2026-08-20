import { CastVoteOverrideParams } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import { castVoteOverrideMutation } from "@/data";
import { requireKnownSnapshotNetwork } from "@/lib/snapshotNetwork";
import { useMutation } from "@tanstack/react-query";
import { track } from "@vercel/analytics";

export function useCastVoteOverride() {
  const { endpointUrl: endpoint, network } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();

  return useMutation({
    mutationKey: ["cast-vote-override"],
    mutationFn: (params: CastVoteOverrideParams) =>
      castVoteOverrideMutation(params, {
        endpoint,
        network: requireKnownSnapshotNetwork(network),
        ncnApiUrl,
      }),
    onMutate: (params) => {
      track("Cast Vote Override init", { proposalId: params.proposalId });
    },
    onSuccess: (_data: unknown, params) => {
      track("Cast Vote Override success", { proposalId: params.proposalId });
    },
    onError: (error: Error, params) => {
      track("Cast Vote Override error", {
        proposalId: params.proposalId,
        error: error.name,
      });
    },
  });
}
