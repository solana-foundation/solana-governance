import { address, createSolanaRpc } from "@solana/kit";
import { fetchAcceptedVoteDelegations } from "@/lib/governance/voteDelegations";
import type { VoteDelegationData } from "@/types";

/**
 * Fetches the accepted vote delegations that name `delegate`: the SPL vote
 * accounts the wallet may create, support and vote for in place of their
 * validator identity, and the staker wallets whose stake it may override for.
 */
export const getAcceptedVoteDelegationsForDelegate = async (
  endpoint: string,
  delegate: string | undefined,
): Promise<VoteDelegationData[]> => {
  if (delegate === undefined) throw new Error("Delegate public key is required");

  return fetchAcceptedVoteDelegations(createSolanaRpc(endpoint), address(delegate));
};
