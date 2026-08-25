import { useEndpoint } from "@/contexts/EndpointContext";
import {
  fetchRawVoteAccounts,
  type RawVoteAccountsData,
} from "@/lib/rpcVoteAccounts";
import { useQuery } from "@tanstack/react-query";
import { GET_VOTE_ACCOUNTS } from "@/helpers";

export type { RawVoteAccountsData } from "@/lib/rpcVoteAccounts";

/**
 * Hook to fetch raw Solana vote accounts (current and delinquent)
 * Caches the data for reuse across components
 * @returns The raw vote accounts data, or undefined if loading/error
 */
export function useRawVoteAccounts() {
  const { endpointUrl } = useEndpoint();

  return useQuery<RawVoteAccountsData>({
    queryKey: [GET_VOTE_ACCOUNTS, endpointUrl],
    queryFn: async () => {
      return fetchRawVoteAccounts(endpointUrl);
    },
    staleTime: 1000 * 120, // 2 minutes
    refetchOnWindowFocus: false,
  });
}
