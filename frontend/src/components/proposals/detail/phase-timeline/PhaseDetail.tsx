import { ProposalStatus } from "@/types";
import { PHASE_DETAILS, FAILED_PHASE_DETAIL } from "./constants";
import type { PhaseKey } from "./types";

interface PhaseDetailProps {
  currentPhase: PhaseKey;
  status: ProposalStatus;
  isLoading?: boolean;
  remainingTime?: string;
  currentEpoch?: number;
  activePhaseEndEpoch?: number;
}

export function PhaseDetail({
  currentPhase,
  status,
  isLoading,
  remainingTime = "-",
  currentEpoch,
  activePhaseEndEpoch,
}: PhaseDetailProps) {
  const isFailed = status === "failed";
  const detail = isFailed ? FAILED_PHASE_DETAIL : PHASE_DETAILS[currentPhase];

  if (!detail) return null;

  return (
    <div className="mx-auto flex max-w-lg flex-col items-center justify-center space-y-2 pt-2 lg:pt-4 lg:pb-2 text-center">
      {isLoading ? (
        <div className="h-4 w-20 bg-white/10 animate-pulse rounded" />
      ) : (
        <p
          className={
            isFailed
              ? "text-sm font-semibold text-destructive"
              : "text-sm font-semibold text-primary/90"
          }
        >
          {detail.title}
        </p>
      )}

      {isLoading ? (
        <div className="flex flex-col gap-2 items-center justify-center mt-1">
          <div className="h-4 w-60 bg-white/10 animate-pulse rounded" />
          <div className="h-4 w-50 bg-white/10 animate-pulse rounded" />
        </div>
      ) : (
        <p className="text-sm text-white/50">{detail.body}</p>
      )}

      {!isLoading && !isFailed && (
        <div className="mt-2 flex items-center gap-14 text-xs text-white/60">
          <p>Next phase in: <span className="text-white font-bold">{remainingTime}</span></p>
          <p>
            Epoch: <span className="text-white font-bold">{currentEpoch ?? "-"} / {activePhaseEndEpoch ?? "-"}</span>
          </p>
        </div>
      )}
    </div>
  );
}
