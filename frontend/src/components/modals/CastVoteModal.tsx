"use client";

import * as React from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { AppButton } from "@/components/ui/AppButton";
import ErrorMessage from "./shared/ErrorMessage";
import { VoteDistributionControls } from "./shared/VoteDistributionControls";
import {
  useCastVote,
  useHasUserVoted,
  useValidatorVotingPower,
  useVoteDistribution,
  useWalletRole,
  VoteDistribution,
} from "@/hooks";
import { toast } from "sonner";
import { useAnchorWallet } from "@solana/wallet-adapter-react";
import { WalletRole } from "@/types";
import {
  formatAddress,
  formatLamportsDisplay,
} from "@/lib/governance/formatters";
import { VotingProposalsDropdown } from "../VotingProposalsDropdown";
import { PublicKey } from "@solana/web3.js";
import { captureException } from "@sentry/nextjs";
import RequirementItem from "./shared/RequirementItem";

export type CastVoteModalDataProps =
  | {
      proposalId: string;
      consensusResult: PublicKey;
      initialVoteDist?: VoteDistribution;
    }
  | {
      proposalId?: undefined;
      consensusResult?: undefined;
      initialVoteDist?: undefined;
    };

type CastVoteModalProps = {
  isOpen: boolean;
  onClose: () => void;
} & CastVoteModalDataProps;

