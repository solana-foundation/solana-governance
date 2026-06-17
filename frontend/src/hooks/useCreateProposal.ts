import { CreateProposalParams } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
import { createProposalMutation } from "@/data";
import { useMutation } from "@tanstack/react-query";

export function useCreateProposal() {
  const { endpointUrl: endpoint, endpointType } = useEndpoint();

  return useMutation({
    mutationKey: ["create-proposal"],
    mutationFn: (params: CreateProposalParams) =>
      createProposalMutation(params, { endpoint, network: endpointType }),
  });
}
