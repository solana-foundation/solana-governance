"use client";

import * as React from "react";
import {
  flexRender,
  getCoreRowModel,
  getPaginationRowModel,
  getSortedRowModel,
  useReactTable,
  type SortingState,
} from "@tanstack/react-table";
import { Download, Search, X } from "lucide-react";

import { useProposalVotes } from "@/hooks/useProposalVotes";
import { createTopVoterColumns } from "./TopVotersColumns";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { AppButton } from "@/components/ui/AppButton";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

import {
  TablePaginationDesktop,
  TablePaginationMobile,
} from "@/components/governance/shared/TablePagination";
import { cn } from "@/lib/utils";
import { createTopVotersCsv } from "@/lib/governance/topVotersCsv";
import type { ProposalRecord, TopVoterRecord } from "@/types";

const DEFAULT_SORTING: SortingState = [{ id: "stakedLamports", desc: true }];

type VoteFilter = "all" | "for" | "against" | "abstain";

const VOTE_FILTER_OPTIONS: { value: VoteFilter; label: string }[] = [
  { value: "all", label: "All votes" },
  { value: "for", label: "For" },
  { value: "against", label: "Against" },
  { value: "abstain", label: "Abstain" },
];

// A split vote matches every side it put a share on.
function matchesVoteFilter(
  { forVotesBp, againstVotesBp, abstainVotesBp }: TopVoterRecord["voteData"],
  filter: VoteFilter,
) {
  if (filter === "for") return !forVotesBp.isZero();
  if (filter === "against") return !againstVotesBp.isZero();
  if (filter === "abstain") return !abstainVotesBp.isZero();
  return true;
}

type VoterTypeFilter = "all" | TopVoterRecord["walletType"];

const VOTER_TYPE_FILTER_OPTIONS: { value: VoterTypeFilter; label: string }[] = [
  { value: "all", label: "All voters" },
  { value: "validator", label: "Validators" },
  { value: "staker", label: "Staker overrides" },
];

interface FocusedValidator {
  validatorVoteAddress: string;
  validatorName: string;
}

interface TopVotersTableProps {
  proposal: ProposalRecord | undefined;
}

