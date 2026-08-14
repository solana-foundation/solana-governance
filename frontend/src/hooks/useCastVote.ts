import { CastVoteParams } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import { castVoteMutation } from "@/data";
import { useMutation } from "@tanstack/react-query";
import {
  SNAPSHOT_UNAVAILABLE_MESSAGE,
  useSnapshotMeta,
} from "./useSnapshotMeta";
import { track } from "@vercel/analytics";

export function useCastVote() {
  const { endpointUrl: endpoint, endpointType } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();
  const { data: meta } = useSnapshotMeta();

  return useMutation({
    mutationKey: ["cast-vote"],
    mutationFn: (params: CastVoteParams) => {
      if (meta?.slot === undefined) {
        throw new Error(SNAPSHOT_UNAVAILABLE_MESSAGE);
      }

      return castVoteMutation(
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
      track("Cast Vote init", { proposalId: params.proposalId });
    },
    onSuccess: (_data: unknown, params) => {
      track("Cast Vote success", { proposalId: params.proposalId });
    },
    onError: (error: Error, params) => {
      track("Cast Vote error", {
        proposalId: params.proposalId,
        error: error.name,
      });
    },
  });
}
