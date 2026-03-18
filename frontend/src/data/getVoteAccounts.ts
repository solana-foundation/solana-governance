import { createProgramWitDummyWallet } from "@/chain";
import { OldVoteAccountData, RawVoteAccountDataAccount } from "@/types";

/**
 * @deprecated cant fetch ALL vote accounts at once.
 */
export const getVoteAccounts = async (
  endpoint: string,
): Promise<OldVoteAccountData[]> => {
  const program = createProgramWitDummyWallet(endpoint);

  // TODO: implement filter. we cant fetch all vote accounts at once.
  // fetch vote accounts for a specific proposals or stake account owner only
  //  (stake account owner to be added to program, revisit this method once program is updated)
  const voteAccs = await program.account.vote.all();

  return voteAccs.map(mapVoteAccountDto);
};

/**
 * Maps raw on-chain vote account to internal type.
 */
export function mapVoteAccountDto(
  rawAccount: RawVoteAccountDataAccount,
): OldVoteAccountData {
  const raw = rawAccount.account;

  return {
    voteAccount: rawAccount.publicKey,
    proposal: raw.proposal,
    // validator data
    activeStake: raw.stake ? +raw.stake.toString() : 0,
    identity: raw.validator,
    commission: 0,
    lastVote: 0,
    credits: 0,
    epochCredits: 0,
    activatedStake: 0,
    // vote data
    forVotesBp: raw.forVotesBp,
    againstVotesBp: raw.againstVotesBp,
    abstainVotesBp: raw.abstainVotesBp,
    forVotesLamports: raw.forVotesLamports,
    againstVotesLamports: raw.againstVotesLamports,
    abstainVotesLamports: raw.abstainVotesLamports,
    stake: raw.stake,
    overrideLamports: raw.overrideLamports,
    voteTimestamp: raw.voteTimestamp,
    bump: raw.bump,
  };
}
