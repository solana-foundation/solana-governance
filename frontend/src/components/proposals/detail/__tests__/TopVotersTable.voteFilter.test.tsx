import BN from "bn.js";
import type { PropsWithChildren } from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { useProposalVotes } from "@/hooks/useProposalVotes";
import type { TopVoterRecord } from "@/types";
import TopVotersTable from "../TopVotersTable";

jest.mock("@/hooks/useProposalVotes", () => ({
  useProposalVotes: jest.fn(),
}));

jest.mock("@/components/governance/shared/CopyableAddressIcon", () => ({
  CopyableAddressIcon: () => null,
}));

// Radix Select doesn't open in jsdom, so drive the same props with a native select.
jest.mock("@/components/ui/select", () => ({
  Select: ({
    value,
    onValueChange,
    children,
  }: PropsWithChildren<{
    value: string;
    onValueChange: (value: string) => void;
  }>) => (
    <select value={value} onChange={(e) => onValueChange(e.target.value)}>
      {children}
    </select>
  ),
  SelectTrigger: () => null,
  SelectValue: () => null,
  SelectContent: ({ children }: PropsWithChildren) => <>{children}</>,
  SelectItem: ({ value, children }: PropsWithChildren<{ value: string }>) => (
    <option value={value}>{children}</option>
  ),
}));

const voter = (
  name: string,
  forBp: number,
  againstBp: number,
  abstainBp: number,
): TopVoterRecord => ({
  id: name,
  validatorName: name,
  validatorIdentity: `${name} identity`,
  stakedLamports: 1_000_000_000,
  votePercentage: 1,
  voteTimestamp: "2026-08-28T12:34:56.000Z",
  voteData: {
    forVotesBp: new BN(forBp),
    againstVotesBp: new BN(againstBp),
    abstainVotesBp: new BN(abstainBp),
  },
  accentColor: "#000",
  walletType: "validator",
});

const voters = [
  voter("Yes Validator", 10_000, 0, 0),
  voter("No Validator", 0, 10_000, 0),
  voter("Split Validator", 6_000, 3_000, 1_000),
  voter("Abstain Validator", 0, 0, 10_000),
];

const voteFilterSelect = () =>
  screen.getByRole("option", { name: "All votes" })
    .parentElement as HTMLSelectElement;

const shownVoters = () =>
  screen
    .getAllByRole("row")
    .slice(1)
    .map(
      (row) =>
        voters.find((v) => row.textContent?.includes(v.validatorName))
          ?.validatorName,
    );

beforeEach(() => {
  jest.mocked(useProposalVotes).mockReturnValue({
    data: voters,
    isLoading: false,
  } as ReturnType<typeof useProposalVotes>);
});

describe("TopVotersTable vote filter", () => {
  it("keeps a split vote under each side it voted for", () => {
    render(<TopVotersTable proposal={undefined} />);
    const shownFor = (value: string) => {
      fireEvent.change(voteFilterSelect(), { target: { value } });
      return shownVoters().sort();
    };

    expect(shownFor("for")).toEqual(["Split Validator", "Yes Validator"]);
    expect(shownFor("against")).toEqual(["No Validator", "Split Validator"]);
    expect(shownFor("abstain")).toEqual([
      "Abstain Validator",
      "Split Validator",
    ]);
  });

  it("clears the vote filter on reset", () => {
    render(<TopVotersTable proposal={undefined} />);

    fireEvent.change(voteFilterSelect(), {
      target: { value: "abstain" },
    });
    expect(shownVoters().sort()).toEqual([
      "Abstain Validator",
      "Split Validator",
    ]);

    fireEvent.click(screen.getByRole("button", { name: "Reset" }));
    expect(shownVoters()).toHaveLength(4);
  });

  it("sorts by the For share from the Voter Split header", () => {
    render(<TopVotersTable proposal={undefined} />);
    const header = screen.getByRole("button", { name: /voter split/i });

    fireEvent.click(header);
    expect(shownVoters().slice(-2)).toEqual([
      "Split Validator",
      "Yes Validator",
    ]);

    fireEvent.click(header);
    expect(shownVoters().slice(0, 2)).toEqual([
      "Yes Validator",
      "Split Validator",
    ]);
  });
});
