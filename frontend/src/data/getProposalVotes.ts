import { createSolanaRpc, type Address } from "@solana/kit";
import { fetchVotes } from "@/lib/governance/programAccounts";
import { mapVoteAccountDto } from "./getVoteAccounts";
import { OldVoteAccountData } from "@/types";

/**
 * Fetches votes for a specific proposal
 * Filters by proposal public key directly on the RPC for efficient querying
 */
export const getProposalVotes = async (
  proposalPublicKey: Address,
  endpoint: string,
): Promise<Array<OldVoteAccountData & { voter: Address }>> => {
  const proposalVotes = await fetchVotes(createSolanaRpc(endpoint), {
    proposal: proposalPublicKey,
  });

  // Map to the expected format with voter field
  return proposalVotes.map(({ address, data }) => ({ ...mapVoteAccountDto(data, address), voter: address }));
};
