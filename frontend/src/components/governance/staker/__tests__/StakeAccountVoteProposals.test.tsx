import React from "react";
import { render, screen } from "@testing-library/react";
import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";
import { StakeAccountVoteProposals } from "../StakeAccountVoteProposals";
import type { StakeAccountData } from "@/types/stakeAccounts";

const useStakerVotedProposals = jest.fn();
jest.mock("@/hooks", () => ({
  useStakerVotedProposals: (...args: unknown[]) =>
    useStakerVotedProposals(...args),
  useCopyToClipboard: () => ({ copied: false, copyToClipboard: jest.fn() }),
}));
jest.mock("@/components/proposals/ProposalHeading", () => ({
  ProposalHeading: ({ title }: { title: string }) => <span>{title}</span>,
}));
jest.mock("next/link", () => ({
  __esModule: true,
  default: ({ children }: { children: React.ReactNode }) => <a>{children}</a>,
}));

// 2^53 + 1 lamports: BN.toNumber() throws above 2^53.
const UNSAFE_STAKE = new BN(2).pow(new BN(53)).addn(1);

describe("StakeAccountVoteProposals", () => {
  it("renders a validator vote whose stake is above 2^53 lamports", () => {
    useStakerVotedProposals.mockReturnValue({
      isLoading: false,
      error: undefined,
      data: [
        {
          votePublicKey: PublicKey.unique().toBase58(),
          proposal: {
            publicKey: PublicKey.unique(),
            description: "https://example.invalid/proposal",
            proposalRef: undefined,
            title: "Large stake proposal",
            status: "voting",
          },
          voteAccount: {
            forVotesBp: new BN(10_000),
            againstVotesBp: new BN(0),
            abstainVotesBp: new BN(0),
            stakeAmount: UNSAFE_STAKE,
          },
        },
      ],
    });

    expect(() =>
      render(
        <StakeAccountVoteProposals
          stakeAccount={
            {
              voteAccount: PublicKey.unique().toBase58(),
            } as unknown as StakeAccountData
          }
        />,
      ),
    ).not.toThrow();
    expect(screen.getByText("Large stake proposal")).toBeInTheDocument();
    expect(screen.getByText(/Total:/)).toBeInTheDocument();
  });
});