export default function TopVotersTable({ proposal }: TopVotersTableProps) {
  const [searchValue, setSearchValue] = React.useState("");
  const [voteFilter, setVoteFilter] = React.useState<VoteFilter>("all");
  const [voterTypeFilter, setVoterTypeFilter] =
    React.useState<VoterTypeFilter>("all");
  const [focusedValidator, setFocusedValidator] =
    React.useState<FocusedValidator | null>(null);
  const [sorting, setSorting] = React.useState<SortingState>(() => [
    ...DEFAULT_SORTING,
  ]);

  const {
    data: topVoters = [],
    isLoading: isLoadingVotes,
    isFetching: isFetchingVotes,
  } = useProposalVotes(proposal?.publicKey);

  const overrideCounts = React.useMemo(() => {
    const counts = new Map<string, number>();
    for (const voter of topVoters) {
      if (voter.walletType !== "staker") continue;
      const address = voter.validatorVoteAddress;
      counts.set(address, (counts.get(address) ?? 0) + 1);
    }
    return counts;
  }, [topVoters]);

  const hasActiveFilters =
    searchValue.trim().length > 0 ||
    voteFilter !== "all" ||
    voterTypeFilter !== "all" ||
    focusedValidator !== null;

  const filteredData = React.useMemo(() => {
    const searchTerm = searchValue.trim().toLowerCase();
    if (!hasActiveFilters) {
      return topVoters;
    }

    return topVoters.filter((voter) => {
      if (!matchesVoteFilter(voter.voteData, voteFilter)) {
        return false;
      }
      if (voterTypeFilter !== "all" && voter.walletType !== voterTypeFilter) {
        return false;
      }
      if (
        focusedValidator &&
        voter.validatorVoteAddress !== focusedValidator.validatorVoteAddress
      ) {
        return false;
      }

      const name = voter.validatorName.toLowerCase();
      const identity = voter.validatorIdentity.toLowerCase();
      const stakeAccount = voter.stakeAccount?.toLowerCase() ?? "";

      return (
        name.includes(searchTerm) ||
        identity.includes(searchTerm) ||
        stakeAccount.includes(searchTerm)
      );
    });
  }, [
    searchValue,
    voteFilter,
    voterTypeFilter,
    focusedValidator,
    hasActiveFilters,
    topVoters,
  ]);

  const columns = React.useMemo(
    () =>
      createTopVoterColumns({
        overrideCounts,
        onShowOverrides: ({ validatorVoteAddress, validatorName }) =>
          setFocusedValidator({ validatorVoteAddress, validatorName }),
      }),
    [overrideCounts],
  );

  const table = useReactTable({
    data: filteredData,
    columns,
    state: {
      sorting,
    },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
    getRowId: (row) => row.id + (row.stakeAccount || row.walletType),
    initialState: {
      pagination: {
        pageSize: 10,
      },
      sorting: DEFAULT_SORTING,
    },
  });

  const handleReset = () => {
    const nextSorting: SortingState = [...DEFAULT_SORTING];
    setSearchValue("");
    setVoteFilter("all");
    setVoterTypeFilter("all");
    setFocusedValidator(null);
    table.setSorting(nextSorting);
    table.setPageIndex(0);
  };

  const handleDownload = () => {
    const voters = table
      .getPrePaginationRowModel()
      .rows.map((row) => row.original);
    if (voters.length === 0) return;

    const blob = new Blob([createTopVotersCsv(voters)], {
      type: "text/csv;charset=utf-8",
    });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = "top-voters.csv";
    document.body.appendChild(link);
    link.click();
    link.remove();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="glass-card overflow-hidden rounded-3xl border border-white/10">
      <div className="flex flex-col gap-4 border-b border-white/10  px-6 py-5 md:flex-row md:items-center md:justify-between">
        <h4 className="h4 font-semibold">Top Voters</h4>
        <div className="flex flex-col gap-3 sm:flex-row sm:flex-wrap sm:items-center sm:justify-end">
          <div className="relative w-full sm:flex-1 sm:max-w-xs md:max-w-[200px] lg:max-w-md">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-white/50" />
            <input
              placeholder="Search voters..."
              value={searchValue}
              onChange={(e) => {
                setSearchValue(e.target.value);
                table.setPageIndex(0);
              }}
              className="w-full pl-10 pr-4 py-2 input"
            />
          </div>
          <Select
            value={voteFilter}
            onValueChange={(value) => {
              setVoteFilter(value as VoteFilter);
              table.setPageIndex(0);
            }}
          >
            <SelectTrigger
              className="w-full sm:w-[140px] text-white/60"
              aria-label="Filter by vote"
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent className="select-background">
              {VOTE_FILTER_OPTIONS.map((option) => (
                <SelectItem
                  key={option.value}
                  value={option.value}
                  className="text-foreground"
                >
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Select
            value={voterTypeFilter}
            onValueChange={(value) => {
              setVoterTypeFilter(value as VoterTypeFilter);
              table.setPageIndex(0);
            }}
          >
            <SelectTrigger
              className="w-full sm:w-[170px] text-white/60"
              aria-label="Filter by voter type"
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent className="select-background">
              {VOTER_TYPE_FILTER_OPTIONS.map((option) => (
                <SelectItem
                  key={option.value}
                  value={option.value}
                  className="text-foreground"
                >
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <div className="flex gap-3 w-full sm:w-auto">
            <AppButton
              variant="outline"
              onClick={handleReset}
              className="bg-transparent text-white"
            >
              Reset
            </AppButton>
          </div>
          <AppButton
            variant="outline"
            size="icon"
            className="hidden lg:flex bg-transparent text-white"
            aria-label="Download top voters"
            onClick={handleDownload}
            disabled={
              isFetchingVotes ||
              table.getPrePaginationRowModel().rows.length === 0
            }
          >
            <Download className="size-4" />
          </AppButton>
        </div>
      </div>

      {focusedValidator && (
        <div className="flex items-center justify-between gap-3 border-b border-white/10 px-6 py-3 text-sm text-white/60">
          <span>
            Showing the vote of{" "}
            <span className="font-medium text-white">
              {focusedValidator.validatorName}
            </span>{" "}
            and its staker overrides
          </span>
          <button
            type="button"
            onClick={() => {
              setFocusedValidator(null);
              table.setPageIndex(0);
            }}
            className="flex items-center gap-1 text-white/50 hover:text-white transition-colors"
            aria-label="Show all validators"
          >
            <X className="size-4" />
            Clear
          </button>
        </div>
      )}

      <div className="overflow-x-auto">
        <Table className="w-full min-w-[720px]">
          <TableHeader>
            {table.getHeaderGroups().map((headerGroup) => (
              <TableRow
                key={headerGroup.id}
                className="border-b border-white/10 hover:bg-transparent"
              >
                {headerGroup.headers.map((header) => {
                  const columnId = header.column.id;
                  return (
                    <TableHead
                      key={header.id}
                      className={cn(
                        "px-6 py-4 text-xs font-semibold uppercase tracking-wider text-white/50",
                        columnId === "validatorName"
                          ? "text-left"
                          : "text-center",
                      )}
                    >
                      {header.isPlaceholder
                        ? null
                        : flexRender(
                            header.column.columnDef.header,
                            header.getContext(),
                          )}
                    </TableHead>
                  );
                })}
              </TableRow>
            ))}
          </TableHeader>
          <TableBody>
            {isLoadingVotes ? (
              <TableRow className="hover:bg-transparent">
                <TableCell
                  colSpan={columns.length}
                  className="h-28 text-center text-sm text-white/60"
                >
                  Loading votes...
                </TableCell>
              </TableRow>
            ) : table.getRowModel().rows.length ? (
              table.getRowModel().rows.map((row) => (
                <TableRow
                  key={row.id}
                  className="border-b border-white/5 bg-transparent hover:bg-transparent"
                >
                  {row.getVisibleCells().map((cell) => (
                    <TableCell
                      key={cell.id}
                      className={cn(
                        "px-6 py-5 text-sm",
                        cell.column.id === "validatorName"
                          ? "text-left"
                          : "text-center",
                      )}
                    >
                      {flexRender(
                        cell.column.columnDef.cell,
                        cell.getContext(),
                      )}
                    </TableCell>
                  ))}
                </TableRow>
              ))
            ) : (
              <TableRow className="hover:bg-transparent">
                <TableCell
                  colSpan={columns.length}
                  className="h-28 text-center text-sm text-white/60"
                >
                  {hasActiveFilters
                    ? "No voters match your filters."
                    : "No votes found for this proposal."}
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>

      <div className="space-y-4 border-t border-white/10 bg-black/20 px-6 py-5">
        <TablePaginationMobile
          table={table}
          totalLabel="Voters"
          totalCount={filteredData.length}
          pageSizeOptions={[10, 20, 30]}
        />
        <TablePaginationDesktop
          table={table}
          totalLabel="Total Voters"
          totalCount={filteredData.length}
          pageSizeOptions={[10, 20, 30, 40, 50]}
        />
      </div>
    </div>
  );
}
