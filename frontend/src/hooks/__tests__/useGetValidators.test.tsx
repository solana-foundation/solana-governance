import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import React from "react";

// helpers/contexts pull in env.ts (ESM-only, untransformed by jest), so stub the
// endpoint context the same way the instruction tests do.
jest.mock("@/contexts/EndpointContext", () => ({
  useEndpoint: () => ({ endpointUrl: "http://localhost:8899" }),
  RPC_URLS: { testnet: "http://localhost:8899" },
}));

const mockGetStakeWizValidators = jest.fn();
jest.mock("@/data", () => ({
  getStakeWizValidators: () => mockGetStakeWizValidators(),
}));

const mockGetVoteAccounts = jest.fn();
jest.mock("@/lib/rpcVoteAccounts", () => ({
  fetchRawVoteAccounts: () => mockGetVoteAccounts(),
}));

import { useGetValidators } from "../useGetValidators";
import { useValidatorsTotalStakedLamports } from "../useValidatorsTotalStakedLamports";

const VOTE_ACCOUNT = "Vote111111111111111111111111111111111111111";
const IDENTITY = "Node1111111111111111111111111111111111111111";

/** RPC stake is an integer in lamports; StakeWiz reports SOL as a float. */
const RPC_STAKE_LAMPORTS = 5_123_456_789_012_345;

function voteAccounts() {
  return {
    current: [
      {
        votePubkey: VOTE_ACCOUNT,
        nodePubkey: IDENTITY,
        activatedStake: RPC_STAKE_LAMPORTS,
        commission: 5,
        epochCredits: [[123, 0, 0]],
        lastVote: 999,
      },
    ],
    delinquent: [],
  };
}

function stakeWiz() {
  return {
    data: [
      {
        name: "Real Validator",
        vote_identity: VOTE_ACCOUNT,
        // Same stake expressed in SOL, as StakeWiz reports it.
        activated_stake: RPC_STAKE_LAMPORTS / 1e9,
        commission: 5,
        epoch_credits: 123,
        last_vote: 999,
      },
    ],
  };
}

function wrapper({ children }: { children: React.ReactNode }) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 } },
  });
  return (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
}

beforeEach(() => {
  jest.clearAllMocks();
});

describe("useGetValidators", () => {
  it("keeps correct stake when StakeWiz is down, losing only display metadata", async () => {
    // The reported failure: a third-party outage used to blank the whole
    // governance UI to zeros while reporting success, because the hook returned
    // [] unless BOTH sources resolved.
    mockGetStakeWizValidators.mockRejectedValue(new Error("stakewiz 503"));
    mockGetVoteAccounts.mockResolvedValue(voteAccounts());

    const { result } = renderHook(() => useGetValidators(), { wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toHaveLength(1);
    expect(result.current.data?.[0].activated_stake).toBe(RPC_STAKE_LAMPORTS);
    expect(result.current.data?.[0].name).toBe("Unknown validator #1");
    expect(result.current.data?.[0].vote_identity).toBe(VOTE_ACCOUNT);
  });

  it("surfaces an RPC failure as a query error rather than an empty list", async () => {
    // Returning [] made react-query record a success, so nothing retried and
    // every consumer divided by a zero denominator.
    mockGetStakeWizValidators.mockResolvedValue(stakeWiz());
    mockGetVoteAccounts.mockRejectedValue(new Error("rpc down"));

    const { result } = renderHook(() => useGetValidators(), { wrapper });
    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(result.current.data).toBeUndefined();
  });

  it("takes stake from the RPC, not StakeWiz, when both resolve", async () => {
    // StakeWiz samples stake independently of the RPC, so matched and unmatched
    // validators used to carry differently-sourced numbers in the same array.
    // Give the two sources different values and pin which one wins.
    mockGetStakeWizValidators.mockResolvedValue({
      data: [{ ...stakeWiz().data[0], activated_stake: 1 }], // 1 SOL, stale
    });
    mockGetVoteAccounts.mockResolvedValue(voteAccounts());

    const { result } = renderHook(() => useGetValidators(), { wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data?.[0].name).toBe("Real Validator");
    expect(result.current.data?.[0].activated_stake).toBe(RPC_STAKE_LAMPORTS);
  });

  it("fills in the identity from the RPC when metadata omits it", async () => {
    // Governance votes key on validator identity; StakeWiz does not always
    // carry it, and consumers index by both.
    mockGetStakeWizValidators.mockResolvedValue(stakeWiz());
    mockGetVoteAccounts.mockResolvedValue(voteAccounts());

    const { result } = renderHook(() => useGetValidators(), { wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data?.[0].identity).toBe(IDENTITY);
  });
});

describe("useValidatorsTotalStakedLamports", () => {
  it("reports an unknown denominator rather than zero when the RPC fails", async () => {
    // Regression (PR #121 review): throwing surfaced the query error, but the
    // derived hook still collapsed the undefined data into 0, so consumers
    // divided by zero and rendered "0.00%" — indistinguishable from a real
    // tally. Unknown must stay unknown all the way to the consumer.
    mockGetStakeWizValidators.mockResolvedValue(stakeWiz());
    mockGetVoteAccounts.mockRejectedValue(new Error("rpc down"));

    const { result } = renderHook(() => useValidatorsTotalStakedLamports(), {
      wrapper,
    });
    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(result.current.totalStakedLamports).toBeUndefined();
  });

  it("sums activated stake once the query resolves", async () => {
    mockGetStakeWizValidators.mockResolvedValue(stakeWiz());
    mockGetVoteAccounts.mockResolvedValue(voteAccounts());

    const { result } = renderHook(() => useValidatorsTotalStakedLamports(), {
      wrapper,
    });
    await waitFor(() =>
      expect(result.current.totalStakedLamports).toBe(RPC_STAKE_LAMPORTS),
    );
  });
});
