import { useEndpoint } from "@/contexts/EndpointContext";
import { getVoteDelegation } from "@/data";
import { useQuery } from "@tanstack/react-query";

/**
 * The vote delegation of `subject` (an SPL vote account or a staker wallet),
 * pending or accepted; `null` when it has none.
 */
export const useVoteDelegation = (subject: string | undefined) => {
  const { endpointUrl: endpoint } = useEndpoint();

  return useQuery({
    staleTime: 1000 * 120, // 2 minutes
    enabled: !!subject,
    queryKey: ["vote_delegation", endpoint, subject],
    queryFn: () => getVoteDelegation(endpoint, subject),
  });
};
