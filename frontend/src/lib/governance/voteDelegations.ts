import type { Address, GetProgramAccountsApi, GetVoteAccountsApi, Rpc } from "@solana/kit";
import type { VoteDelegationData } from "@/types";
import { fetchVoteDelegations } from "./programAccounts";

/**
 * Lists the accepted vote delegations that name `delegate`, whatever their
 * subject (SPL vote accounts of validators and voting wallets of stakers alike).
 * A pending delegation grants no authority, so it is excluded.
 */
export async function fetchAcceptedVoteDelegations(
  rpc: Rpc<GetProgramAccountsApi>,
  delegate: Address,
): Promise<VoteDelegationData[]> {
  const delegations = await fetchVoteDelegations(rpc, { delegate });
  return delegations
    .filter(({ data }) => data.delegationAccepted)
    .map(({ data }) => ({
      subject: data.subject,
      delegate: data.delegate,
      delegationAccepted: data.delegationAccepted,
    }));
}

/**
 * Resolves the SPL vote account `signer` acts for: the vote account whose node
 * identity is `signer`, or otherwise the single vote account with an accepted
 * vote delegation naming `signer`. Delegations whose subject is a staker wallet
 * are ignored here.
 *
 * @throws If the signer is neither a validator identity nor the accepted
 * delegate of exactly one vote account.
 */
export async function resolveSignerVoteAccount(
  rpc: Rpc<GetProgramAccountsApi & GetVoteAccountsApi>,
  signer: Address,
): Promise<Address> {
  const voteAccounts = await rpc.getVoteAccounts().send();
  const allVoteAccounts = [...voteAccounts.current, ...voteAccounts.delinquent];
  const own = allVoteAccounts.find((account) => account.nodePubkey === signer);
  if (own) return own.votePubkey;

  const votePubkeys = new Set<string>(allVoteAccounts.map((account) => account.votePubkey));
  const delegated = (await fetchAcceptedVoteDelegations(rpc, signer)).filter(
    ({ subject }) => votePubkeys.has(subject),
  );
  if (delegated.length === 1) return delegated[0].subject;
  if (delegated.length > 1) {
    throw new Error(
      `Wallet ${signer} is the accepted vote delegate of ${delegated.length} vote accounts; voting for several validators from one wallet is not supported here yet`,
    );
  }
  throw new Error(
    `No SPL vote account found for validator identity ${signer}, and no accepted vote delegation names it`,
  );
}
