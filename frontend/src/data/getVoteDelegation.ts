import { address, createSolanaRpc } from "@solana/kit";
import { fetchMaybeVoteDelegation, findVoteDelegationPda } from "@solana/svmgov";
import type { VoteDelegationData } from "@/types";

/**
 * Fetches the vote delegation of `subject` (an SPL vote account or a staker
 * wallet), pending or accepted, or `null` when it has none.
 */
export const getVoteDelegation = async (
  endpoint: string,
  subject: string | undefined,
): Promise<VoteDelegationData | null> => {
  if (subject === undefined) throw new Error("Subject public key is required");

  const [pda] = await findVoteDelegationPda({ splVoteAccount: address(subject) });
  const account = await fetchMaybeVoteDelegation(createSolanaRpc(endpoint), pda);
  if (!account.exists) return null;

  return {
    subject: account.data.subject,
    delegate: account.data.delegate,
    delegationAccepted: account.data.delegationAccepted,
  };
};
