import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";
import { getProposalVoteOverrides } from "../getProposalVoteOverrides";

const all = jest.fn();
jest.mock("@/chain", () => ({
  createProgramWitDummyWallet: () => ({
    account: { voteOverride: { all: (...args: unknown[]) => all(...args) } },
  }),
}));

// 2^53 + 1 lamports (~9.007M SOL): BN.toNumber() throws above 2^53, and
// single mainnet stake accounts are already past 8M SOL.
const UNSAFE_STAKE = new BN(2).pow(new BN(53)).addn(1);

function overrideAccount(stakeAmount: BN) {
  return {
    publicKey: PublicKey.unique(),
    account: {
      delegator: PublicKey.unique(),
      stakeAccount: PublicKey.unique(),
      validator: PublicKey.unique(),
      proposal: PublicKey.unique(),
      voteAccountValidator: PublicKey.unique(),
      forVotesBp: new BN(10_000),
      againstVotesBp: new BN(0),
      abstainVotesBp: new BN(0),
      stakeAmount,
      voteOverrideTimestamp: new BN(1),
      bump: 255,
      forVotesLamports: stakeAmount,
      againstVotesLamports: new BN(0),
      abstainVotesLamports: new BN(0),
    },
  };
}

describe("getProposalVoteOverrides", () => {
  it("maps a stake amount above 2^53 lamports without throwing", async () => {
    all.mockResolvedValueOnce([overrideAccount(UNSAFE_STAKE)]);

    const [override] = await getProposalVoteOverrides(
      PublicKey.unique(),
      "http://localhost",
    );

    expect(override.activeStake).toBe(Number(UNSAFE_STAKE.toString()));
    expect(Number.isFinite(override.activeStake)).toBe(true);
  });

  it("keeps exact values below 2^53", async () => {
    all.mockResolvedValueOnce([overrideAccount(new BN(123_456_789))]);

    const [override] = await getProposalVoteOverrides(
      PublicKey.unique(),
      "http://localhost",
    );

    expect(override.activeStake).toBe(123_456_789);
  });
});
