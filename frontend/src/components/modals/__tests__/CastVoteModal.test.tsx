import { fireEvent, render, screen } from "@testing-library/react";
import { captureException } from "@sentry/nextjs";
import type { Address } from "@solana/kit";
import { toast } from "sonner";
import { NCN_PROOF_NOT_FOUND_MESSAGE, NcnApiHttpError } from "@/lib/ncnApi";
import { TransactionFailedError } from "@/lib/transactionErrors";
import { WALLET_SIGNING_CANCELLED_MESSAGE } from "@/lib/walletSigning";
import { CastVoteModal } from "../CastVoteModal";

const mockCastVote = jest.fn();
// Stable across renders: the modal resets itself whenever this identity changes.
const mockResetDistribution = jest.fn();

jest.mock("@/hooks", () => ({
  useCastVote: () => ({ mutate: mockCastVote }),
  useHasUserVoted: () => ({ data: false, isLoading: false }),
  useValidatorVotingPower: () => ({ isLoading: false, votingPower: 1_000_000_000n }),
  useVoteDistribution: () => ({
    distribution: { abstain: 0, against: 0, for: 100 },
    handleOptionChange: jest.fn(),
    handleQuickSelect: jest.fn(),
    isValidDistribution: true,
    resetDistribution: mockResetDistribution,
    totalPercentage: 100,
  }),
  useWalletRole: () => ({ walletRole: "validator" }),
  VOTE_OPTIONS: ["for", "against", "abstain"],
}));
jest.mock("@solana/connector/react", () => ({
  useConnector: () => ({ account: "SysvarC1ock11111111111111111111111111111111" }),
}));
jest.mock("../../VotingProposalsDropdown", () => ({ VotingProposalsDropdown: () => null }));
jest.mock("sonner", () => ({
  toast: { error: jest.fn(), info: jest.fn(), success: jest.fn() },
}));
jest.mock("@sentry/nextjs", () => ({ captureException: jest.fn() }));

/** Submits the vote and has the mutation fail with `error`. */
function castVoteFailingWith(error: Error) {
  mockCastVote.mockImplementation((_input, { onError }) => onError(error));
  render(
    <CastVoteModal
      consensusResult={"Config1111111111111111111111111111111111111" as Address}
      isOpen
      onClose={jest.fn()}
      proposalId="SysvarRent111111111111111111111111111111111"
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: "Cast Vote" }));
}

beforeEach(() => {
  jest.clearAllMocks();
  jest.spyOn(console, "log").mockImplementation(() => {});
});

describe("CastVoteModal when the vote fails", () => {
  it("treats a rejected wallet approval as a cancellation, not an error", () => {
    // The connector wraps the wallet's rejection in its own signing error,
    // which used to raise an error toast and a Sentry report.
    castVoteFailingWith(
      Object.assign(new Error("Failed to sign transaction 1 of 1"), {
        code: "SIGNING_FAILED",
        originalError: Object.assign(new Error("User rejected the request."), { code: 4001 }),
      }),
    );

    expect(toast.info).toHaveBeenCalledWith(WALLET_SIGNING_CANCELLED_MESSAGE);
    expect(toast.error).not.toHaveBeenCalled();
    expect(captureException).not.toHaveBeenCalled();
  });

  it("explains that a validator absent from the snapshot cannot vote", () => {
    castVoteFailingWith(
      new NcnApiHttpError("vote account proof", 404, {
        resource: "vote-account-proof",
        url: "https://ncn.example/proof",
      }),
    );

    expect(toast.info).toHaveBeenCalledWith(NCN_PROOF_NOT_FOUND_MESSAGE);
    expect(captureException).not.toHaveBeenCalled();
  });

  it("shows the reason a transaction failed and reports it", () => {
    const failure = new TransactionFailedError(
      "Transaction failed: Voting on proposal not yet started",
    );

    castVoteFailingWith(failure);

    expect(screen.getByText("Transaction failed: Voting on proposal not yet started")).toBeInTheDocument();
    expect(captureException).toHaveBeenCalledWith(failure);
  });
});
