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
import { cn } from "@/lib/utils";
import { AppButton } from "@/components/ui/AppButton";
import ErrorMessage from "./shared/ErrorMessage";
import RequirementItem from "./shared/RequirementItem";
import { toast } from "sonner";
import { formatAddress } from "@/lib/governance/formatters";
import { useWalletSession } from "@/contexts/WalletSessionContext";
import { useSupportProposal } from "@/hooks";
import { captureException } from "@sentry/nextjs";

interface SupportProposalModalProps {
  proposalId?: string;
  isOpen: boolean;
  onClose: () => void;
}

export function SupportProposalModal({
  proposalId: initialProposalId = "",
  isOpen,
  onClose,
}: SupportProposalModalProps) {
  const [proposalId, setProposalId] = React.useState(initialProposalId);

  const [isLoading, setIsLoading] = React.useState(false);
  const [error, setError] = React.useState<string | undefined>();

  const { anchorWallet: wallet } = useWalletSession();

  const { mutate: supportProposal } = useSupportProposal(
    wallet?.publicKey?.toBase58(),
  );

  React.useEffect(() => {
    if (isOpen) {
      setProposalId(initialProposalId);
      setError(undefined);
    }
  }, [isOpen, initialProposalId]);

  const [requirements] = React.useState({
    hasActiveVoteAccount: true,
    hasIdentityKeypair: true,
    canSupportOnce: true,
  });

  const allRequirementsMet = Object.values(requirements).every(Boolean);

  const handleSuccess = () => {
    toast.success("Proposal supported successfully");
    onClose();
    setIsLoading(false);
  };

  const handleError = (err: Error) => {
    console.log("error mutating support proposal:", err);
    captureException(err);
    toast.error(`Error supporting proposalId ${proposalId}`);
    setError(err instanceof Error ? err.message : "Failed to support proposal");
    setIsLoading(false);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!proposalId || !allRequirementsMet || isLoading) return;

    setIsLoading(true);
    setError(undefined);

    supportProposal(
      {
        proposalId,
        wallet,
      },
      {
        onSuccess: handleSuccess,
        onError: handleError,
      },
    );
  };

  const handleClose = () => {
    setProposalId("");
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
              <DialogTitle className="text-foreground">
                Support Proposal
              </DialogTitle>
              <DialogDescription className="sr-only">
                Support a proposal by entering its public key
              </DialogDescription>
            </DialogHeader>

            <form
              id="support-proposal-form"
              onSubmit={handleSubmit}
              className="space-y-6"
            >
              {/* Proposal ID Input */}
              <div className="space-y-2">
                <label
                  htmlFor="proposal-id"
                  className="text-sm font-medium text-white/80"
                >
                  Proposal ID
                </label>
                <input
                  id="proposal-id"
                  type="text"
                  value={proposalId}
                  onChange={(e) => setProposalId(e.target.value)}
                  placeholder="Enter proposal public key"
                  className={cn(
                    "input",
                    "w-full rounded-md border border-white/10 bg-white/5 px-3 py-1.5 mt-2",
                    "placeholder:text-sm placeholder:text-white/40",
                  )}
                  disabled={isLoading || !!initialProposalId}
                />
                <p className="text-xs text-white/50">
                  The public key of the proposal you want to support
                </p>
              </div>

              {/* Supporting Info */}
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-sm text-white/60">Supporting as:</span>
                  <div className="flex items-center gap-2">
                    <span className="font-mono text-xs sm:text-sm text-foreground">
                      {formatAddress(wallet?.publicKey?.toBase58() || "", 6)}
                    </span>
                    <button
                      type="button"
                      className="text-xs text-cyan-400 transition-colors hover:text-cyan-300"
                    >
                      PREVIEW
                    </button>
                  </div>
                </div>
              </div>

              {/* Requirements */}
              <div className="space-y-3">
                <h3 className="text-xs font-medium uppercase tracking-wide text-white/80 sm:text-sm">
                  Requirements:
                </h3>
                <div className="space-y-2">
                  <RequirementItem
                    met={requirements.hasActiveVoteAccount}
                    text="Must be a validator with an active vote account"
                  />
                  <RequirementItem
                    met={requirements.hasIdentityKeypair}
                    text="Validator identity keypair required"
                  />
                  <RequirementItem
                    met={requirements.canSupportOnce}
                    text="Can only support once per proposal"
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
            form="support-proposal-form"
            size="lg"
            disabled={!proposalId || !allRequirementsMet || isLoading}
            onClick={handleSubmit}
            variant="gradient"
            text={isLoading ? "Supporting..." : "Support Proposal"}
          />
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
