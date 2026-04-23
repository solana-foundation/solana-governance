"use client";

import React, { createContext, useContext, useState, ReactNode } from "react";
import { RPCEndpoint } from "@/types";
import { useQueryClient } from "@tanstack/react-query";
import { env } from "@/env";
import { getRpcUrls } from "@/lib/getRpcUrls";

interface EndpointContextType {
  endpointType: RPCEndpoint;
  endpointUrl: string;
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
