"use client";

import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { ProposalStatus } from "@/types";
import { Circle, Loader, X } from "lucide-react";
import { FAILED_PHASE_DETAIL } from "../proposals/detail/phase-timeline/constants";
import { SUPPORT_THRESHOLD_PERCENT } from "../proposals/detail/support-phase-progress";

const STAGE_ORDER: ProposalStatus[] = [
  "supporting",
  "discussion",
  "voting",
  "finalized",
];

const STAGE_LABEL: Record<ProposalStatus, string> = {
  supporting: "Supporting",
  discussion: "Discussion",
  voting: "Voting",
  finalized: "Finished",
  failed: "Failed",
};

const STAGE_DESCRIPTION: Record<ProposalStatus, string> = {
  supporting: `The support phase requires ${SUPPORT_THRESHOLD_PERCENT}% off total validator stake expressing support for the proposal before it can move on to discussion and voting phase.`,
  discussion:
    "The discussion phase covers the 4-5 epoch period while the NCN is created. Voting begins only after this process completes.",
  voting:
    "Validators vote on active governance proposals. Delegators can override their validator's vote using stake account verification.",
  finalized:
    "Voting period has ended and all votes have been counted. The proposal is finalized and ready for on-chain execution.",
  failed: FAILED_PHASE_DETAIL.body,
};

const FAILED_LABEL = "Support Failed";
const FAILED_DESCRIPTION =
  "This proposal did not receive enough support to proceed to the discussion phase. The support threshold was not met within the required timeframe.";

type LifecycleIndicatorProps = {
  status: ProposalStatus;
};

export default function LifecycleIndicator({
  status,
}: LifecycleIndicatorProps) {
  const isFailed = status === "failed";
  const isComplete = status === "finalized" || isFailed;

  const activeIndex = isFailed ? 0 : Math.max(STAGE_ORDER.indexOf(status), 0);

  const label = isFailed ? FAILED_LABEL : STAGE_LABEL[status];
  const description = isFailed ? FAILED_DESCRIPTION : STAGE_DESCRIPTION[status];

  const indicators = (
    <div className="flex items-center justify-center gap-2">
      {STAGE_ORDER.map((value, index) => (
        <span
          key={value}
          className={`h-2 w-2 rounded-full transition ${
            index <= activeIndex ? "bg-foreground/80" : "bg-white/10"
          }`}
        />
      ))}
    </div>
  );

  // Only show tooltip on desktop (lg and above)
  return (
    <>
      {/* Desktop with tooltip */}
      <div className="hidden lg:block">
        <TooltipProvider delayDuration={150}>
          <Tooltip>
            <TooltipTrigger asChild>{indicators}</TooltipTrigger>
            <TooltipContent
              side="top"
              className="w-[240px] rounded-xl border border-white/10 bg-white/10 px-4 py-3 shadow-xl backdrop-blur-md"
              sideOffset={8}
            >
              <div className="flex flex-col gap-2">
                <div className="flex items-center gap-2">
                  {isFailed ? (
                    <X className="size-4 text-white" />
                  ) : isComplete ? (
                    <Circle className="size-3 text-white" />
                  ) : (
                    <Loader className="size-4 animate-spin text-white" />
                  )}
                  <p className="mb-1 text-sm font-semibold text-white">
                    {label}
                  </p>
                </div>
                <p className="text-xs leading-[1.5] text-white whitespace-pre-wrap ">
                  {description}
                </p>
              </div>
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      </div>

      {/* Mobile/Tablet without tooltip */}
      <div className="block lg:hidden">{indicators}</div>
    </>
  );
}
