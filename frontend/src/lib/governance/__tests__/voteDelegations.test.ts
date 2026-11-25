import type { Address } from "@solana/kit";
import { fetchVoteDelegations } from "../programAccounts";
import { resolveSignerVoteAccount } from "../voteDelegations";

jest.mock("../programAccounts", () => ({ fetchVoteDelegations: jest.fn() }));

const mockFetchVoteDelegations = jest.mocked(fetchVoteDelegations);
const rpc = {} as Parameters<typeof resolveSignerVoteAccount>[0];

const WALLET = "Wallet1111111111111111111111111111111111111" as Address;
const VOTE_A = "VoteA11111111111111111111111111111111111111" as Address;
const VOTE_B = "VoteB11111111111111111111111111111111111111" as Address;
const STAKER = "Staker1111111111111111111111111111111111111" as Address;

const account = (votePubkey: string, nodePubkey: string) => ({ nodePubkey, votePubkey });

/** On-chain vote delegations naming WALLET as delegate. */
function delegationsOf(...entries: { subject: Address; accepted: boolean }[]) {
  mockFetchVoteDelegations.mockResolvedValue(
    entries.map(({ subject, accepted }) => ({
      data: { delegate: WALLET, delegationAccepted: accepted, subject },
    })) as unknown as Awaited<ReturnType<typeof fetchVoteDelegations>>,
  );
}

beforeEach(() => {
  mockFetchVoteDelegations.mockReset();
});

describe("resolveSignerVoteAccount", () => {
  it("resolves a validator identity to its own vote account without reading delegations", async () => {
    const voteAccounts = { current: [account(VOTE_A, WALLET)], delinquent: [] };

    await expect(resolveSignerVoteAccount(rpc, voteAccounts, WALLET)).resolves.toBe(VOTE_A);
    expect(mockFetchVoteDelegations).not.toHaveBeenCalled();
  });

  it("does not treat the identity of a delinquent vote account as a validator", async () => {
    // Role detection reads only current vote accounts, so matching a delinquent
    // identity here would let a wallet the UI calls a non-validator sign as one.
    delegationsOf();
    const voteAccounts = { current: [], delinquent: [account(VOTE_A, WALLET)] };

    await expect(resolveSignerVoteAccount(rpc, voteAccounts, WALLET)).rejects.toThrow(
      "No SPL vote account found",
    );
  });

  it("resolves an accepted delegate to the vote account it acts for", async () => {
    delegationsOf(
      { subject: VOTE_A, accepted: true },
      // A pending delegation grants nothing, and a staker wallet is not a vote account.
      { subject: VOTE_B, accepted: false },
      { subject: STAKER, accepted: true },
    );
    const voteAccounts = {
      current: [account(VOTE_A, "IdentityA")],
      delinquent: [account(VOTE_B, "IdentityB")],
    };

    await expect(resolveSignerVoteAccount(rpc, voteAccounts, WALLET)).resolves.toBe(VOTE_A);
  });

  it("refuses to pick one when the wallet is the delegate of several vote accounts", async () => {
    delegationsOf({ subject: VOTE_A, accepted: true }, { subject: VOTE_B, accepted: true });
    const voteAccounts = {
      current: [account(VOTE_A, "IdentityA"), account(VOTE_B, "IdentityB")],
      delinquent: [],
    };

    await expect(resolveSignerVoteAccount(rpc, voteAccounts, WALLET)).rejects.toThrow(
      "accepted vote delegate of 2 vote accounts",
    );
  });
});
