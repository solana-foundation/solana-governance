jest.mock("@/env", () => ({
  env: {
    NEXT_PUBLIC_SOLANA_RPC_MAINNET: "https://api.mainnet-beta.solana.com",
    NEXT_PUBLIC_SOLANA_RPC_TESTNET: "https://api.testnet.solana.com",
    NEXT_PUBLIC_SOLANA_RPC_DEVNET: "https://api.devnet.solana.com",
  },
}));

jest.mock("@sentry/nextjs", () => ({
  setTag: jest.fn(),
}));

const mockGetGenesisHash = jest.fn();
jest.mock("@solana/web3.js", () => ({
  ...jest.requireActual("@solana/web3.js"),
  Connection: jest.fn().mockImplementation(() => ({
    getGenesisHash: () => mockGetGenesisHash(),
  })),
}));

import React, { type ReactNode } from "react";
import { renderHook, waitFor, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

import { CLUSTER_GENESIS_HASHES } from "@/lib/snapshotNetwork";
import { EndpointProvider, useEndpoint } from "../EndpointContext";

const STORAGE_KEY = "solana-rpc-endpoint";

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
    },
  });

  return function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        <EndpointProvider>{children}</EndpointProvider>
      </QueryClientProvider>
    );
  };
}

describe("EndpointProvider network resolution", () => {
  beforeEach(() => {
    localStorage.clear();
    mockGetGenesisHash.mockReset();
  });

  it("uses the preset endpoint type as the network without calling RPC", () => {
    const { result } = renderHook(() => useEndpoint(), {
      wrapper: createWrapper(),
    });

    expect(result.current.endpointType).toBe("mainnet");
    expect(result.current.network).toBe("mainnet");
    expect(result.current.isResolvingNetwork).toBe(false);
    expect(mockGetGenesisHash).not.toHaveBeenCalled();
  });

  it("resolves a custom RPC from its genesis hash", async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ type: "custom", url: "https://my-rpc.example" }),
    );
    mockGetGenesisHash.mockResolvedValue(CLUSTER_GENESIS_HASHES.devnet);

    const { result } = renderHook(() => useEndpoint(), {
      wrapper: createWrapper(),
    });

    expect(result.current.endpointType).toBe("custom");
    expect(result.current.network).toBeUndefined();

    await waitFor(() => {
      expect(result.current.network).toBe("devnet");
    });
    expect(result.current.isResolvingNetwork).toBe(false);
  });

  it("leaves network unset when a custom RPC genesis hash is unrecognized", async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ type: "custom", url: "http://localhost:8899" }),
    );
    mockGetGenesisHash.mockResolvedValue("localnet-genesis");

    const { result } = renderHook(() => useEndpoint(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isResolvingNetwork).toBe(false);
    });
    expect(result.current.network).toBeUndefined();
  });

  it("re-resolves when switching to a custom RPC", async () => {
    mockGetGenesisHash.mockResolvedValue(CLUSTER_GENESIS_HASHES.mainnet);

    const { result } = renderHook(() => useEndpoint(), {
      wrapper: createWrapper(),
    });

    act(() => {
      result.current.setEndpoint("custom", "https://mainnet-rpc.example");
    });

    await waitFor(() => {
      expect(result.current.network).toBe("mainnet");
    });
    expect(result.current.endpointType).toBe("custom");
  });
});
