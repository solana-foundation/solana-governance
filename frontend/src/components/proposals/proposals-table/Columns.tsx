"use client";

import type {} from "@/components/proposals/proposals-table/ProposalsTable";
import { ColumnDef } from "@tanstack/react-table";
import { ChevronDown } from "lucide-react";
import StatusBadge from "@/components/ui/StatusBadge";
import LifecycleIndicator from "@/components/ui/LifecycleIndicator";
import { SortableHeaderButton } from "@/components/SortableHeaderButton";
import { ProposalRefLabel } from "@/components/proposals/ProposalRefLabel";
import { ProposalRecord } from "@/types";
import type { NetworkMetaResponse } from "@/chain";
import {
  getProposalVoteStats,
  ProposalVoteShare,
} from "@/components/proposals/ProposalVoteStats";

interface ProposalColumnsOptions {
  snapshotMetasBySlot: Map<number, NetworkMetaResponse>;
  isLoadingSnapshotMeta: boolean;
}

function getVoteStats(
  proposal: ProposalRecord,
  snapshotMetasBySlot: Map<number, NetworkMetaResponse>,
) {
  return getProposalVoteStats(
    proposal,
    snapshotMetasBySlot.get(proposal.snapshotSlot),
  );
}

function VoteStatsLoadingCell({ width }: { width: string }) {
  return (
    <div className={`mx-auto h-4 animate-pulse rounded bg-white/10 ${width}`} />
  );
}

function ParticipationCell({
  proposal,
  snapshotMetasBySlot,
  isLoadingSnapshotMeta,
}: ProposalColumnsOptions & { proposal: ProposalRecord }) {
  if (isLoadingSnapshotMeta) {
    return <VoteStatsLoadingCell width="w-16" />;
  }

  const stats = getVoteStats(proposal, snapshotMetasBySlot);
  const { quorum } = stats;

  if (!quorum.known) {
    return (
      <div className="flex flex-col items-center gap-1 text-white/40">
        <span className="text-sm font-semibold tabular-nums">—</span>
        <span className="text-[10px]">Snapshot unavailable</span>
      </div>
    );
  }

  return (
    <div className="flex flex-col items-center gap-1 tabular-nums">
      <span
        className={`text-sm font-semibold ${
          quorum.isMet ? "text-primary" : "text-white/80"
        }`}
      >
        {quorum.participationPercent.toFixed(2)}%
      </span>
      <span className="text-[10px] text-white/40">
        {quorum.isMet ? "Quorum met" : "of total stake"}
      </span>
    </div>
  );
}

// function VotingEndsInCell({ votingEndsIn }: { votingEndsIn: string }) {
//   const mounted = useMounted();

//   if (!mounted) {
//     return <span className="text-sm font-medium text-white/60">-</span>;
//   }

//   const value = calculateVotingEndsIn(votingEndsIn);
//   return (
//     <span className="text-sm font-medium text-white/60">{value || "-"}</span>
//   );
// }

export function getProposalColumns({
  snapshotMetasBySlot,
  isLoadingSnapshotMeta,
}: ProposalColumnsOptions): ColumnDef<ProposalRecord>[] {
  return [
    {
      id: "proposalRef",
      // Sorting and filtering use the synchronously-parsed value; the cell layers the
      // network-resolved one on top for pull-request links.
      accessorFn: (row) => row.proposalRef?.label ?? "",
      header: "Proposal",
      cell: ({ row }) => (
        <ProposalRefLabel
          url={row.original.description}
          fallback={row.original.proposalRef}
          className="text-sm font-medium text-white/90"
        />
      ),
    },
    {
      id: "lifecycleStage",
      accessorKey: "status",
      header: ({ column }) => (
        <SortableHeaderButton column={column} label="Lifecycle Stage" />
      ),
      cell: ({ row }) => <LifecycleIndicator status={row.original.status} />,
    },
    {
      id: "votingPeriod",
      accessorFn: (row) => row.startEpoch,
      header: ({ column }) => (
        <SortableHeaderButton column={column} label="Voting Period" />
      ),
      cell: ({ row }) => {
        const { startEpoch, endEpoch } = row.original;
        const value =
          startEpoch > 0 && endEpoch > 0
            ? `${startEpoch} - ${endEpoch - 1}`
            : "-";

        return (
          <span className="text-sm font-medium text-white/60 tabular-nums">
            {value}
          </span>
        );
      },
    },
    {
      accessorKey: "status",
      header: "Status",
      cell: ({ row }) => <StatusBadge status={row.original.status} />,
    },
    {
      id: "participation",
      accessorFn: (row) => {
        const stats = getVoteStats(row, snapshotMetasBySlot);
        return stats.quorum.known ? stats.quorum.participationPercent : null;
      },
      header: ({ column }) => (
        <SortableHeaderButton column={column} label="Participation" />
      ),
      cell: ({ row }) => (
        <ParticipationCell
          proposal={row.original}
          snapshotMetasBySlot={snapshotMetasBySlot}
          isLoadingSnapshotMeta={isLoadingSnapshotMeta}
        />
      ),
    },
    {
      id: "voteBreakdown",
      header: "Vote Breakdown",
      cell: ({ row }) => (
        <ProposalVoteShare
          proposal={row.original}
          snapshotMeta={snapshotMetasBySlot.get(row.original.snapshotSlot)}
          className="mx-auto min-w-48 max-w-56"
        />
      ),
    },
    {
      id: "toggle",
      header: () => null,
      cell: ({ row }) => (
        <div className="flex justify-end">
          <ChevronDown
            className={`size-4 text-white/60 transition-transform ${
              row.getIsExpanded() ? "rotate-180" : "rotate-0"
            }`}
            aria-hidden
          />
        </div>
      ),
      size: 56,
    },
  ];
}
