import { useEndpoint } from "@/contexts/EndpointContext";
import { getValidatorProposalVoteAccount } from "@/data/getValidatorProposalVoteAccount";
import { GET_VALIDATOR_PROPOSAL_VOTE_ACCOUNTS } from "@/helpers";
import { useConnector } from "@solana/connector/react";
import { useQuery } from "@tanstack/react-query";

export const useValidatorProposalVoteAccount = (
  proposalId: string | undefined,
  enabled = true
) => {
  const { endpointUrl: endpoint } = useEndpoint();

  const { account, isConnected: connected } = useConnector();
  const publicKey = account ?? undefined;

  const enabledQuery = connected && !!publicKey && !!proposalId && enabled;

  return useQuery({
    queryKey: [
      GET_VALIDATOR_PROPOSAL_VOTE_ACCOUNTS,
      endpoint,
      proposalId,
      publicKey,
    ],
    enabled: enabledQuery,
    staleTime: 1000 * 120, // 2 minutes
    queryFn: () =>
      getValidatorProposalVoteAccount(
        endpoint,
        proposalId,
        publicKey
      ),
  });
};
