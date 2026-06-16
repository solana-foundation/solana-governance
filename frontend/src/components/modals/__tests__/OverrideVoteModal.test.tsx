import React from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { PublicKey } from "@solana/web3.js";
import { OverrideVoteModal } from "../OverrideVoteModal";
import { WalletRole } from "@/types";

const mockMutate = jest.fn();
const mockUseChainVoteAccount = jest.fn();
const mockUseWalletRole = jest.fn();
const mockUseWalletStakeAccounts = jest.fn();
const mockUseVoteOverrideAccounts = jest.fn();
const mockHandleOptionChange = jest.fn();
const mockHandleQuickSelect = jest.fn();
const mockResetDistribution = jest.fn();

jest.mock("@solana/wallet-adapter-react", () => ({
  useAnchorWallet: jest.fn(),
}));

jest.mock("@/hooks", () => ({
  useCastVoteOverride: jest.fn(() => ({
    mutate: mockMutate,
  })),
  useChainVoteAccount: (...args: unknown[]) => mockUseChainVoteAccount(...args),
  useVoteDistribution: jest.fn(() => ({
    distribution: { for: 100, against: 0, abstain: 0 },
    totalPercentage: 100,
    isValidDistribution: true,
    handleOptionChange: mockHandleOptionChange,
    handleQuickSelect: mockHandleQuickSelect,
    resetDistribution: mockResetDistribution,
  })),
  useVoteOverrideAccounts: (...args: unknown[]) =>
    mockUseVoteOverrideAccounts(...args),
  useWalletRole: (...args: unknown[]) => mockUseWalletRole(...args),
  useWalletStakeAccounts: (...args: unknown[]) =>
    mockUseWalletStakeAccounts(...args),
  VOTE_OPTIONS: ["for", "against", "abstain"],
}));

jest.mock("../../StakeAccountsDropdown", () => ({
  StakeAccountsDropdown: ({
    onValueChange,
  }: {
    onValueChange: (value: string) => void;
  }) => {
    React.useEffect(() => {
      onValueChange("stake-account");
    }, [onValueChange]);

    return <div>Stake account: stake-account</div>;
  },
}));

jest.mock("../../VotingProposalsDropdown", () => ({
  VotingProposalsDropdown: () => <div>Proposal: proposal-id</div>,
}));

jest.mock("sonner", () => ({
  toast: {
    success: jest.fn(),
    error: jest.fn(),
  },
}));

jest.mock("@sentry/nextjs", () => ({
  captureException: jest.fn(),
}));

// eslint-disable-next-line @typescript-eslint/no-require-imports
const { useAnchorWallet } = require("@solana/wallet-adapter-react");

beforeAll(() => {
  globalThis.ResizeObserver = class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
});

describe("OverrideVoteModal", () => {
  const wallet = {
    publicKey: new PublicKey("11111111111111111111111111111111"),
    signTransaction: jest.fn(),
    signAllTransactions: jest.fn(),
  };

  const defaultProps = {
    isOpen: true,
    onClose: jest.fn(),
    proposalId: "proposal-id",
    consensusResult: new PublicKey("11111111111111111111111111111111"),
  };

  beforeEach(() => {
    jest.clearAllMocks();
    useAnchorWallet.mockReturnValue(wallet);
    mockUseWalletRole.mockReturnValue({ walletRole: WalletRole.STAKER });
    mockUseWalletStakeAccounts.mockReturnValue({
      data: [
        {
          activeStake: 1_000,
          stakeAccount: "stake-account",
          voteAccount: "vote-account",
        },
      ],
    });
    mockUseVoteOverrideAccounts.mockReturnValue({ data: [] });
    mockUseChainVoteAccount.mockReturnValue({
      data: null,
      isLoading: false,
    });
  });

  it("allows normal staker override votes without validator confirmation", async () => {
    render(<OverrideVoteModal {...defaultProps} />);

    expect(screen.getByText("Stake override")).toBeInTheDocument();
    expect(
      screen.queryByRole("checkbox", {
        name: /submit stake override instead of validator vote/i,
      })
    ).not.toBeInTheDocument();

    const submitButton = screen.getByRole("button", { name: "Cast Vote" });
    await waitFor(() => expect(submitButton).not.toBeDisabled());

    fireEvent.click(submitButton);

    expect(mockMutate).toHaveBeenCalledWith(
      expect.objectContaining({
        stakeAccount: "stake-account",
        voteAccount: "vote-account",
      }),
      expect.any(Object)
    );
  });

  it("requires explicit confirmation before a validator-identity wallet can submit an override vote", async () => {
    mockUseChainVoteAccount.mockReturnValue({
      data: {
        activeStake: 1_000,
        nodePubkey: wallet.publicKey.toBase58(),
        voteAccount: "validator-vote-account",
      },
      isLoading: false,
    });

    render(<OverrideVoteModal {...defaultProps} />);

    const confirmation = screen.getByRole("checkbox", {
      name: /submit stake override instead of validator vote/i,
    });
    const submitButton = screen.getByRole("button", { name: "Cast Vote" });

    expect(confirmation).not.toBeChecked();
    expect(submitButton).toBeDisabled();

    fireEvent.click(confirmation);

    await waitFor(() => expect(submitButton).not.toBeDisabled());

    fireEvent.click(submitButton);

    expect(mockMutate).toHaveBeenCalledWith(
      expect.objectContaining({
        stakeAccount: "stake-account",
        voteAccount: "vote-account",
      }),
      expect.any(Object)
    );
  });
});
