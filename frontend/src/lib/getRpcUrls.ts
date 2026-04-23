import type { RPCEndpoint } from "@/types";

/** Default RPC URLs when env vars are not set. Single source of truth for mainnet/testnet/devnet. */
export const DEFAULT_RPC_URLS: Record<
  Exclude<RPCEndpoint, "custom">,
  string
> = {
  mainnet: "https://api.mainnet-beta.solana.com",
  testnet: "https://api.testnet.solana.com",
  devnet: "https://api.devnet.solana.com",
};

export interface RpcEnvSource {
  NEXT_PUBLIC_SOLANA_RPC_MAINNET?: string;
  NEXT_PUBLIC_SOLANA_RPC_TESTNET?: string;
  NEXT_PUBLIC_SOLANA_RPC_DEVNET?: string;
}

/**
 * Returns RPC URLs for mainnet, testnet, devnet (env overrides with defaults).
 * Use from client with getRpcUrls(env) or from server with getRpcUrls(process.env).
 */
export function getRpcUrls(
  envSource: RpcEnvSource = {},
): Record<Exclude<RPCEndpoint, "custom">, string> {
  return {
    mainnet:
      envSource.NEXT_PUBLIC_SOLANA_RPC_MAINNET ?? DEFAULT_RPC_URLS.mainnet,
    testnet:
      envSource.NEXT_PUBLIC_SOLANA_RPC_TESTNET ?? DEFAULT_RPC_URLS.testnet,
    devnet: envSource.NEXT_PUBLIC_SOLANA_RPC_DEVNET ?? DEFAULT_RPC_URLS.devnet,
  };
}

/**
 * Resolves the effective RPC URL for an endpoint type.
 * For custom, pass the URL in customRpcUrl (validated as URL).
 */
export function getRpcUrlForEndpoint(
  endpoint: RPCEndpoint,
  customRpcUrl?: string | null,
  envSource?: RpcEnvSource,
): string {
  if (endpoint === "custom") {
    if (!customRpcUrl || typeof customRpcUrl !== "string") {
      throw new Error("rpcUrl is required when endpoint is custom");
    }
    try {
      new URL(customRpcUrl);
    } catch {
      throw new Error("Invalid rpcUrl for custom endpoint");
    }
    return customRpcUrl;
  }
  const urls = getRpcUrls(
    envSource ??
      (typeof process !== "undefined"
        ? (process.env as RpcEnvSource)
        : {}),
  );
  return urls[endpoint];
}
