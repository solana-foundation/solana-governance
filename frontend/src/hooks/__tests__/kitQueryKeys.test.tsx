import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import React from "react";

jest.mock("@/contexts/EndpointContext", () => ({
  useEndpoint: () => ({ endpointUrl: "http://localhost:8899" }),
}));

jest.mock("@/contexts/GovernanceConfigContext", () => ({
  useGovernanceConfigContext: () => ({
    data: {
      discussionEpochs: 1,
      maxSupportEpochs: 1,
      snapshotEpochExtension: 1,
      votingEpochs: 1,
    },
    isLoading: false,
    isPending: false,
    isSuccess: true,
  }),
}));

const epochData = {
  epochInfo: {
    absoluteSlot: 1n,
    epoch: 123n,
    slotIndex: 0n,
    slotsInEpoch: 432_000n,
  },
  epochSchedule: {
    firstNormalEpoch: 0n,
    firstNormalSlot: 0n,
    leaderScheduleSlotOffset: 432_000n,
    slotsPerEpoch: 432_000n,
    warmup: false,
  },
};

jest.mock("../useEpochInfo", () => ({
  useEpochInfo: () => ({ data: epochData, isLoading: false }),
}));

jest.mock("../useRawVoteAccounts", () => ({
  useRawVoteAccounts: () => ({
    data: { current: [], delinquent: [] },
    isLoading: false,
  }),
}));

jest.mock("@/data", () => ({
  getProposals: () => Promise.resolve([]),
}));

jest.mock("@/helpers/date", () => ({
  epochToDate: () => Promise.resolve(new Date(0)),
}));

import { useEpochToDate } from "../useEpochToDate";
import { useProposals } from "../useProposals";

function wrapper({ children }: { children: React.ReactNode }) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 } },
  });
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

describe("Kit bigint query keys", () => {
  it("allows proposal queries to use a Kit epoch value", async () => {
    const { result } = renderHook(() => useProposals(), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
  });

  it("allows epoch-to-date queries to use a Kit epoch value", async () => {
    const { result } = renderHook(() => useEpochToDate(124), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
  });
});
