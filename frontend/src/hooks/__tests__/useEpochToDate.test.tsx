import * as React from "react";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

const mockEpochToDate = jest.fn();
const mockUseEpochInfo = jest.fn();

// EndpointContext reaches env.ts, an ESM-only package Jest does not transform.
jest.mock("@/contexts/EndpointContext", () => ({
  useEndpoint: () => ({ endpointUrl: "http://localhost:8899" }),
}));
jest.mock("@/helpers/date", () => ({
  epochToDate: (...args: unknown[]) => mockEpochToDate(...args),
}));
jest.mock("../useEpochInfo", () => ({
  useEpochInfo: () => mockUseEpochInfo(),
}));

import { useEpochToDate } from "../useEpochToDate";

const TARGET_EPOCH = 1024n;

function epochInfo(epoch: bigint, absoluteSlot: bigint) {
  return {
    data: {
      epochInfo: { epoch, absoluteSlot, slotIndex: 0n, slotsInEpoch: 432_000n },
      epochSchedule: {
        firstNormalEpoch: 0n,
        firstNormalSlot: 0n,
        leaderScheduleSlotOffset: 432_000n,
        slotsPerEpoch: 432_000n,
        warmup: false,
      },
    },
    isLoading: false,
  };
}

/** One client for the whole render, so the cache survives rerenders. */
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

beforeEach(() => {
  jest.clearAllMocks();
  mockEpochToDate.mockResolvedValue(new Date("2026-09-01T00:00:00Z"));
});

describe("useEpochToDate", () => {
  it("does not recompute when only the slot advances", async () => {
    // The reported bug: absoluteSlot was in the query key, so every refresh of
    // useEpochInfo produced a new cache entry instead of reusing one. Each was a
    // fresh miss, so staleTime never applied and epochToDate — which makes one
    // or two RPC calls — ran again for an answer that had not changed.
    mockUseEpochInfo.mockReturnValue(epochInfo(1020n, 500_000_000n));

    const { rerender } = renderHook(() => useEpochToDate(TARGET_EPOCH), {
      wrapper: makeWrapper(),
    });

    await waitFor(() => expect(mockEpochToDate).toHaveBeenCalledTimes(1));

    // Same epoch, later slot — what a useEpochInfo refetch looks like.
    mockUseEpochInfo.mockReturnValue(epochInfo(1020n, 500_000_750n));
    rerender();
    mockUseEpochInfo.mockReturnValue(epochInfo(1020n, 500_001_500n));
    rerender();

    await waitFor(() => expect(mockEpochToDate).toHaveBeenCalledTimes(1));
  });

  it("recomputes when the epoch rolls over", async () => {
    // The projection reads the epoch's actual block time once the target is no
    // longer in the future, so a rollover must not wait for the interval.
    mockUseEpochInfo.mockReturnValue(epochInfo(1020n, 500_000_000n));

    const { rerender } = renderHook(() => useEpochToDate(TARGET_EPOCH), {
      wrapper: makeWrapper(),
    });

    await waitFor(() => expect(mockEpochToDate).toHaveBeenCalledTimes(1));

    mockUseEpochInfo.mockReturnValue(epochInfo(1021n, 500_432_000n));
    rerender();

    await waitFor(() => expect(mockEpochToDate).toHaveBeenCalledTimes(2));
  });

  it("computes separately for each target epoch", async () => {
    // A proposal detail page mounts three of these, one per phase boundary.
    mockUseEpochInfo.mockReturnValue(epochInfo(1020n, 500_000_000n));
    const wrapper = makeWrapper();

    renderHook(() => useEpochToDate(1021n), { wrapper });
    renderHook(() => useEpochToDate(1024n), { wrapper });

    await waitFor(() => expect(mockEpochToDate).toHaveBeenCalledTimes(2));
    expect(mockEpochToDate.mock.calls.map((call) => call[0])).toEqual([
      1021n, 1024n,
    ]);
  });
});
