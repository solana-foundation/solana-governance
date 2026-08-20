"use client";

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
import RequirementItem from "./shared/RequirementItem";
import { VoteDistributionControls } from "./shared/VoteDistributionControls";
import {
  useModifyVote,
  useVoteDistribution,
  useWalletRole,
  useHasValidatorVoted,
  VoteDistribution,
} from "@/hooks";
import { toast } from "sonner";
import { useConnector } from "@solana/connector/react";
import type { Address } from "@solana/kit";
import { WalletRole } from "@/types";
import { VotingProposalsDropdown } from "../VotingProposalsDropdown";
import { FormEvent, useEffect, useState } from "react";
import { captureException } from "@sentry/nextjs";

export interface ModifyVoteModalDataProps {
  proposalId?: string;
  consensusResult?: Address;
  initialVoteDist?: VoteDistribution;
}

interface ModifyVoteModalProps extends ModifyVoteModalDataProps {
  isOpen: boolean;
  onClose: () => void;
}

export function ModifyVoteModal({
  proposalId: initialProposalId,
  consensusResult,
  initialVoteDist,
  isOpen,
  onClose,
}: ModifyVoteModalProps) {
  const [selectedProposal, setSelectedProposal] = useState({
    id: initialProposalId,
    consensusResult,
  });

  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | undefined>();
  const {
    distribution,
    totalPercentage,
    isValidDistribution,
    handleOptionChange,
    handleQuickSelect,
    resetDistribution,
  } = useVoteDistribution(initialVoteDist);

  const { account } = useConnector();
  const publicKey = account ?? undefined;

  const { walletRole } = useWalletRole(publicKey);

  const { mutate: modifyVote } = useModifyVote();

  const { data: hasVoted = true, isPending: isLoadingHasVoted } =
    useHasValidatorVoted(selectedProposal.id);

  const [isFinalized] = useState(false);

  useEffect(() => {
    if (isOpen) {
      setSelectedProposal({ id: initialProposalId, consensusResult });
      resetDistribution();
      setError(undefined);
    }
  }, [isOpen, initialProposalId, resetDistribution, consensusResult]);

  const handleProposalChange = (
    proposalId: string,
    consensusResult: Address
  ) => {
    setSelectedProposal({ id: proposalId, consensusResult });
  };

  const handleSuccess = () => {
    toast.success("Vote modified successfully");
    onClose();
    setIsLoading(false);
  };

  const handleError = (err: Error) => {
    console.log("error mutating modify vote:", err);
    captureException(err);
    toast.error(`Error modifying vote for proposal ${initialProposalId}`);
    setError(err instanceof Error ? err.message : "Failed to modify vote");
    setIsLoading(false);
  };

  const handleModifyVote = (voteDistribution: VoteDistribution) => {
    if (!publicKey) {
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
      modifyVote(
        {
          publicKey,
          proposalId: selectedProposal.id,
          // convert basis points to BN, not %
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

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    if (!selectedProposal.id || !isValidDistribution || isLoading) return;

    setIsLoading(true);
    setError(undefined);

    console.log("Modifying vote:", {
      proposalId: selectedProposal.id,
      distribution,
    });
    handleModifyVote(distribution);
  };

  const handleClose = () => {
    setSelectedProposal({ id: undefined, consensusResult: undefined });
    resetDistribution();
    setError(undefined);
    onClose();
  };

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && handleClose()}>
      <DialogContent className="app-modal-content" showCloseButton={false}>
        <div className="app-modal-scroll-region">
          <div className="app-modal-body">
            {/* Mobile handle bar */}
            <div className="app-modal-handle" />

            <DialogHeader>
              <DialogTitle className="text-foreground">Modify Vote</DialogTitle>
              <DialogDescription className="sr-only">
                Modify your vote on a proposal
              </DialogDescription>
            </DialogHeader>

            <form
              id="modify-vote-form"
              onSubmit={handleSubmit}
              className="space-y-6"
            >
              {/* Proposal ID Input */}
              <VotingProposalsDropdown
                value={selectedProposal.id}
                onValueChange={handleProposalChange}
                disabled={!!initialProposalId}
              />

              <VoteDistributionControls
                distribution={distribution}
                totalPercentage={totalPercentage}
                isValidDistribution={isValidDistribution}
                onOptionChange={handleOptionChange}
                onQuickSelect={handleQuickSelect}
                className="space-y-3"
              />

              {/* Requirements */}
              <div className="space-y-3">
                <h3 className="text-xs font-medium uppercase tracking-wide text-white/80 sm:text-sm">
                  Requirements:
                </h3>
                <div className="space-y-2">
                  <RequirementItem
                    met={hasVoted}
                    text="You must have already voted on this proposal"
                    isLoading={isLoadingHasVoted}
                  />
                  <RequirementItem
                    met={!isFinalized}
                    text="The proposal must not be finalized"
                  />
                  <RequirementItem
                    met={isValidDistribution}
                    text="New vote percentages must sum to 100%"
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
            form="modify-vote-form"
            size="lg"
            disabled={
              !selectedProposal.id ||
              !isValidDistribution ||
              !hasVoted ||
              isFinalized ||
              isLoading ||
              isLoadingHasVoted
            }
            onClick={handleSubmit}
            variant="gradient"
            text={isLoading ? "Modifying..." : "Modify Vote"}
          />
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
