import { VoterSummaryResponse } from "@/chain";
import { RPCEndpoint } from "@/types";
import { DEFAULT_NCN_API_URL } from "@/lib/constants";

export const getVoterWalletSummary = async (
  network: RPCEndpoint,
  walletAddress: string | undefined,
  slot: number,
  ncnApiUrl?: string
): Promise<VoterSummaryResponse> => {
  if (walletAddress === undefined) throw new Error("Wallet not connected");

  const baseUrl = ncnApiUrl || DEFAULT_NCN_API_URL;
  const url = `${baseUrl}/voter/${walletAddress}?network=${network}&slot=${slot}`;
  const response = await fetch(url);

  if (!response.ok) {
    throw new Error(`Failed to get voter summary: ${response.statusText}`);
  }

  return await response.json();
};
