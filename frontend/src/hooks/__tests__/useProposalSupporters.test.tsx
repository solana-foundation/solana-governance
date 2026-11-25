import type { Address } from "@solana/kit";
import { renderHook } from "@testing-library/react";

const mockUseSupportAccounts = jest.fn();
const mockUseGetValidators = jest.fn();
const mockUseValidatorsTotalStakedLamports = jest.fn();

jest.mock("../useSupportAccounts", () => ({
  buildSupportFilters: () => [{ name: "proposal", value: "x" }],
  useSupportAccounts: () => mockUseSupportAccounts(),
}));
jest.mock("../useGetValidators", () => ({
  useGetValidators: () => mockUseGetValidators(),
}));
jest.mock("../useValidatorsTotalStakedLamports", () => ({
  useValidatorsTotalStakedLamports: () => mockUseValidatorsTotalStakedLamports(),
}));

import { useProposalSupporters } from "../useProposalSupporters";

const PROPOSAL = "11111111111111111111111111111111" as Address;
const SUPPORT = "SysvarC1ock11111111111111111111111111111111";
const IDENTITY = "Vote111111111111111111111111111111111111111";

const validators = [
  {
    name: "Real Validator",
    vote_identity: IDENTITY,
    identity: IDENTITY,
    activated_stake: 25_000_000_000n,
    commission: 5,
    epoch_credits: 1n,
    last_vote: 1n,
  },
];

beforeEach(() => {
  jest.clearAllMocks();
  mockUseSupportAccounts.mockReturnValue({
    data: [{ publicKey: SUPPORT, validator: IDENTITY }],
    isLoading: false,
  });
});

describe("useProposalSupporters", () => {
  it("computes a stake percentage when validator data is available", () => {
    mockUseGetValidators.mockReturnValue({ data: validators, isLoading: false });
    mockUseValidatorsTotalStakedLamports.mockReturnValue({
      totalStakedLamports: 100_000_000_000n,
    });

    const { result } = renderHook(() => useProposalSupporters(PROPOSAL));

    expect(result.current.data[0].validatorName).toBe("Real Validator");
    expect(result.current.data[0].stakedLamports).toBe(25_000_000_000n);
    expect(result.current.data[0].stakePercentage).toBe(25);
  });

  it("keeps the fraction of a small supporter's share", () => {
    // The share used to be truncated to two decimals before it was stored, so
    // a validator holding 0.004% of stake was recorded as holding none.
    mockUseGetValidators.mockReturnValue({
      data: [{ ...validators[0], activated_stake: 4_000_000n }],
      isLoading: false,
    });
    mockUseValidatorsTotalStakedLamports.mockReturnValue({
      totalStakedLamports: 100_000_000_000n,
    });

    const { result } = renderHook(() => useProposalSupporters(PROPOSAL));

    expect(result.current.data[0].stakePercentage).toBe(0.004);
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
      totalStakedLamports: 100_000_000_000n,
    });

    const { result } = renderHook(() => useProposalSupporters(PROPOSAL));

    expect(result.current.data).toHaveLength(1);
    expect(result.current.data[0].validatorName).toBe("Unknown Validator");
    expect(result.current.data[0].stakePercentage).toBeUndefined();
  });
});
