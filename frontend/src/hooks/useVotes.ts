import { createProgramWitDummyWallet } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
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
  const program = createProgramWitDummyWallet(endpoint);

  const votes = await program.account.vote.all();
  if (votes.length === 0) return null;

  return votes;
};
