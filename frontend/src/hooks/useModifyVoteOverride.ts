import { CastVoteOverrideParams } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import { modifyVoteOverrideMutation } from "@/data";
import { useMutation } from "@tanstack/react-query";
import { useSnapshotMeta } from "./useSnapshotMeta";
import { track } from "@vercel/analytics";

export function useModifyVoteOverride() {
  const { endpointUrl: endpoint, endpointType } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();

  const { data: meta } = useSnapshotMeta();
  return useMutation({
    mutationKey: ["modify-vote-override"],
    mutationFn: (params: CastVoteOverrideParams) =>
      modifyVoteOverrideMutation(
        params,
        {
          endpoint,
          network: endpointType,
          ncnApiUrl,
        },
        meta?.slot
      ),
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
