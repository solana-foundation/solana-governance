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
import { computeProposalVoteStats, resolveQuorumDenominator } from "@/chain";

interface ProposalColumnsOptions {
  snapshotMetasBySlot: Map<number, NetworkMetaResponse>;
  isLoadingSnapshotMeta: boolean;
}

function getVoteStats(
  proposal: ProposalRecord,
  snapshotMetasBySlot: Map<number, NetworkMetaResponse>,
) {
  return computeProposalVoteStats({
    forLamports: proposal.forVotesLamports,
    againstLamports: proposal.againstVotesLamports,
    abstainLamports: proposal.abstainVotesLamports,
    totalActiveStake: resolveQuorumDenominator(
      snapshotMetasBySlot.get(proposal.snapshotSlot),
      proposal.snapshotSlot,
    ),
  });
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

const VOTE_SEGMENTS = [
  {
    key: "forPercent",
    label: "For",
    dotClassName: "bg-primary",
    barClassName: "bg-primary",
  },
  {
    key: "againstPercent",
    label: "Against",
    dotClassName: "bg-destructive",
    barClassName: "bg-destructive",
  },
  {
    key: "abstainPercent",
    label: "Abstain",
    dotClassName: "bg-white/40",
    barClassName: "bg-white/40",
  },
] as const;

function VoteBreakdownCell({
  proposal,
  snapshotMetasBySlot,
}: Pick<ProposalColumnsOptions, "snapshotMetasBySlot"> & {
  proposal: ProposalRecord;
}) {
  const stats = getVoteStats(proposal, snapshotMetasBySlot);
  const { voteShare } = stats;

  if (!voteShare.known) {
    return <span className="text-sm font-semibold text-white/40">—</span>;
  }

  return (
    <div className="mx-auto min-w-48 max-w-56 space-y-2 tabular-nums">
      <div
        className="flex h-1.5 overflow-hidden rounded-full bg-white/8"
        aria-label={`Share of votes cast: For ${voteShare.forPercent.toFixed(2)}%, Against ${voteShare.againstPercent.toFixed(2)}%, Abstain ${voteShare.abstainPercent.toFixed(2)}%`}
        role="img"
      >
        {VOTE_SEGMENTS.map((segment) => (
          <span
            key={segment.key}
            className={segment.barClassName}
            style={{ width: `${voteShare[segment.key]}%` }}
          />
        ))}
      </div>
      <div className="grid grid-cols-3 gap-2 text-[10px] text-white/55">
        {VOTE_SEGMENTS.map((segment) => (
          <span
            key={segment.key}
            className="flex flex-col items-center gap-0.5 whitespace-nowrap"
          >
            <span className="inline-flex items-center gap-1">
              <span
                className={`size-1.5 rounded-full ${segment.dotClassName}`}
                aria-hidden
              />
              {segment.label}
            </span>
            <span className="font-medium text-white/75">
              {voteShare[segment.key].toFixed(2)}%
            </span>
          </span>
        ))}
      </div>
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
        <VoteBreakdownCell
          proposal={row.original}
          snapshotMetasBySlot={snapshotMetasBySlot}
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
