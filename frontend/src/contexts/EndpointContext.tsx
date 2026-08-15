"use client";

import React, {
  createContext,
  useContext,
  useEffect,
  useState,
  ReactNode,
} from "react";
import { RPCEndpoint } from "@/types";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { setTag } from "@sentry/nextjs";
import { env } from "@/env";
import { getRpcUrls } from "@/lib/getRpcUrls";
import {
  resolveSnapshotNetwork,
  type KnownSnapshotNetwork,
} from "@/lib/snapshotNetwork";

interface EndpointContextType {
  endpointType: RPCEndpoint;
  endpointUrl: string;
  network: KnownSnapshotNetwork | undefined;
  isResolvingNetwork: boolean;
  setEndpoint: (type: RPCEndpoint, url?: string) => void;
  resetToDefault: () => void;
}

const EndpointContext = createContext<EndpointContextType | undefined>(
  undefined,
);

export const RPC_URLS = getRpcUrls(env);

const DEFAULT_TYPE: RPCEndpoint = "mainnet";
const DEFAULT_URL = RPC_URLS[DEFAULT_TYPE];

const STORAGE_KEY = "solana-rpc-endpoint";

const getStoredValues = () => {
  if (typeof window !== "undefined") {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) {
      try {
        const { type, url } = JSON.parse(saved);
        return { endpointType: type, endpointUrl: url };
      } catch {
        console.error("error parsing rpc endpoint from local storage");
        // fallback
        return { endpointType: DEFAULT_TYPE, endpointUrl: DEFAULT_URL };
      }
    }
  }
  return { endpointType: DEFAULT_TYPE, endpointUrl: DEFAULT_URL };
};

export function EndpointProvider({ children }: { children: ReactNode }) {
  const [endpoint, setEndpoint] = useState<{
    endpointType: RPCEndpoint;
    endpointUrl: string;
  }>(getStoredValues());

  const queryClient = useQueryClient();

  const networkQuery = useQuery({
    queryKey: ["snapshot-network", endpoint.endpointType, endpoint.endpointUrl],
    queryFn: async () => {
      const resolved = await resolveSnapshotNetwork(
        endpoint.endpointType,
        endpoint.endpointUrl,
      );
      return resolved ?? null;
    },
    staleTime: Infinity,
    enabled: endpoint.endpointType === "custom" && Boolean(endpoint.endpointUrl),
  });

  const network: KnownSnapshotNetwork | undefined =
    endpoint.endpointType === "custom"
      ? (networkQuery.data ?? undefined)
      : endpoint.endpointType;

  // Which network an error came from is otherwise invisible in Sentry, and it is the first thing
  // you need: the NCN API serves a different snapshot per network. Type only — a custom
  // endpointUrl can contain an RPC provider API key.
  useEffect(() => {
    setTag("solana_network", network ?? endpoint.endpointType);
  }, [network, endpoint.endpointType]);

  const setEndpointData = (type: RPCEndpoint, customUrl?: string) => {
    const url = type === "custom" ? (customUrl ?? "") : RPC_URLS[type];
    setEndpoint({
      endpointType: type,
      endpointUrl: url,
    });
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ type, url }));
    queryClient.removeQueries();
  };

  const resetToDefault = () => {
    setEndpoint({
      endpointType: DEFAULT_TYPE,
      endpointUrl: DEFAULT_URL,
    });
    localStorage.removeItem(STORAGE_KEY);
  };

  return (
    <EndpointContext.Provider
      value={{
        endpointType: endpoint.endpointType,
        endpointUrl: endpoint.endpointUrl,
        network,
        isResolvingNetwork: networkQuery.isFetching,
        setEndpoint: setEndpointData,
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
