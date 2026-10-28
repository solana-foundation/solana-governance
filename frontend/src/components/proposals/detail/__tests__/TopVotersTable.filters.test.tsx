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
  validatorVoteAddress: `${name} vote`,
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

const override = (
  validator: TopVoterRecord,
  stakeAccount: string,
): TopVoterRecord => ({
  ...validator,
  stakeAccount,
  walletType: "staker",
});

const [yesValidator, noValidator] = voters;
const overrides = [
  override(yesValidator, "stake-a"),
  override(yesValidator, "stake-b"),
  override(noValidator, "stake-c"),
];

const selectWithOption = (name: string) =>
  screen.getByRole("option", { name }).parentElement as HTMLSelectElement;

const voteFilterSelect = () => selectWithOption("All votes");

const shownVoters = () =>
  screen
    .getAllByRole("row")
    .slice(1)
    .map(
      (row) =>
        voters.find((v) => row.textContent?.includes(v.validatorName))
          ?.validatorName,
    );

const STAKE_ACCOUNTS = overrides.map((o) => o.stakeAccount);

// Overrides render their stake account under "Voted as"; validators don't.
const shownRows = () =>
  screen
    .getAllByRole("row")
    .slice(1)
    .map((row) => {
      const text = row.textContent ?? "";
      const name = voters.find((v) => text.includes(v.validatorName));
      const stake = STAKE_ACCOUNTS.find((s) => s && text.includes(s));
      return stake ? `${name?.validatorName} / ${stake}` : name?.validatorName;
    })
    .sort();

const mockVotes = (data: TopVoterRecord[]) =>
  jest.mocked(useProposalVotes).mockReturnValue({
    data,
    isLoading: false,
  } as ReturnType<typeof useProposalVotes>);

beforeEach(() => mockVotes(voters));

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

  it("sorts the Voter Split by For, then Against, then Abstain", () => {
    render(<TopVotersTable proposal={undefined} />);
    const header = screen.getByRole("button", { name: /voter split/i });

    fireEvent.click(header);
    expect(shownVoters()).toEqual([
      "Abstain Validator",
      "No Validator",
      "Split Validator",
      "Yes Validator",
    ]);

    fireEvent.click(header);
    expect(shownVoters()).toEqual([
      "Yes Validator",
      "Split Validator",
      "No Validator",
      "Abstain Validator",
    ]);
  });
});

describe("TopVotersTable override filters", () => {
  beforeEach(() => mockVotes([...voters, ...overrides]));

  it("separates staker overrides from validator votes", () => {
    render(<TopVotersTable proposal={undefined} />);
    const voterTypeSelect = selectWithOption("All voters");

    fireEvent.change(voterTypeSelect, { target: { value: "staker" } });
    expect(shownRows()).toEqual([
      "No Validator / stake-c",
      "Yes Validator / stake-a",
      "Yes Validator / stake-b",
    ]);

    fireEvent.change(voterTypeSelect, { target: { value: "validator" } });
    expect(shownRows()).toEqual(voters.map((v) => v.validatorName).sort());
  });

  it("shows one validator's vote alongside only its own overrides", () => {
    render(<TopVotersTable proposal={undefined} />);

    expect(
      screen.getByRole("button", { name: "1 override" }),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "2 overrides" }));

    expect(shownRows()).toEqual([
      "Yes Validator",
      "Yes Validator / stake-a",
      "Yes Validator / stake-b",
    ]);

    fireEvent.click(
      screen.getByRole("button", { name: "Show all validators" }),
    );
    expect(shownRows()).toHaveLength(voters.length + overrides.length);
  });
});
