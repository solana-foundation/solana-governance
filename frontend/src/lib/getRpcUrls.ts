import "server-only";
import type { RpcNetwork } from "@/types";

/** Default RPC URLs when env vars are not set. Single source of truth for mainnet/testnet/devnet. */
export const DEFAULT_RPC_URLS: Record<RpcNetwork, string> = {
  mainnet: "https://api.mainnet-beta.solana.com",
  testnet: "https://api.testnet.solana.com",
  devnet: "https://api.devnet.solana.com",
};

export interface RpcEnvSource {
  SOLANA_RPC_URL_MAINNET?: string;
  SOLANA_RPC_URL_TESTNET?: string;
  SOLANA_RPC_URL_DEVNET?: string;
}

/**
 * Returns server-side RPC URLs for mainnet, testnet, and devnet.
 */
export function getRpcUrls(
  envSource: RpcEnvSource = {},
): Record<RpcNetwork, string> {
  return {
    mainnet: envSource.SOLANA_RPC_URL_MAINNET ?? DEFAULT_RPC_URLS.mainnet,
    testnet: envSource.SOLANA_RPC_URL_TESTNET ?? DEFAULT_RPC_URLS.testnet,
    devnet: envSource.SOLANA_RPC_URL_DEVNET ?? DEFAULT_RPC_URLS.devnet,
  };
}

/**
 * Resolves the server-side upstream RPC URL for a supported cluster.
 */
export function getRpcUrlForNetwork(
  network: RpcNetwork,
  envSource?: RpcEnvSource,
): string {
  const urls = getRpcUrls(
    envSource ??
      (typeof process !== "undefined" ? (process.env as RpcEnvSource) : {}),
  );
  return urls[network];
}
