import { ChainVoteAccountData } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
import { getAcceptedVoteDelegationsForDelegate } from "@/data";
import { useQuery } from "@tanstack/react-query";
import { useChainVoteAccounts } from "./useChainVoteAccounts";

export const useChainVoteAccount = (userPubKey: string | undefined) => {
  const { endpointUrl: endpoint } = useEndpoint();

  const {
    data: chainVoteAccounts = [],
    isLoading: isLoadingChainVoteAccounts,
    isPending: isPendingChainVoteAccounts,
  } = useChainVoteAccounts();

  const enabled =
    !!userPubKey && !isLoadingChainVoteAccounts && !isPendingChainVoteAccounts;

  const query = useQuery({
    staleTime: 1000 * 120, // 2 minutes
    enabled,
    queryKey: [
      "chain_vote_account",
      endpoint,
      userPubKey,
      chainVoteAccounts.length,
    ],
    queryFn: async (): Promise<ChainVoteAccountData | null> => {
      const chainVoteAccount = chainVoteAccounts.find(
        (voteAccount) => voteAccount.nodePubkey === userPubKey
      );
      if (chainVoteAccount) return chainVoteAccount;

      // A wallet that is the accepted vote delegate of exactly one validator
      // acts as that validator everywhere in the validator UI. Delegations
      // from staker wallets do not make the wallet a validator.
      const delegations = await getAcceptedVoteDelegationsForDelegate(
        endpoint,
        userPubKey
      );
      const delegatedVoteAccounts = chainVoteAccounts.filter((voteAccount) =>
        delegations.some(
          (delegation) => delegation.subject === voteAccount.voteAccount
        )
      );
      return delegatedVoteAccounts.length === 1
        ? delegatedVoteAccounts[0]
        : null;
    },
  });

  const isWaitingForChainVoteAccounts =
    !!userPubKey && (isLoadingChainVoteAccounts || isPendingChainVoteAccounts);

  return {
    ...query,
    isLoading: isWaitingForChainVoteAccounts || query.isLoading,
    isPending: isWaitingForChainVoteAccounts || query.isPending,
  };
};
