import * as React from "react";
import type { Address } from "@solana/kit";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

const mockGetProposalVotes = jest.fn();
const mockGetProposalVoteOverrides = jest.fn();
const mockUseGetValidators = jest.fn();

// EndpointContext reaches env.ts, an ESM-only package Jest does not transform.
jest.mock("@/contexts/EndpointContext", () => ({
  useEndpoint: () => ({ endpointUrl: "https://rpc.example" }),
}));
jest.mock("@/data", () => ({
  getProposalVotes: () => mockGetProposalVotes(),
  getProposalVoteOverrides: () => mockGetProposalVoteOverrides(),
}));
jest.mock("../useGetValidators", () => ({
  useGetValidators: () => mockUseGetValidators(),
}));

import { useProposalVotes } from "../useProposalVotes";

const PROPOSAL = "7QJD8MzheHWJLHS39NkoAbFCGFKg5d9QbVviRqD4YExP" as Address;
const IDENTITY = "D3htsc6iRQJLqCNWcC2xcZgUuvcd1JT8zoYNqraNcTQz";
const VOTE_ACCOUNT = "3xjfK9C9YNcta8MvK1US4sQ3bc6DEjoJoR3qLExGf9xE";
const VOTE_RECORD = "2RMEME7xh3UyMwDTec7pYgEnss8vuexdZWKZAvEcN4RH";
const STAKE_ACCOUNT = "13GD6r7kLNa2n2qgHBb6U2jq8gTX9cLsTHc4djS7EgeG";

const voteData = {
  forVotesBp: 10_000n,
  againstVotesBp: 0n,
  abstainVotesBp: 0n,
  voteTimestamp: 1_786_000_000n,
};

function makeWrapper() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
  };
}

beforeEach(() => {
  jest.clearAllMocks();
  mockUseGetValidators.mockReturnValue({
    data: [
      {
        name: "Real Validator",
        vote_identity: VOTE_ACCOUNT,
        identity: IDENTITY,
        activated_stake: 100_000_000_000n,
      },
    ],
  });
  // A validator's Vote records its node identity, while a staker's
  // VoteOverride records the validator's vote account.
  mockGetProposalVotes.mockResolvedValue([
    { ...voteData, identity: IDENTITY, voter: VOTE_RECORD, activeStake: 90_000_000_000n },
  ]);
  mockGetProposalVoteOverrides.mockResolvedValue([
    {
      ...voteData,
      identity: VOTE_ACCOUNT,
      stakeAccount: STAKE_ACCOUNT,
      voteAccountValidator: VOTE_RECORD,
      activeStake: 10_000_000_000n,
    },
  ]);
});

describe("useProposalVotes", () => {
  it("shows the validator identity on staker override rows too", async () => {
    const { result } = renderHook(() => useProposalVotes(PROPOSAL), {
      wrapper: makeWrapper(),
    });

    await waitFor(() => expect(result.current.data).toHaveLength(2));
    const [validatorRow, stakerRow] = result.current.data!;
    expect(validatorRow.validatorIdentity).toBe(IDENTITY);
    expect(stakerRow.walletType).toBe("staker");
    expect(stakerRow.validatorName).toBe("Real Validator");
    expect(stakerRow.validatorIdentity).toBe(IDENTITY);
  });
});
