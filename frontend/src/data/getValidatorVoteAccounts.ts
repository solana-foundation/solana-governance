import { ValidatorVoteAccountData } from "@/types";
import { createSolanaRpc } from "@solana/kit";

export const getValidatorVoteAccounts = async (
  endpoint: string,
  validatorPubkey: string | undefined,
) => {
  if (!validatorPubkey) throw new Error("User public key is required");

  const voteAccounts = await createSolanaRpc(endpoint).getVoteAccounts().send();
  // Delinquent vote accounts are excluded, as in useChainVoteAccounts, so the
  // wallet's voting power and its detected role always agree.
  const validatorVoteAccount = voteAccounts.current.find(
    (account) => account.nodePubkey === validatorPubkey,
  );

  if (!validatorVoteAccount) {
    console.warn(`No SPL vote account found for validator identity ${validatorPubkey}`);
    return null;
  }

  return {
    voteAccount: validatorVoteAccount.votePubkey,
    activeStake: validatorVoteAccount.activatedStake,
    nodePubkey: validatorVoteAccount.nodePubkey,
  } satisfies ValidatorVoteAccountData;
};
