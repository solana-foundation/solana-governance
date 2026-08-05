import { renderHook } from "@testing-library/react";
import { PublicKey } from "@solana/web3.js";

const mockUseSupportAccounts = jest.fn();
const mockUseGetValidators = jest.fn();
const mockUseValidatorsTotalStakedLamports = jest.fn();

jest.mock("../useSupportAccounts", () => ({
  buildSupportFilters: () => [{ memcmp: { offset: 8, bytes: "x" } }],
  useSupportAccounts: () => mockUseSupportAccounts(),
}));
jest.mock("../useGetValidators", () => ({
  useGetValidators: () => mockUseGetValidators(),
}));
jest.mock("../useValidatorsTotalStakedLamports", () => ({
  useValidatorsTotalStakedLamports: () => mockUseValidatorsTotalStakedLamports(),
}));

import { useProposalSupporters } from "../useProposalSupporters";

const PROPOSAL = new PublicKey("11111111111111111111111111111111");
const SUPPORT = new PublicKey("SysvarC1ock11111111111111111111111111111111");
const IDENTITY = new PublicKey("Vote111111111111111111111111111111111111111");

const supportAccounts = [{ publicKey: SUPPORT, validator: IDENTITY }];

const validators = [
  {
    name: "Real Validator",
    vote_identity: IDENTITY.toBase58(),
    identity: IDENTITY.toBase58(),
    activated_stake: 25_000_000_000,
    commission: 5,
    epoch_credits: 1,
    last_vote: 1,
  },
];

beforeEach(() => {
  jest.clearAllMocks();
  mockUseSupportAccounts.mockReturnValue({
    data: supportAccounts,
    isLoading: false,
  });
});

describe("useProposalSupporters", () => {
  it("computes a stake percentage when validator data is available", () => {
    mockUseGetValidators.mockReturnValue({
      data: validators,
      isLoading: false,
    });
    mockUseValidatorsTotalStakedLamports.mockReturnValue({
      totalStakedLamports: 100_000_000_000,
    });

    const { result } = renderHook(() => useProposalSupporters(PROPOSAL));

    expect(result.current.data[0].validatorName).toBe("Real Validator");
    expect(result.current.data[0].stakedLamports).toBe(25_000_000_000);
    expect(result.current.data[0].stakePercentage).toBe(25);
  });

  it("leaves stake unknown rather than zero when validators fail to load", () => {
    // A zero denominator used to render every supporter as "0 SOL / 0.00%",
    // indistinguishable from a supporter that genuinely holds no stake.
    mockUseGetValidators.mockReturnValue({ data: undefined, isLoading: false });
    mockUseValidatorsTotalStakedLamports.mockReturnValue({
      totalStakedLamports: undefined,
    });

    const { result } = renderHook(() => useProposalSupporters(PROPOSAL));

    expect(result.current.data).toHaveLength(1);
    expect(result.current.data[0].stakedLamports).toBeUndefined();
    expect(result.current.data[0].stakePercentage).toBeUndefined();
  });

  it("still lists a supporter that is missing from validator metadata", () => {
    // The supporter is real — it came from an on-chain Support account — so the
    // row must stay, with its stake marked unknown.
    mockUseGetValidators.mockReturnValue({ data: [], isLoading: false });
    mockUseValidatorsTotalStakedLamports.mockReturnValue({
      totalStakedLamports: 100_000_000_000,
    });

    const { result } = renderHook(() => useProposalSupporters(PROPOSAL));

    expect(result.current.data).toHaveLength(1);
    expect(result.current.data[0].validatorName).toBe("Unknown Validator");
    expect(result.current.data[0].stakePercentage).toBeUndefined();
  });
});
