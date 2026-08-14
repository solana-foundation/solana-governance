import { CastVoteOverrideParams } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import { castVoteOverrideMutation } from "@/data";
import { useMutation } from "@tanstack/react-query";
import {
  SNAPSHOT_UNAVAILABLE_MESSAGE,
  useSnapshotMeta,
} from "./useSnapshotMeta";
import { track } from "@vercel/analytics";

export function useCastVoteOverride() {
  const { endpointUrl: endpoint, endpointType } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();
  const { data: meta } = useSnapshotMeta();

  return useMutation({
    mutationKey: ["cast-vote-override"],
    mutationFn: (params: CastVoteOverrideParams) => {
      if (meta?.slot === undefined) {
        throw new Error(SNAPSHOT_UNAVAILABLE_MESSAGE);
      }

      return castVoteOverrideMutation(
        params,
        {
          endpoint,
          network: endpointType,
          ncnApiUrl,
        },
        meta.slot
      );
    },
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
