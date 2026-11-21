import { useEndpoint } from "@/contexts/EndpointContext";
import { getValidatorProposalVoteAccount } from "@/data/getValidatorProposalVoteAccount";
import { GET_VALIDATOR_PROPOSAL_VOTE_ACCOUNTS } from "@/helpers";
import { useWallet } from "@solana/wallet-adapter-react";
import { useQuery } from "@tanstack/react-query";
import { useChainVoteAccount } from "./useChainVoteAccount";

/**
 * The connected wallet's vote on `proposalId`, looked up by the validator
 * identity the wallet acts for. Votes name the identity, so a delegate's vote
 * is found through its validator rather than the delegate's own key.
 */
export const useValidatorProposalVoteAccount = (
  proposalId: string | undefined,
  enabled = true
) => {
  const { endpointUrl: endpoint } = useEndpoint();

  const { publicKey, connected } = useWallet();
  const { data: chainVoteAccount, isLoading: isResolvingIdentity } =
    useChainVoteAccount(publicKey?.toBase58());
  const validatorIdentity = chainVoteAccount?.nodePubkey ?? publicKey?.toBase58();

  const enabledQuery =
    connected && !!publicKey && !!proposalId && enabled && !isResolvingIdentity;

  return useQuery({
    queryKey: [
      GET_VALIDATOR_PROPOSAL_VOTE_ACCOUNTS,
      endpoint,
      proposalId,
      validatorIdentity,
    ],
    enabled: enabledQuery,
    staleTime: 1000 * 120, // 2 minutes
    queryFn: () =>
      getValidatorProposalVoteAccount(endpoint, proposalId, validatorIdentity),
  });
};
