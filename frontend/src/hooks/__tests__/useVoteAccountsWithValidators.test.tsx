import * as React from "react";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { PublicKey } from "@solana/web3.js";

const mockUseGetValidators = jest.fn();
const mockUseVoteAccounts = jest.fn();

jest.mock("../useGetValidators", () => ({
  useGetValidators: () => mockUseGetValidators(),
}));
jest.mock("../useVoteAccounts", () => ({
  useVoteAccounts: () => mockUseVoteAccounts(),
}));

import { useVoteAccountsWithValidators } from "../useVoteAccountsWithValidators";

const key = (byte: number) => new PublicKey(new Uint8Array(32).fill(byte));

/** A validator signs governance votes with its identity, not its vote account. */
const IDENTITY = key(1);
const VOTE_ACCOUNT = key(2);
const VOTE_PDA = key(3);

const validator = {
  name: "Real Validator",
  vote_identity: VOTE_ACCOUNT.toBase58(),
  identity: IDENTITY.toBase58(),
  activated_stake: 25_000_000_000,
  commission: 5,
  epoch_credits: 111,
  last_vote: 222,
  credits: 333,
};

/** A validator the vote record does not belong to. */
const OTHER_VALIDATOR = {
  ...validator,
  vote_identity: key(8).toBase58(),
  identity: key(9).toBase58(),
};

/** As mapVoteAccountDto builds it: validator metadata is left unset. */
const voteRecord = {
  voteAccount: VOTE_PDA,
  identity: IDENTITY,
  activeStake: 25_000_000_000,
};

function wrapper({ children }: { children: React.ReactNode }) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

function renderWith(validators: unknown[], votes: unknown[]) {
  mockUseGetValidators.mockReturnValue({
    data: validators,
    isLoading: false,
  });
  mockUseVoteAccounts.mockReturnValue({ data: votes, isLoading: false });

  return renderHook(() => useVoteAccountsWithValidators(), { wrapper });
}

beforeEach(() => {
  jest.clearAllMocks();
  jest.spyOn(console, "warn").mockImplementation(() => {});
});

afterEach(() => {
  jest.restoreAllMocks();
});

describe("useVoteAccountsWithValidators", () => {
  it("matches a vote record on the validator identity it records", async () => {
    // The reported bug: the lookup compared the recorded identity against
    // vote_identity, which holds the vote account address, so no row ever
    // matched and every one kept the placeholder metadata.
    const { result } = renderWith([validator], [voteRecord]);

    await waitFor(() => expect(result.current.data).toBeDefined());

    const entry = result.current.data!.voteMap[VOTE_PDA.toBase58()];
    expect(entry.validator?.name).toBe("Real Validator");
    expect(entry.voteAccount.commission).toBe(5);
    expect(entry.voteAccount.lastVote).toBe(222);
  });

  it("also matches on the vote account address", async () => {
    // Sibling hooks index both keys; a record carrying the vote account instead
    // of the identity has to resolve the same validator.
    const { result } = renderWith(
      [validator],
      [{ ...voteRecord, identity: VOTE_ACCOUNT }],
    );

    await waitFor(() => expect(result.current.data).toBeDefined());

    expect(
      result.current.data!.voteMap[VOTE_PDA.toBase58()].validator?.name,
    ).toBe("Real Validator");
  });

  it("leaves metadata unset when no validator matches", async () => {
    // Zeros here would render as a real 0% commission with no credits, which is
    // indistinguishable from a validator that genuinely has those values.
    const { result } = renderWith([OTHER_VALIDATOR], [voteRecord]);

    await waitFor(() => expect(result.current.data).toBeDefined());

    const entry = result.current.data!.voteMap[VOTE_PDA.toBase58()];
    expect(entry.validator).toBeUndefined();
    expect(entry.voteAccount.commission).toBeUndefined();
    expect(entry.voteAccount.lastVote).toBeUndefined();
    expect(entry.voteAccount.credits).toBeUndefined();
  });

  it("reports how many records went unmatched", async () => {
    // The previous code swallowed this, so a wholly broken join looked normal.
    const warn = jest.spyOn(console, "warn");

    const { result } = renderWith([OTHER_VALIDATOR], [voteRecord]);
    await waitFor(() => expect(result.current.data).toBeDefined());

    expect(warn).toHaveBeenCalledWith(expect.stringContaining("1 of 1"));
  });
});
