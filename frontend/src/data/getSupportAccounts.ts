import { SupportAccountData } from "@/types";
import { createSolanaRpc, type Address } from "@solana/kit";
import { fetchSupports } from "@/lib/governance/programAccounts";
import { toLegacyPublicKey } from "@/lib/governance/legacyAdapters";

interface GetSupportFilter {
  name: "proposal" | "validator";
  value: string;
}

export type GetSupportFilters = GetSupportFilter[];

const filterOffsetMap: Record<GetSupportFilter["name"], number> = {
  proposal: 8, // 8 bytes discriminator
  validator: 40, // 8 bytes discriminator + 32 bytes proposal
};

export const getSupportAccounts = async (
  endpoint: string,
  filters: GetSupportFilters
): Promise<SupportAccountData[]> => {
  if (filters.length === 0) {
    throw new Error(
      "getSupportAccounts: At least one filter is required. Cannot fetch all support accounts."
    );
  }

  const values = Object.fromEntries(filters.map(({ name, value }) => [name, value])) as Partial<Record<GetSupportFilter["name"], string>>;
  const supportAccs = await fetchSupports(createSolanaRpc(endpoint), {
    proposal: values.proposal as Address | undefined,
    validator: values.validator as Address | undefined,
  });
  return supportAccs.map(({ address, data }) => ({
    publicKey: toLegacyPublicKey(address),
    proposal: toLegacyPublicKey(data.proposal),
    validator: toLegacyPublicKey(data.validator),
    bump: data.bump,
  }));
};
