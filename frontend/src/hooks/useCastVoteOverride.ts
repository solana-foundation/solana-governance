import { CastVoteOverrideParams } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import { castVoteOverrideMutation } from "@/data";
import { useMutation } from "@tanstack/react-query";
import { useSnapshotMeta } from "./useSnapshotMeta";
import { track } from "@vercel/analytics";

export function useCastVoteOverride() {
  const { endpointUrl: endpoint, endpointType } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();
  const { data: meta } = useSnapshotMeta();

  return useMutation({
    mutationKey: ["cast-vote-override"],
    mutationFn: (params: CastVoteOverrideParams) =>
      castVoteOverrideMutation(
        params,
        {
          endpoint,
          network: endpointType,
          ncnApiUrl,
        },
        meta?.slot
      ),
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
