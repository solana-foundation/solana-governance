import { CastVoteParams } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import { castVoteMutation } from "@/data";
import { requireKnownSnapshotNetwork } from "@/lib/snapshotNetwork";
import { useMutation } from "@tanstack/react-query";
import { track } from "@vercel/analytics";

export function useCastVote() {
  const { endpointUrl: endpoint, network } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();

  return useMutation({
    mutationKey: ["cast-vote"],
    mutationFn: (params: CastVoteParams) =>
      castVoteMutation(params, {
        endpoint,
        network: requireKnownSnapshotNetwork(network),
        ncnApiUrl,
      }),
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
