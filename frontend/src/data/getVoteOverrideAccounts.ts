import { VoteOverrideAccountData } from "@/types";
import { createSolanaRpc, type Address } from "@solana/kit";
import { fetchVoteOverrides, type VoteOverride } from "@/lib/governance/programAccounts";
import { toLegacyBn, toLegacyPublicKey } from "@/lib/governance/legacyAdapters";

interface GetVoteOverrideFilter {
  name: "delegator" | "proposal" | "validator" | "stakeAccount";
  value: string;
}

export type GetVoteOverrideFilters = GetVoteOverrideFilter[];

export const getVoteOverrideAccounts = async (
  endpoint: string,
  filters: GetVoteOverrideFilters,
): Promise<VoteOverrideAccountData[]> => {
  if (filters.length === 0) {
    throw new Error(
      "getVoteOverrideAccounts: At least one filter is required. Cannot fetch all voteOverride accounts.",
    );
  }

  const values = Object.fromEntries(filters.map(({ name, value }) => [name, value])) as Partial<Record<GetVoteOverrideFilter["name"], string>>;
  const accounts = await fetchVoteOverrides(createSolanaRpc(endpoint), {
    delegator: values.delegator as Address | undefined,
    stakeAccount: values.stakeAccount as Address | undefined,
    validator: values.validator as Address | undefined,
    proposal: values.proposal as Address | undefined,
  });
  return accounts.map(({ address, data }) => mapVoteOverrideAccountDto(data, address));
};

/**
 * Maps raw on-chain vote account to internal type.
 */
export function mapVoteOverrideAccountDto(
  raw: VoteOverride,
  address: string,
): VoteOverrideAccountData {
  return {
    publicKey: toLegacyPublicKey(address), delegator: toLegacyPublicKey(raw.delegator), stakeAccount: toLegacyPublicKey(raw.stakeAccount), validator: toLegacyPublicKey(raw.validator), proposal: toLegacyPublicKey(raw.proposal), voteAccountValidator: toLegacyPublicKey(raw.voteAccountValidator),
    forVotesBp: toLegacyBn(raw.forVotesBp), againstVotesBp: toLegacyBn(raw.againstVotesBp), abstainVotesBp: toLegacyBn(raw.abstainVotesBp),
    forVotesLamports: toLegacyBn(raw.forVotesLamports), againstVotesLamports: toLegacyBn(raw.againstVotesLamports), abstainVotesLamports: toLegacyBn(raw.abstainVotesLamports), stakeAmount: toLegacyBn(raw.stakeAmount), voteOverrideTimestamp: toLegacyBn(raw.voteOverrideTimestamp),
    bump: raw.bump,
  };
}
