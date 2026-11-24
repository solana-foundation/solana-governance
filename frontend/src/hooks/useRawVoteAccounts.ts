import { useEndpoint } from "@/contexts/EndpointContext";
import {
  fetchRawVoteAccounts,
  type RawVoteAccountsData,
} from "@/lib/rpcVoteAccounts";
import { queryOptions, useQuery } from "@tanstack/react-query";
import { GET_VOTE_ACCOUNTS } from "@/helpers";

export type { RawVoteAccountsData } from "@/lib/rpcVoteAccounts";

/**
 * Query for the cluster's raw vote accounts (current and delinquent), shared
 * so every reader of `endpointUrl` reuses one cached download.
 */
export function rawVoteAccountsQueryOptions(endpointUrl: string) {
  return queryOptions<RawVoteAccountsData>({
    queryKey: [GET_VOTE_ACCOUNTS, endpointUrl],
    queryFn: ({ signal }) => fetchRawVoteAccounts(endpointUrl, signal),
    staleTime: 1000 * 120, // 2 minutes
  });
}

/**
 * Hook to fetch raw Solana vote accounts (current and delinquent)
 * Caches the data for reuse across components
 * @returns The raw vote accounts data, or undefined if loading/error
 */
export function useRawVoteAccounts() {
  const { endpointUrl } = useEndpoint();

  return useQuery({
    ...rawVoteAccountsQueryOptions(endpointUrl),
    refetchOnWindowFocus: false,
  });
}
