import { UpdateProposalDescriptionParams } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
import { updateProposalDescriptionMutation } from "@/data";
import { useMutation } from "@tanstack/react-query";

/** Creates a mutation bound to the currently selected RPC endpoint. */
export function useUpdateProposalDescription() {
  const { endpointUrl: endpoint, network } = useEndpoint();
  return useMutation({
    mutationKey: ["update-proposal-description"],
    mutationFn: (params: UpdateProposalDescriptionParams) =>
      updateProposalDescriptionMutation(params, { endpoint, network }),
  });
}
