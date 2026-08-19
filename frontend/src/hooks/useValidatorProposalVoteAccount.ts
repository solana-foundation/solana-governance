import { useEndpoint } from "@/contexts/EndpointContext";
import { getValidatorProposalVoteAccount } from "@/data/getValidatorProposalVoteAccount";
import { GET_VALIDATOR_PROPOSAL_VOTE_ACCOUNTS } from "@/helpers";
import { useWalletSession } from "@/contexts/WalletSessionContext";
import { useQuery } from "@tanstack/react-query";

export const useValidatorProposalVoteAccount = (
  proposalId: string | undefined,
  enabled = true
) => {
  const { endpointUrl: endpoint } = useEndpoint();

  const { publicKey, connected } = useWalletSession();

  const enabledQuery = connected && !!publicKey && !!proposalId && enabled;

  return useQuery({
    queryKey: [
      GET_VALIDATOR_PROPOSAL_VOTE_ACCOUNTS,
      endpoint,
      proposalId,
      publicKey?.toBase58(),
    ],
    enabled: enabledQuery,
    staleTime: 1000 * 120, // 2 minutes
    queryFn: () =>
      getValidatorProposalVoteAccount(
        endpoint,
        proposalId,
        publicKey?.toBase58()
      ),
  });
};
