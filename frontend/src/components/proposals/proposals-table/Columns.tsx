"use client";

import { ColumnDef } from "@tanstack/react-table";
import { ChevronDown } from "lucide-react";
import StatusBadge from "@/components/ui/StatusBadge";
import LifecycleIndicator from "@/components/ui/LifecycleIndicator";
import { SortableHeaderButton } from "@/components/SortableHeaderButton";
import { ProposalRecord } from "@/types";
import { useGovernanceConfigContext } from "@/contexts/GovernanceConfigContext";
import {
  epochConstantsFromGovernanceConfig,
  getProposalPhaseEpochs,
} from "@/lib/proposals";
import {
  useEpochToDate,
  useMounted,
  useValidatorsTotalStakedLamports,
} from "@/hooks";
import { calculateVotingEndsIn } from "@/helpers";

function getPhaseEndEpoch(
  proposal: ProposalRecord,
  supportEndEpoch: number | undefined,
  discussionEndEpoch: number | undefined,
): number | undefined {
  switch (proposal.status) {
    case "supporting":
      return supportEndEpoch;
    case "discussion":
      return discussionEndEpoch;
    case "voting":
      return proposal.endEpoch || undefined;
    default:
      return undefined;
  }
}

function RemainingTimeCell({ proposal }: { proposal: ProposalRecord }) {
  const mounted = useMounted();
  const governanceConfigQuery = useGovernanceConfigContext();
  const epochConstants = governanceConfigQuery.data
    ? epochConstantsFromGovernanceConfig(governanceConfigQuery.data)
    : undefined;
  const phaseEpochs =
    epochConstants !== undefined
      ? getProposalPhaseEpochs(proposal.creationEpoch, epochConstants)
      : undefined;

  const endEpoch = getPhaseEndEpoch(
    proposal,
    phaseEpochs?.supportEndEpoch,
    phaseEpochs?.discussionEndEpoch,
  );

  const { data: endsAt, isLoading } = useEpochToDate(endEpoch);

  if (
    proposal.status === "finalized" ||
    proposal.status === "failed" ||
    endEpoch === undefined
  ) {
    return <span className="text-sm font-medium text-white/60">-</span>;
  }

  if (!mounted || isLoading || !endsAt) {
    return <span className="text-sm font-medium text-white/60">-</span>;
  }

  const value = calculateVotingEndsIn(endsAt.toISOString());
  return (
    <span className="text-sm font-medium text-white/60">{value || "-"}</span>
  );
}

function SupportPercentCell({ proposal }: { proposal: ProposalRecord }) {
  const { totalStakedLamports, isLoading } =
    useValidatorsTotalStakedLamports();

  if (isLoading || totalStakedLamports <= 0) {
    return <span className="text-sm font-medium text-white/60">-</span>;
  }

  const supportPercent =
    (proposal.clusterSupportLamports / totalStakedLamports) * 100;

  return (
    <span className="text-sm font-medium text-white/60">
      {supportPercent.toFixed(1)}%
    </span>
  );
}

const proposalColumn: ColumnDef<ProposalRecord> = {
  id: "proposal",
  accessorKey: "title",
  header: "Proposal",
  cell: ({ row }) => (
    <div className="min-w-0 max-w-[18rem] text-left">
      <div className="truncate text-sm font-medium text-white/90">
        {row.original.title || "-"}
      </div>
      {row.original.simd ? (
        <div className="truncate text-xs text-white/45">
          SIMD {row.original.simd}
        </div>
      ) : null}
    </div>
  ),
};

const lifecycleColumn: ColumnDef<ProposalRecord> = {
  id: "lifecycleStage",
  accessorKey: "status",
  header: ({ column }) => (
    <SortableHeaderButton column={column} label="Lifecycle Stage" />
  ),
  cell: ({ row }) => <LifecycleIndicator status={row.original.status} />,
};

const remainingTimeColumn: ColumnDef<ProposalRecord> = {
  id: "remainingTime",
  header: "Remaining Time",
  cell: ({ row }) => <RemainingTimeCell proposal={row.original} />,
};

const statusColumn: ColumnDef<ProposalRecord> = {
  accessorKey: "status",
  header: "Status",
  cell: ({ row }) => <StatusBadge status={row.original.status} />,
};

const toggleColumn: ColumnDef<ProposalRecord> = {
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
};

export const supportColumns: ColumnDef<ProposalRecord>[] = [
  proposalColumn,
  lifecycleColumn,
  {
    id: "supportPercent",
    accessorFn: (row) => row.clusterSupportLamports,
    header: "Support (%)",
    cell: ({ row }) => <SupportPercentCell proposal={row.original} />,
  },
  remainingTimeColumn,
  statusColumn,
  toggleColumn,
];

export const votingColumns: ColumnDef<ProposalRecord>[] = [
  proposalColumn,
  lifecycleColumn,
  {
    id: "quorumPercent",
    accessorKey: "quorumPercent",
    header: "Quorum (%)",
    cell: ({ row }) => (
      <span className="text-sm font-medium text-white/60">
        {row.original.quorumPercent}
      </span>
    ),
  },
  remainingTimeColumn,
  {
    id: "startEpoch",
    accessorKey: "startEpoch",
    header: ({ column }) => (
      <SortableHeaderButton column={column} label="Voting Start" />
    ),
    cell: ({ row }) => (
      <span className="text-sm font-medium text-white/60">
        {row.original.startEpoch || "-"}
      </span>
    ),
  },
  {
    id: "endEpoch",
    accessorKey: "endEpoch",
    header: ({ column }) => (
      <SortableHeaderButton column={column} label="Voting End" />
    ),
    cell: ({ row }) => (
      <span className="text-sm font-medium text-white/60">
        {row.original.endEpoch || "-"}
      </span>
    ),
  },
  statusColumn,
  toggleColumn,
];

/** @deprecated Prefer supportColumns / votingColumns */
export const columns = supportColumns;
