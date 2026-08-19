import { VoteOverrideAccountData } from "@/types";
import { createSolanaRpc, type Address } from "@solana/kit";
import { fetchVoteOverrides } from "@/lib/governance/programAccounts";
import { mapVoteOverrideAccountDto } from "./getVoteOverrideAccounts";
import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";

/**
 * Fetches vote overrides for a specific proposal
 * Filters by proposal public key directly on the RPC for efficient querying
 */
export const getProposalVoteOverrides = async (
  proposalPublicKey: PublicKey,
  endpoint: string,
): Promise<
  Array<
    VoteOverrideAccountData & {
      voter: PublicKey;
      activeStake: number;
      identity: PublicKey;
      voteTimestamp: BN;
    }
  >
> => {
  const proposalOverrides = await fetchVoteOverrides(createSolanaRpc(endpoint), {
    proposal: proposalPublicKey.toBase58() as Address,
  });

  // Map to the expected format with voter, activeStake, identity, and voteTimestamp fields
  return proposalOverrides.map(({ address, data }) => {
    const mapped = mapVoteOverrideAccountDto(data, address);
    return {
      ...mapped,
      voter: mapped.stakeAccount,
      activeStake: mapped.stakeAmount.toNumber() || 0,
      identity: mapped.validator, // Map validator to identity for consistency
      voteTimestamp: mapped.voteOverrideTimestamp, // Map voteOverrideTimestamp to voteTimestamp
    };
  });
};
