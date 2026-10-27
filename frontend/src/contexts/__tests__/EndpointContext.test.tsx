jest.mock("@sentry/nextjs", () => ({ setTag: jest.fn() }));

import React from "react";
import { act, renderHook } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { EndpointProvider, useEndpoint } from "../EndpointContext";

const STORAGE_KEY = "solana-rpc-cluster";
const LEGACY_STORAGE_KEY = "solana-rpc-endpoint";

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: Infinity } },
  });
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>
      <EndpointProvider>{children}</EndpointProvider>
    </QueryClientProvider>
  );

  return { queryClient, wrapper };
}

describe("EndpointProvider", () => {
  beforeEach(() => localStorage.clear());

  it("uses the same-origin mainnet proxy by default", () => {
    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useEndpoint(), { wrapper });

    expect(result.current.network).toBe("mainnet");
    expect(result.current.endpointUrl).toBe(
      "http://localhost/api/rpc?cluster=mainnet",
    );
  });

  it("switches clusters without accepting or storing an upstream URL", () => {
    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useEndpoint(), { wrapper });

    act(() => result.current.setEndpoint("devnet"));

    expect(result.current.network).toBe("devnet");
    expect(result.current.endpointUrl).toBe(
      "http://localhost/api/rpc?cluster=devnet",
    );
    expect(localStorage.getItem(STORAGE_KEY)).toBe("devnet");
  });

  it("clears cached derived data when the cluster changes", () => {
    const { queryClient, wrapper } = createWrapper();
    queryClient.setQueryData(["validatorsTable", "weight"], ["mainnet-row"]);
    queryClient.setQueryData(["validatorsVoterSplits"], { mainnet: true });
    queryClient.setQueryData(["vote-accounts-with-validators"], {
      mainnet: true,
    });
    const { result } = renderHook(() => useEndpoint(), { wrapper });

    act(() => result.current.setEndpoint("devnet"));

    expect(
      queryClient.getQueryData(["validatorsTable", "weight"]),
    ).toBeUndefined();
    expect(queryClient.getQueryData(["validatorsVoterSplits"])).toBeUndefined();
    expect(
      queryClient.getQueryData(["vote-accounts-with-validators"]),
    ).toBeUndefined();

    queryClient.setQueryData(["validatorsVoterSplits"], { devnet: true });
    act(() => result.current.resetToDefault());
    expect(queryClient.getQueryData(["validatorsVoterSplits"])).toBeUndefined();
    expect(result.current.network).toBe("mainnet");
  });

  it("migrates the cluster from legacy storage and discards its URL", () => {
    localStorage.setItem(
      LEGACY_STORAGE_KEY,
      JSON.stringify({
        type: "testnet",
        url: "https://secret-rpc.example/key",
      }),
    );

    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useEndpoint(), { wrapper });

    expect(result.current.network).toBe("testnet");
    expect(result.current.endpointUrl).toBe(
      "http://localhost/api/rpc?cluster=testnet",
    );
    expect(localStorage.getItem(LEGACY_STORAGE_KEY)).toBeNull();
  });

  it("does not restore a legacy custom RPC", () => {
    localStorage.setItem(
      LEGACY_STORAGE_KEY,
      JSON.stringify({ type: "custom", url: "https://secret-rpc.example/key" }),
    );

    const { wrapper } = createWrapper();
    const { result } = renderHook(() => useEndpoint(), { wrapper });

    expect(result.current.network).toBe("mainnet");
    expect(localStorage.getItem(LEGACY_STORAGE_KEY)).toBeNull();
  });
});
