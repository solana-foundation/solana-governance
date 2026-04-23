import { ProposalStatus } from "@/types";
import { SupportButton } from "../SupportButton";
import { AppButton } from "@/components/ui/AppButton";
import { cn } from "@/lib/utils";
import Link from "next/link";
import { SUPPORT_THRESHOLD_PERCENT } from "./support-phase-progress";

interface SupportProposalProps {
  proposalId?: string;
  proposalStatus?: ProposalStatus;
  isLoading?: boolean;
  disabled?: boolean;
  className?: string;
  variant?: "support" | "discussion" | "failed" | "finalized";
}

const VARIANT_CONFIG = {
  support: {
    title: "Support this proposal",
    message:
      `The support phase requires ${SUPPORT_THRESHOLD_PERCENT}% off total validator stake expressing support for the proposal before it can move on to discussion and voting phase`,
    showSupportButton: true,
    showCheckOtherButton: false,
  },
  discussion: {
    title: "Discussion Phase",
    message:
      "The discussion phase covers the 4-5 epoch period while the NCN is created. Voting begins only after this process completes.",
    showSupportButton: false,
    showCheckOtherButton: true,
  },
  failed: {
    title: "Support Failed",
    message:
      "This proposal did not gather sufficient validator stake to advance. The required support threshold was not reached before the deadline expired.",
    showSupportButton: false,
    showCheckOtherButton: true,
  },
  finalized: {
    title: "Voting Ended",
    message:
      "Voting period has ended and all votes have been counted. The proposal is finalized and ready for on-chain execution.",
    showSupportButton: false,
    showCheckOtherButton: true,
  },
};

export default function SupportProposalSection({
  proposalId,
  proposalStatus,
  isLoading,
  disabled,
  className,
  variant = "support",
}: SupportProposalProps) {
  const config = VARIANT_CONFIG[variant];
  const disabledButtons = disabled || isLoading;

  return (
    <div
      className={cn(
        "glass-card flex h-full flex-col items-center justify-center p-6 md:p-6 lg:p-8",
        className
      )}
    >
      <div className="flex w-full max-w-md flex-col items-center text-center">
        <h4 className="h4 font-semibold">{config.title}</h4>
        <p className="mt-4 text-sm text-white/60">{config.message}</p>
        {config.showSupportButton && (
          <div className="mt-8 w-full">
            <SupportButton
              proposalId={proposalId}
              proposalStatus={proposalStatus}
              disabled={disabledButtons}
            />
          </div>
        )}
        {config.showCheckOtherButton && (
          <div className="mt-8 w-full">
            <AppButton
              asChild
              variant="outline"
              size="lg"
              className="w-full rounded-full bg-white/3"
            >
              <Link href="/proposals">Check Other Proposals</Link>
            </AppButton>
          </div>
        )}
      </div>
    </div>
  );
}
