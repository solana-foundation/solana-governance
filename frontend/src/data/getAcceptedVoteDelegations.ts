import {
  createProgramWitDummyWallet,
  getAcceptedVoteDelegations,
  VoteDelegationData,
} from "@/chain";
import { PublicKey } from "@solana/web3.js";

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

  const program = createProgramWitDummyWallet(endpoint);
  const delegations = await getAcceptedVoteDelegations(
    program,
    new PublicKey(delegate),
  );

  return delegations.map(({ account }) => ({
    subject: account.subject.toBase58(),
    delegate: account.delegate.toBase58(),
    delegationAccepted: account.delegationAccepted,
  }));
};
