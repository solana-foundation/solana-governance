import { createSolanaRpc } from "@solana/kit";
import { useEndpoint } from "@/contexts/EndpointContext";
import { fetchVotes } from "@/lib/governance/programAccounts";
import { useQuery } from "@tanstack/react-query";

export const useVotes = () => {
  const { endpointUrl: endpoint } = useEndpoint();

  return useQuery({
    queryKey: ["proposalsVotes", endpoint],
    queryFn: () => getAllVotes(endpoint),
    select: (data) => data || [],
  });
};

const getAllVotes = async (endpoint: string) => {
  const votes = await fetchVotes(createSolanaRpc(endpoint), {});
  if (votes.length === 0) return null;

  return votes;
};