export function CastVoteModal({
  proposalId: initialProposalId,
  consensusResult,
  initialVoteDist,
  isOpen,
  onClose,
}: CastVoteModalProps) {
  const [selectedProposal, setSelectedProposal] = React.useState({
    id: initialProposalId,
    consensusResult,
  });

  const [isLoading, setIsLoading] = React.useState(false);
  const [error, setError] = React.useState<string | undefined>();
  const {
    distribution,
    totalPercentage,
    isValidDistribution,
    handleOptionChange,
    handleQuickSelect,
    resetDistribution,
  } = useVoteDistribution(initialVoteDist);

  const wallet = useAnchorWallet();

  const { walletRole } = useWalletRole(wallet?.publicKey?.toBase58());

  const { votingPower, isLoading: isLoadingVotingPower } =
    useValidatorVotingPower(wallet?.publicKey?.toBase58());

  const { data: hasVoted = false, isLoading: isLoadingHasVoted } =
    useHasUserVoted(selectedProposal.id);

  const { mutate: castVote } = useCastVote();

  React.useEffect(() => {
    if (isOpen) {
      setSelectedProposal({ id: initialProposalId, consensusResult });
      resetDistribution();
      setError(undefined);
    }
  }, [isOpen, initialProposalId, resetDistribution, consensusResult]);

  const handleProposalChange = (
    proposalId: string,
    consensusResult: PublicKey
  ) => {
    setSelectedProposal({ id: proposalId, consensusResult });
  };

  const handleSuccess = () => {
    toast.success("Vote cast successfully");
    onClose();
    setIsLoading(false);
  };

  const handleError = (err: Error) => {
    console.log("error mutating cast vote:", err);
    captureException(err);
    toast.error(`Error voting for proposal ${initialProposalId}`);
    setError(err instanceof Error ? err.message : "Failed to cast vote");
    setIsLoading(false);
  };

  const handleVote = (voteDistribution: VoteDistribution) => {
    if (!wallet) {
      toast.error("Wallet not connected");
      return;
    }
    if (!selectedProposal.id) {
      toast.error("No proposal ID provided");
      return;
    }
    if (!selectedProposal.consensusResult) {
      toast.error("Couldn't obtain consensus result");
      return;
    }

    if (walletRole === WalletRole.NONE) {
      toast.error("You are not authorized to vote");
      return;
    } else if (
      walletRole === WalletRole.VALIDATOR ||
      walletRole === WalletRole.BOTH
    ) {
      castVote(
        {
          wallet,
          proposalId: selectedProposal.id,
          forVotesBp: voteDistribution.for * 100,
          againstVotesBp: voteDistribution.against * 100,
          abstainVotesBp: voteDistribution.abstain * 100,
          consensusResult: selectedProposal.consensusResult,
        },
        {
          onSuccess: handleSuccess,
          onError: handleError,
        }
      );
      return;
    } else if (walletRole === WalletRole.STAKER) {
      toast.error("Staker can only cast an override vote");
      setError("Staker can only cast an override vote");
      setIsLoading(false);
      return;
    }
    setError("Unknown error, unable to cast vote");
    setIsLoading(false);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedProposal.id || !isValidDistribution || isLoading) return;

    setIsLoading(true);
    setError(undefined);

    console.log("Casting vote:", {
      proposalId: selectedProposal.id,
      distribution,
    });
    handleVote(distribution);
  };

  const handleClose = () => {
    setSelectedProposal({ id: undefined, consensusResult: undefined });
    resetDistribution();
    setError(undefined);
    onClose();
  };

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="app-modal-content" showCloseButton={false}>
        <div className="app-modal-scroll-region">
          <div className="app-modal-body">
            {/* Mobile handle bar */}
            <div className="app-modal-handle" />

            <DialogHeader>
              <DialogTitle className="text-foreground">Cast Vote</DialogTitle>
              <DialogDescription className="sr-only">
                Cast your vote on a proposal
              </DialogDescription>
            </DialogHeader>

            <form
              id="cast-vote-form"
              onSubmit={handleSubmit}
              className="space-y-6"
            >
              {/* Proposal ID Input */}
              <VotingProposalsDropdown
                value={selectedProposal.id}
                onValueChange={handleProposalChange}
                disabled={!!initialProposalId}
              />

              {/* Voting Info */}
              <div className="space-y-4 rounded-lg bg-white/5 p-4">
                <div className="flex items-center justify-between">
                  <span className="font-mono text-xs text-foreground sm:text-sm">
                    {formatAddress(wallet?.publicKey?.toBase58() || "", 6)}
                  </span>
                  <div className="text-right">
                    <p className="text-xs text-white/60">Voting Power</p>
                    {isLoadingVotingPower && (
                      <div className="flex justify-center">
                        <div className="h-5 w-14 mt-1 animate-pulse rounded bg-white/10" />
                      </div>
                    )}
                    {!isLoadingVotingPower && votingPower && (
                      <p className="text-sm font-semibold text-foreground sm:text-base">
                        {formatLamportsDisplay(votingPower).value}
                      </p>
                    )}
                  </div>
                </div>
                <div className="border-t border-white/10 pt-4">
                  <p className="text-xs text-white/60">Vote path</p>
                  <p className="text-sm font-semibold text-foreground">
                    Validator vote
                  </p>
                </div>
              </div>
              <VoteDistributionControls
                distribution={distribution}
                totalPercentage={totalPercentage}
                isValidDistribution={isValidDistribution}
                onOptionChange={handleOptionChange}
                onQuickSelect={handleQuickSelect}
                distributionLabel="Custom Vote Distribution"
                invalidTotalMessage="Total must equal 100%"
                className="space-y-3"
              />

              {/* Requirements */}
              <div className="space-y-3">
                <h3 className="text-xs font-medium uppercase tracking-wide text-white/80 sm:text-sm">
                  Requirements:
                </h3>
                <div className="space-y-2">
                  <RequirementItem
                    met={!hasVoted}
                    text="You haven't voted on this proposal yet"
                    isLoading={isLoadingHasVoted}
                  />
                </div>
              </div>

              {/* Error Message */}
              {error && <ErrorMessage error={error} />}
            </form>
          </div>
        </div>

        <DialogFooter className="app-modal-footer">
          <AppButton
            variant="outline"
            text="Cancel"
            size="lg"
            onClick={handleClose}
            disabled={isLoading}
          />
          <AppButton
            form="cast-vote-form"
            size="lg"
            disabled={
              !selectedProposal.id ||
              !isValidDistribution ||
              hasVoted ||
              isLoading ||
              isLoadingHasVoted
            }
            onClick={handleSubmit}
            variant="gradient"
            text={isLoading ? "Casting..." : "Cast Vote"}
          />
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
