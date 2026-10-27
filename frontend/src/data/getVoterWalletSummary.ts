import { VoterSummaryResponse } from "@/chain";
import { DEFAULT_NCN_API_URL, fetchNcnJson } from "@/lib/ncnApi";
import type { RpcNetwork } from "@/types";

export const getVoterWalletSummary = async (
  network: RpcNetwork,
  walletAddress: string | undefined,
  slot: number,
  ncnApiUrl?: string,
  signal?: AbortSignal,
): Promise<VoterSummaryResponse> => {
  if (walletAddress === undefined) throw new Error("Wallet not connected");

  const baseUrl = ncnApiUrl || DEFAULT_NCN_API_URL;
  const url = `${baseUrl}/voter/${walletAddress}?network=${network}&slot=${slot}`;

  return fetchNcnJson<VoterSummaryResponse>(url, {
    signal,
    label: "voter summary",
    resource: "voter-summary",
  });
};
