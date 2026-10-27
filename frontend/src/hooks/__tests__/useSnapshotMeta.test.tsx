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

/** Every slot resolves except 700, standing in for one the cleanup job pruned. */
const PRUNED_SLOT = 700;

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

function lastUrl() {
  return new URL(mockFetchNcnJson.mock.calls.at(-1)?.[0]);
}

beforeEach(() => {
  jest.clearAllMocks();
  mockFetchNcnJson.mockImplementation((url: string) => {
    const params = new URL(url).searchParams;
    const slots = params.get("slots");

    if (slots !== null) {
      return Promise.resolve(
        slots
          .split(",")
          .map(Number)
          .filter((slot) => slot !== PRUNED_SLOT)
          .map(metaFor),
      );
    }

    const slot = params.get("slot");
    return Promise.resolve(metaFor(slot === null ? 999 : Number(slot)));
  });
});

describe("useSnapshotMeta", () => {
  it("asks for the newest snapshot when given no slot", async () => {
    const { result } = renderHook(() => useSnapshotMeta(), {
      wrapper: makeWrapper(),
    });

    await waitFor(() => expect(result.current.data).toBeDefined());
    expect(lastUrl().searchParams.get("slot")).toBeNull();
  });

  it("asks for one snapshot when given its slot", async () => {
    const { result } = renderHook(() => useSnapshotMeta(500), {
      wrapper: makeWrapper(),
    });

    await waitFor(() => expect(result.current.data).toBeDefined());
    expect(lastUrl().searchParams.get("slot")).toBe("500");
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

  it("asks for every slot in a single request", async () => {
    // One request per row would exhaust the verifier's burst limit on a page
    // holding more proposals than the limit allows.
    const { result } = renderHook(
      () => useSnapshotMetas([500, 600, 500, 800]),
      { wrapper: makeWrapper() },
    );

    await waitFor(() => expect(result.current.bySlot.size).toBe(3));
    expect(mockFetchNcnJson).toHaveBeenCalledTimes(1);
    expect(lastUrl().searchParams.get("slots")).toBe("500,600,800");
  });

  it("splits a history longer than one request accepts", async () => {
    // The verifier rejects an over-long slot list, and a rejected batch would
    // leave every row without a denominator rather than just the overflow.
    const slots = Array.from({ length: 250 }, (_, index) => 1_000 + index);

    const { result } = renderHook(() => useSnapshotMetas(slots), {
      wrapper: makeWrapper(),
    });

    await waitFor(() => expect(result.current.bySlot.size).toBe(250));
    expect(mockFetchNcnJson).toHaveBeenCalledTimes(3);
    for (const call of mockFetchNcnJson.mock.calls) {
      const requested = new URL(call[0]).searchParams.get("slots")!.split(",");
      expect(requested.length).toBeLessThanOrEqual(100);
    }
  });

  it("omits a slot the response does not carry", async () => {
    const { result } = renderHook(() => useSnapshotMetas([500, PRUNED_SLOT]), {
      wrapper: makeWrapper(),
    });

    await waitFor(() => expect(result.current.bySlot.size).toBe(1));
    expect(result.current.bySlot.has(PRUNED_SLOT)).toBe(false);
  });

  it("skips proposals with no snapshot slot", async () => {
    // snapshot_slot is 0 until activate_voting sets it.
    const { result } = renderHook(() => useSnapshotMetas([0, undefined, 500]), {
      wrapper: makeWrapper(),
    });

    await waitFor(() => expect(result.current.bySlot.size).toBe(1));
    expect(lastUrl().searchParams.get("slots")).toBe("500");
  });

  it("makes no request when nothing has a snapshot yet", async () => {
    const { result } = renderHook(() => useSnapshotMetas([0, undefined]), {
      wrapper: makeWrapper(),
    });

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(mockFetchNcnJson).not.toHaveBeenCalled();
    expect(result.current.bySlot.size).toBe(0);
  });
});
