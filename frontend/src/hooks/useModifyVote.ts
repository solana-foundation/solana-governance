import { ModifyVoteParams } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import { modifyVoteMutation } from "@/data";
import { useMutation } from "@tanstack/react-query";
import { useSnapshotMeta } from "./useSnapshotMeta";
import { track } from "@vercel/analytics";

export function useModifyVote() {
  const { endpointUrl: endpoint, endpointType } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();

  const { data: meta } = useSnapshotMeta();

  return useMutation({
    mutationKey: ["modify-vote"],
    mutationFn: (params: ModifyVoteParams) =>
      modifyVoteMutation(
        params,
        { endpoint, network: endpointType, ncnApiUrl },
        meta?.slot
      ),
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
