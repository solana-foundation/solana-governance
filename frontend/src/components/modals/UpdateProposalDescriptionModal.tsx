"use client";

import * as React from "react";
import { useAnchorWallet } from "@solana/wallet-adapter-react";
import { toast } from "sonner";
import { useQueryClient } from "@tanstack/react-query";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { AppButton } from "@/components/ui/AppButton";
import ErrorMessage from "./shared/ErrorMessage";
import { useUpdateProposalDescription } from "@/hooks";
import {
  ProposalDocumentValidationError,
  validateProposalUrl,
} from "@/lib/github";
import { GET_ALL_PROPOSALS, GET_PROPOSAL_DOCUMENT } from "@/helpers";

interface Props {
  isOpen: boolean;
  onClose: () => void;
  proposalId?: string;
  currentDescription?: string;
}

/** Lets a proposal author replace its document before voting starts. */
export function UpdateProposalDescriptionModal({
  isOpen,
  onClose,
  proposalId,
  currentDescription,
}: Props) {
  const wallet = useAnchorWallet();
  const queryClient = useQueryClient();
  const { mutate: update, isPending } = useUpdateProposalDescription();
  const [description, setDescription] = React.useState(currentDescription ?? "");
  const [error, setError] = React.useState<string>();
  const [allowUnverified, setAllowUnverified] = React.useState(false);
  const [canSkipVerification, setCanSkipVerification] = React.useState(false);
  const validation = React.useMemo(
    () => validateProposalUrl(description),
    [description],
  );

  React.useEffect(() => {
    if (isOpen) {
      setDescription(currentDescription ?? "");
      setError(undefined);
      setAllowUnverified(false);
      setCanSkipVerification(false);
    }
  }, [currentDescription, isOpen]);

  if (!proposalId || !currentDescription) return null;

  const submit = (event: React.FormEvent) => {
    event.preventDefault();
    if (!validation.ok) return;
    setError(undefined);
    setCanSkipVerification(false);
    update(
      {
        proposalId,
        description: validation.normalized,
        wallet,
        skipDocumentCheck: allowUnverified,
      },
      {
        onSuccess: () => {
          queryClient.invalidateQueries({ queryKey: [GET_ALL_PROPOSALS] });
          queryClient.removeQueries({
            queryKey: [GET_PROPOSAL_DOCUMENT, currentDescription],
          });
          toast.success("Proposal document updated");
          onClose();
        },
        onError: (reason) => {
          if (reason instanceof ProposalDocumentValidationError) {
            setError(`${reason.message} You may explicitly continue without document verification.`);
            setCanSkipVerification(true);
            return;
          }
          setError(reason instanceof Error ? reason.message : "Failed to update proposal document");
        },
      },
    );
  };

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="app-modal-content">
        <DialogHeader>
          <DialogTitle>Update Proposal Document</DialogTitle>
          <DialogDescription>
            Replace the pinned GitHub markdown URL before voting begins.
          </DialogDescription>
        </DialogHeader>
        <form id="update-proposal-description" className="space-y-4" onSubmit={submit}>
          <label htmlFor="updated-proposal-description" className="space-y-2 text-sm font-medium text-white/80">
            Commit-pinned markdown URL
            <input
              id="updated-proposal-description"
              type="url"
              value={description}
              onChange={(event) => {
                setDescription(event.target.value);
                setAllowUnverified(false);
                setCanSkipVerification(false);
              }}
              className="input mt-1 w-full rounded-md border border-white/10 bg-white/5 px-3 py-1.5"
            />
          </label>
          {!validation.ok && validation.errors.map((issue) => (
            <p key={issue.code} className="text-xs text-red-400">{issue.message}</p>
          ))}
          {error && <ErrorMessage error={error} />}
          {canSkipVerification && (
            <label className="flex items-center gap-2 text-sm text-white/70">
              <input
                type="checkbox"
                checked={allowUnverified}
                onChange={(event) => setAllowUnverified(event.target.checked)}
              />
              Submit without document verification
            </label>
          )}
        </form>
        <DialogFooter>
          <AppButton variant="outline" text="Cancel" size="lg" onClick={onClose} disabled={isPending} />
          <AppButton
            form="update-proposal-description"
            variant="gradient"
            text={isPending ? "Updating..." : "Update Document"}
            size="lg"
            disabled={!validation.ok || isPending}
          />
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
