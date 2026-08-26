"use client";

import type {} from "@/components/proposals/proposals-table/ProposalsTable";
import { ColumnDef } from "@tanstack/react-table";
import { ChevronDown } from "lucide-react";
import StatusBadge from "@/components/ui/StatusBadge";
import LifecycleIndicator from "@/components/ui/LifecycleIndicator";
import { SortableHeaderButton } from "@/components/SortableHeaderButton";
import { ProposalRefLabel } from "@/components/proposals/ProposalRefLabel";
import { ProposalRecord } from "@/types";

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

export const columns: ColumnDef<ProposalRecord>[] = [
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
    accessorKey: "startEpoch",
    header: ({ column }) => (
      <SortableHeaderButton column={column} label="Voting Start" />
    ),
    cell: ({ row }) => {
      const value = row.original.startEpoch;
      return (
        <span className="text-sm font-medium text-white/60">
          {value || "-"}
        </span>
      );
    },
  },
  {
    accessorKey: "endEpoch",
    header: ({ column }) => (
      <SortableHeaderButton column={column} label="Voting Through" />
    ),
    cell: ({ row }) => {
      const value =
        row.original.endEpoch > 0 ? row.original.endEpoch - 1n : null;
      return (
        <span className="text-sm font-medium text-white/60">
          {value ?? "-"}
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
