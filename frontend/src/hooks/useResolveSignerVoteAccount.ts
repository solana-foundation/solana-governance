import { address, createSolanaRpc, type Address } from "@solana/kit";
import { useQueryClient } from "@tanstack/react-query";
import { useCallback } from "react";
import { useEndpoint } from "@/contexts/EndpointContext";
import { resolveSignerVoteAccount } from "@/lib/governance/voteDelegations";
import { rawVoteAccountsQueryOptions } from "./useRawVoteAccounts";

/**
 * Returns a function resolving the SPL vote account a wallet acts for, as the
 * validator identity or as its accepted vote delegate. The cluster's vote
 * accounts come from the shared query cache, so they are downloaded only when
 * no fresh copy is cached.
 */
export function useResolveSignerVoteAccount() {
  const { endpointUrl } = useEndpoint();
  const queryClient = useQueryClient();

  return useCallback(
    async (signer: string): Promise<Address> => {
      const voteAccounts = await queryClient.fetchQuery(
        rawVoteAccountsQueryOptions(endpointUrl),
      );
      return resolveSignerVoteAccount(
        createSolanaRpc(endpointUrl),
        voteAccounts,
        address(signer),
      );
    },
    [endpointUrl, queryClient],
  );
}
