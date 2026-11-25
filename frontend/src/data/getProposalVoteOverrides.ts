import { VoteOverrideAccountData } from "@/types";
import { createSolanaRpc, type Address } from "@solana/kit";
import { fetchVoteOverrides } from "@/lib/governance/programAccounts";
import { mapVoteOverrideAccountDto } from "./getVoteOverrideAccounts";

/**
 * Fetches vote overrides for a specific proposal
 * Filters by proposal public key directly on the RPC for efficient querying
 */
export const getProposalVoteOverrides = async (
  proposalPublicKey: Address,
  endpoint: string,
): Promise<
  Array<
    VoteOverrideAccountData & {
      voter: Address;
      activeStake: bigint;
      identity: Address;
      voteTimestamp: bigint;
    }
  >
> => {
  const proposalOverrides = await fetchVoteOverrides(createSolanaRpc(endpoint), {
    proposal: proposalPublicKey,
  });

  // Map to the expected format with voter, activeStake, identity, and voteTimestamp fields
  return proposalOverrides.map(({ address, data }) => {
    const mapped = mapVoteOverrideAccountDto(data, address);
    return {
      ...mapped,
      voter: mapped.stakeAccount,
      activeStake: mapped.stakeAmount,
      identity: mapped.validator, // Map validator to identity for consistency
      voteTimestamp: mapped.voteOverrideTimestamp, // Map voteOverrideTimestamp to voteTimestamp
    };
  });
};
