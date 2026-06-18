import { CastVoteParams } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import { castVoteMutation } from "@/data";
import { useMutation } from "@tanstack/react-query";
import { useSnapshotMeta } from "./useSnapshotMeta";
import { track } from "@vercel/analytics";

export function useCastVote() {
  const { endpointUrl: endpoint, endpointType } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();
  const { data: meta } = useSnapshotMeta();

  return useMutation({
    mutationKey: ["cast-vote"],
    mutationFn: (params: CastVoteParams) =>
      castVoteMutation(
        params,
        {
          endpoint,
          network: endpointType,
          ncnApiUrl,
        },
        meta?.slot
      ),
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
