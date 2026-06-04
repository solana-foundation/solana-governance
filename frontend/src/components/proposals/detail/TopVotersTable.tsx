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
import { Download, Search } from "lucide-react";

import { useProposalVotes } from "@/hooks/useProposalVotes";
import { topVoterColumns } from "./TopVotersColumns";
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
  TablePaginationDesktop,
  TablePaginationMobile,
} from "@/components/governance/shared/TablePagination";
import { cn } from "@/lib/utils";
import type { ProposalRecord } from "@/types";

const DEFAULT_SORTING: SortingState = [{ id: "stakedLamports", desc: true }];

const TABLE_COLUMNS = topVoterColumns;

interface TopVotersTableProps {
  proposal: ProposalRecord | undefined;
}

export default function TopVotersTable({ proposal }: TopVotersTableProps) {
  const [searchValue, setSearchValue] = React.useState("");
  // const [outcomeFilter, setOutcomeFilter] =
  //   React.useState<VoteOutcomeFilter>("all");
  const [sorting, setSorting] = React.useState<SortingState>(() => [
    ...DEFAULT_SORTING,
  ]);

  // Fetch votes for the proposal
  const { data: topVoters = [], isLoading: isLoadingVotes } = useProposalVotes(
    proposal?.publicKey,
  );

  const filteredData = React.useMemo(() => {
    const searchTerm = searchValue.trim().toLowerCase();
    if (searchTerm.length === 0) {
      return topVoters;
    }

    return topVoters.filter((voter) => {
      const name = voter.validatorName.toLowerCase();
      const identity = voter.validatorIdentity.toLowerCase();
      const stakeAccount = voter.stakeAccount?.toLowerCase() ?? "";

      return (
        name.includes(searchTerm) ||
        identity.includes(searchTerm) ||
        stakeAccount.includes(searchTerm)
      );
    });
  }, [searchValue, topVoters]);

  const table = useReactTable({
    data: filteredData,
    columns: TABLE_COLUMNS,
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
    // setOutcomeFilter("all");
    table.setSorting(nextSorting);
    table.setPageIndex(0);
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
          >
            <Download className="size-4" />
          </AppButton>
        </div>
      </div>

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
                  colSpan={TABLE_COLUMNS.length}
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
                  colSpan={TABLE_COLUMNS.length}
                  className="h-28 text-center text-sm text-white/60"
                >
                  {searchValue.trim()
                    ? "No voters match your search."
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
