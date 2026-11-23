"use client";

import React, {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { setTag } from "@sentry/nextjs";
import { useQueryClient } from "@tanstack/react-query";
import type { RpcNetwork } from "@/types";
import { getRpcProxyUrl } from "@/lib/getRpcProxyUrl";

interface EndpointContextType {
  network: RpcNetwork;
  endpointUrl: string;
  setEndpoint: (type: RpcNetwork) => void;
  resetToDefault: () => void;
}

const EndpointContext = createContext<EndpointContextType | undefined>(
  undefined,
);

const DEFAULT_NETWORK: RpcNetwork = "mainnet";
const STORAGE_KEY = "solana-rpc-cluster";
const LEGACY_STORAGE_KEY = "solana-rpc-endpoint";
const ENDPOINTS = new Set<RpcNetwork>(["mainnet", "testnet", "devnet"]);

function isRpcEndpoint(value: unknown): value is RpcNetwork {
  return typeof value === "string" && ENDPOINTS.has(value as RpcNetwork);
}

function getStoredEndpoint(): RpcNetwork {
  if (typeof window === "undefined") return DEFAULT_NETWORK;

  const saved = localStorage.getItem(STORAGE_KEY);
  if (isRpcEndpoint(saved)) return saved;

  // Migrate the previous { type, url } value without retaining its RPC URL.
  const legacy = localStorage.getItem(LEGACY_STORAGE_KEY);
  if (legacy) {
    try {
      const { type } = JSON.parse(legacy) as { type?: unknown };
      if (isRpcEndpoint(type)) return type;
    } catch {
      // Ignore malformed legacy settings and fall back to mainnet.
    } finally {
      localStorage.removeItem(LEGACY_STORAGE_KEY);
    }
  }

  return DEFAULT_NETWORK;
}

export function EndpointProvider({ children }: { children: ReactNode }) {
  const [network, setNetwork] = useState<RpcNetwork>(getStoredEndpoint);
  const endpointUrl = useMemo(() => getRpcProxyUrl(network), [network]);
  const queryClient = useQueryClient();

  useEffect(() => {
    setTag("solana_network", network);
  }, [network]);

  const setEndpoint = (type: RpcNetwork) => {
    if (type !== network) {
      // Query keys for some values derived from multiple RPC sources do not carry the endpoint.
      // Clear the cache before switching so none can survive into the next cluster.
      queryClient.removeQueries();
      setNetwork(type);
    }
    localStorage.setItem(STORAGE_KEY, type);
  };

  const resetToDefault = () => {
    if (network !== DEFAULT_NETWORK) queryClient.removeQueries();
    setNetwork(DEFAULT_NETWORK);
    localStorage.removeItem(STORAGE_KEY);
    localStorage.removeItem(LEGACY_STORAGE_KEY);
  };

  return (
    <EndpointContext.Provider
      value={{
        network,
        endpointUrl,
        setEndpoint,
        resetToDefault,
      }}
    >
      {children}
    </EndpointContext.Provider>
  );
}

export function useEndpoint() {
  const context = useContext(EndpointContext);
  if (context === undefined) {
    throw new Error("useEndpoint must be used within an EndpointProvider");
  }
  return context;
}
