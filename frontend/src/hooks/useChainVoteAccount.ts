import { ChainVoteAccountData } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
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

      return chainVoteAccount || null;
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
