import { createSolanaRpc, type Address } from "@solana/kit";
import { fetchVotes } from "@/lib/governance/programAccounts";
import { mapVoteAccountDto } from "./getVoteAccounts";
import { OldVoteAccountData } from "@/types";
import { toLegacyPublicKey, type LegacyPublicKey as PublicKey } from "@/lib/governance/legacyAdapters";

/**
 * Fetches votes for a specific proposal
 * Filters by proposal public key directly on the RPC for efficient querying
 */
export const getProposalVotes = async (
  proposalPublicKey: PublicKey,
  endpoint: string,
): Promise<Array<OldVoteAccountData & { voter: PublicKey }>> => {
  const proposalVotes = await fetchVotes(createSolanaRpc(endpoint), {
    proposal: proposalPublicKey.toBase58() as Address,
  });

  // Map to the expected format with voter field
  return proposalVotes.map(({ address, data }) => ({ ...mapVoteAccountDto(data, address), voter: toLegacyPublicKey(address) }));
};
