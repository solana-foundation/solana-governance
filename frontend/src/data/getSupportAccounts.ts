import { SupportAccountData } from "@/types";
import { createSolanaRpc, type Address } from "@solana/kit";
import { fetchSupports } from "@/lib/governance/programAccounts";

interface GetSupportFilter {
  name: "proposal" | "validator";
  value: string;
}

export type GetSupportFilters = GetSupportFilter[];

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
    publicKey: address,
    proposal: data.proposal,
    validator: data.validator,
    bump: data.bump,
  }));
};
