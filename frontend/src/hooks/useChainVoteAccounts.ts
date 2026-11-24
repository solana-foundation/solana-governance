import { useEndpoint } from "@/contexts/EndpointContext";
import { fetchRawVoteAccounts, type ChainVoteAccountData, type RpcVoteAccountData } from "@/lib/rpcVoteAccounts";
import { useQuery } from "@tanstack/react-query";

export const useChainVoteAccounts = () => {
  const { endpointUrl: endpoint } = useEndpoint();

  return useQuery({
    staleTime: 1000 * 120, // 2 minutes
    queryKey: ["chain_vote_accounts", endpoint],
    queryFn: async ({ signal }): Promise<ChainVoteAccountData[]> => {
      const voteAccounts = await fetchRawVoteAccounts(endpoint, signal);

      const mappedVoteAccounts = voteAccounts.current.map(
        mapChainVoteAccountDto
      );
      return mappedVoteAccounts;
    },
  });
};

function mapChainVoteAccountDto(
  voteAccount: RpcVoteAccountData
): ChainVoteAccountData {
  return {
    voteAccount: voteAccount.votePubkey,
    activeStake: voteAccount.activatedStake,
    nodePubkey: voteAccount.nodePubkey,
  };
}
