import { ProposalStatus } from "@/types";

export type PhaseStatus = "in-progress" | "complete" | "failed";

export const PHASE_STATUS_STYLES: Record<PhaseStatus, string> = {
  failed: "bg-red-500/20 text-red-400",
  "in-progress": "bg-white/10 text-white",
  complete: "bg-emerald-500/20 text-emerald-400",
};

export const PHASE_STATUS_LABELS: Record<PhaseStatus, string> = {
  failed: "Failed",
  "in-progress": "In Progress",
  complete: "Complete",
};

export function getPhaseStatus(status: ProposalStatus): PhaseStatus {
  if (status === "failed") return "failed";
  if (
    status === "supporting" ||
    status === "discussion" ||
    status === "voting"
  ) {
    return "in-progress";
  }
  return "complete";
}
