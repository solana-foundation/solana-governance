import * as React from "react";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

const mockFetchNcnJson = jest.fn();

// EndpointContext reaches env.ts, an ESM-only package Jest does not transform.
jest.mock("@/contexts/EndpointContext", () => ({
  useEndpoint: () => ({ network: "mainnet" }),
}));
jest.mock("@/contexts/NcnApiContext", () => ({
  useNcnApi: () => ({ ncnApiUrl: "https://ncn.example" }),
}));
jest.mock("@/lib/ncnApi", () => ({
  fetchNcnJson: (url: string) => mockFetchNcnJson(url),
}));

import { useSnapshotMeta, useSnapshotMetas } from "../useSnapshotMeta";

function metaFor(slot: number) {
  return { network: "mainnet", slot, total_active_stake: slot * 10 };
}

function makeWrapper() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return (
      <QueryClientProvider client={client}>{children}</QueryClientProvider>
    );
  };
}

function requestedSlot(url: string) {
  return new URL(url).searchParams.get("slot");
}

beforeEach(() => {
  jest.clearAllMocks();
  mockFetchNcnJson.mockImplementation((url: string) => {
    const slot = requestedSlot(url);
    return Promise.resolve(metaFor(slot === null ? 999 : Number(slot)));
  });
});

describe("useSnapshotMeta", () => {
  it("asks for the newest snapshot when given no slot", async () => {
    const { result } = renderHook(() => useSnapshotMeta(), {
      wrapper: makeWrapper(),
    });

    await waitFor(() => expect(result.current.data).toBeDefined());
    expect(requestedSlot(mockFetchNcnJson.mock.calls[0][0])).toBeNull();
  });

  it("asks for one snapshot when given its slot", async () => {
    const { result } = renderHook(() => useSnapshotMeta(500), {
      wrapper: makeWrapper(),
    });

    await waitFor(() => expect(result.current.data).toBeDefined());
    expect(requestedSlot(mockFetchNcnJson.mock.calls[0][0])).toBe("500");
    expect(result.current.data?.slot).toBe(500);
  });
});

describe("useSnapshotMetas", () => {
  it("keys each proposal's snapshot by its own slot", async () => {
    // The reported bug: every row was measured against whichever snapshot was
    // newest, so quorum resolved for at most one proposal.
    const { result } = renderHook(() => useSnapshotMetas([500, 600]), {
      wrapper: makeWrapper(),
    });

    await waitFor(() => expect(result.current.bySlot.size).toBe(2));
    expect(result.current.bySlot.get(500)?.total_active_stake).toBe(5_000);
    expect(result.current.bySlot.get(600)?.total_active_stake).toBe(6_000);
  });

  it("fetches a repeated slot once", async () => {
    const { result } = renderHook(() => useSnapshotMetas([500, 600, 500]), {
      wrapper: makeWrapper(),
    });

    await waitFor(() => expect(result.current.bySlot.size).toBe(2));
    expect(mockFetchNcnJson).toHaveBeenCalledTimes(2);
  });

  it("skips proposals with no snapshot slot", async () => {
    // snapshot_slot is 0 until activate_voting sets it.
    const { result } = renderHook(() => useSnapshotMetas([0, undefined, 500]), {
      wrapper: makeWrapper(),
    });

    await waitFor(() => expect(result.current.bySlot.size).toBe(1));
    expect(mockFetchNcnJson).toHaveBeenCalledTimes(1);
    expect(requestedSlot(mockFetchNcnJson.mock.calls[0][0])).toBe("500");
  });

  it("shares its cache with useSnapshotMeta for the same slot", async () => {
    // The detail page and the table must not fetch the same snapshot twice.
    const wrapper = makeWrapper();
    renderHook(() => useSnapshotMetas([500]), { wrapper });
    const { result } = renderHook(() => useSnapshotMeta(500), { wrapper });

    await waitFor(() => expect(result.current.data).toBeDefined());
    expect(mockFetchNcnJson).toHaveBeenCalledTimes(1);
  });
});
