import {
  createProgramWitDummyWallet,
  deriveVoteDelegationPda,
  VoteDelegationData,
} from "@/chain";
import { PublicKey } from "@solana/web3.js";

/**
 * Fetches the vote delegation of `subject` (an SPL vote account or a staker
 * wallet), pending or accepted, or `null` when it has none.
 */
export const getVoteDelegation = async (
  endpoint: string,
  subject: string | undefined,
): Promise<VoteDelegationData | null> => {
  if (subject === undefined) throw new Error("Subject public key is required");

  const program = createProgramWitDummyWallet(endpoint);
  const pda = deriveVoteDelegationPda(
    new PublicKey(subject),
    program.programId,
  );
  const account = await program.account.voteDelegation.fetchNullable(pda);
  if (!account) return null;

  return {
    subject: account.subject.toBase58(),
    delegate: account.delegate.toBase58(),
    delegationAccepted: account.delegationAccepted,
  };
};
