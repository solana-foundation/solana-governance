import { CastVoteParams } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import { castVoteMutation } from "@/data";
import { resolveSnapshotNetwork } from "@/lib/snapshotNetwork";
import { useMutation } from "@tanstack/react-query";
import { track } from "@vercel/analytics";

export function useCastVote() {
  const { endpointUrl: endpoint, endpointType } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();

  return useMutation({
    mutationKey: ["cast-vote"],
    mutationFn: async (params: CastVoteParams) => {
      const network = await resolveSnapshotNetwork(endpointType, endpoint);

      return castVoteMutation(params, {
        endpoint,
        network,
        ncnApiUrl,
      });
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
