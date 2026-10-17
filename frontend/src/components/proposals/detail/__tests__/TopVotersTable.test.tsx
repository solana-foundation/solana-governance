import BN from "bn.js";
import { render, screen } from "@testing-library/react";
import { useProposalVotes } from "@/hooks/useProposalVotes";
import type { TopVoterRecord } from "@/types";
import TopVotersTable from "../TopVotersTable";

jest.mock("@/hooks/useProposalVotes", () => ({
  useProposalVotes: jest.fn(),
}));

jest.mock("../TopVotersColumns", () => ({
  topVoterColumns: [
    { accessorKey: "validatorName", header: "Voter" },
    { accessorKey: "stakedLamports", header: "Staked" },
  ],
}));

const voter: TopVoterRecord = {
  id: "validator-1",
  validatorName: "Validator One",
  validatorIdentity: "validator-address",
  stakeAccount: "stake-address",
  stakedLamports: 1_500_000_000,
  votePercentage: 12.5,
  voteTimestamp: "2026-08-28T12:34:56.000Z",
  voteData: {
    forVotesBp: new BN(7_500),
    againstVotesBp: new BN(2_000),
    abstainVotesBp: new BN(500),
  },
  accentColor: "#000",
  walletType: "staker",
};

const mockUseProposalVotes = jest.mocked(useProposalVotes);

describe("TopVotersTable CSV export", () => {
  it("disables download while cached votes are being refetched", () => {
    mockUseProposalVotes.mockReturnValue({
      data: [voter],
      isLoading: false,
      isFetching: true,
    } as ReturnType<typeof useProposalVotes>);

    render(<TopVotersTable proposal={undefined} />);

    expect(
      screen.getByRole("button", { name: "Download top voters" }),
    ).toBeDisabled();
  });

  it("enables download when the populated vote query is settled", () => {
    mockUseProposalVotes.mockReturnValue({
      data: [voter],
      isLoading: false,
      isFetching: false,
    } as ReturnType<typeof useProposalVotes>);

    render(<TopVotersTable proposal={undefined} />);

    expect(
      screen.getByRole("button", { name: "Download top voters" }),
    ).toBeEnabled();
  });
});
