import type { RpcNetwork } from "@/types";

/**
 * Returns the same-origin HTTP endpoint used by browser Solana clients.
 *
 * The localhost fallback is used only while client components are rendered on
 * the server. Browser query and mutation functions resolve against the real
 * window origin before issuing RPC requests.
 */
export function getRpcProxyUrl(network: RpcNetwork): string {
  const origin =
    typeof window === "undefined" ? "http://localhost" : window.location.origin;
  const url = new URL("/api/rpc", origin);
  url.searchParams.set("cluster", network);
  return url.toString();
}
