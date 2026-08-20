import { VoteAccountData } from "@/types";
import { createSolanaRpc, type Address } from "@solana/kit";
import { fetchVotes, type Vote } from "@/lib/governance/programAccounts";

/**
 * Fetches a validator's vote account for a specific proposal
 */
export const getValidatorProposalVoteAccount = async (
  endpoint: string,
  proposalPublicKey: string | undefined,
  validatorPublicKey: string | undefined,
): Promise<VoteAccountData | null> => {
  if (proposalPublicKey === undefined)
    throw new Error("Proposal public key is not loaded");

  if (validatorPublicKey === undefined)
    throw new Error("Validator public key is required");

  const voteAccounts = await fetchVotes(createSolanaRpc(endpoint), {
    proposal: proposalPublicKey as Address,
    validator: validatorPublicKey as Address,
  });

  if (voteAccounts.length === 0) {
    console.warn(
      `No program vote account found for validator ${validatorPublicKey} and proposal ${proposalPublicKey}`,
    );
    return null;
  }

  // Should only be one result since PDA is unique per (proposal, spl_vote_account)
  if (voteAccounts.length > 1) {
    console.warn(
      `Multiple vote accounts found for validator ${validatorPublicKey} and proposal ${proposalPublicKey}, using first one`,
    );
  }

  return mapVoteAccountDto(voteAccounts[0].data);
};

/**
 * Maps raw on-chain vote account to internal type.
 */
function mapVoteAccountDto(raw: Vote): VoteAccountData {
  return {
    validator: raw.validator, proposal: raw.proposal,
    forVotesBp: raw.forVotesBp, againstVotesBp: raw.againstVotesBp, abstainVotesBp: raw.abstainVotesBp,
    forVotesLamports: raw.forVotesLamports, againstVotesLamports: raw.againstVotesLamports, abstainVotesLamports: raw.abstainVotesLamports,
    stake: raw.stake, overrideLamports: raw.overrideLamports, voteTimestamp: raw.voteTimestamp,
    bump: raw.bump,
  };
}
