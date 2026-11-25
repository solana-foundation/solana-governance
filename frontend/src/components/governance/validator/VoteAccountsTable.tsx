"use client";

import * as React from "react";
import {
  ColumnFiltersState,
  SortingState,
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getPaginationRowModel,
  getSortedRowModel,
  useReactTable,
} from "@tanstack/react-table";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { columns } from "@/components/governance/validator/ValidatorColumns";
import { TableFilters } from "@/components/governance/shared/TableFilters";
import {
  TablePaginationMobile,
  TablePaginationDesktop,
} from "@/components/governance/shared/TablePagination";
import {
  MobileRowDrawer,
  DetailRow,
} from "@/components/governance/shared/MobileRowDrawer";
import { CopyableAddress } from "@/components/governance/shared/CopyableAddress";
import {
  formatAddress,
  formatCommission,
  formatLamportsDisplay,
  formatOptionalCount,
} from "@/lib/governance/formatters";
import { OldVoteAccountData } from "@/types";
import { useVoteAccountsWithValidators, VoteValidatorEntry } from "@/hooks";

const stakeSizeOptions = [
  { value: "All", label: "Stake Size" },
  { value: "1000", label: "> 1,000 SOL" },
  { value: "10000", label: "> 10,000 SOL" },
  { value: "100000", label: "> 100,000 SOL" },
  { value: "1000000", label: "> 1,000,000 SOL" },
];

export function VoteAccountsTable() {
  // State Management
  const [sorting, setSorting] = React.useState<SortingState>([]);
  const [columnFilters, setColumnFilters] = React.useState<ColumnFiltersState>(
    []
  );
  const [searchValue, setSearchValue] = React.useState("");
  const [stakeSizeFilter, setStakeSizeFilter] = React.useState("All");

  // Mobile drawer state
  const [selectedRow, setSelectedRow] =
    React.useState<OldVoteAccountData | null>(null);
  const [drawerOpen, setDrawerOpen] = React.useState(false);

  const {
    data: votingAccountsValidators,
    isLoading,
    // error,
  } = useVoteAccountsWithValidators();

  const data = React.useMemo(() => {
    const values: VoteValidatorEntry[] = votingAccountsValidators
      ? Object.values(votingAccountsValidators.voteMap)
      : [];
    return values.flatMap((v) => v.voteAccount);
  }, [votingAccountsValidators]);

  // Data Filtering
  const filteredData = React.useMemo(() => {
    let filtered = [...data];

    if (searchValue) {
      filtered = filtered.filter((row) =>
        row.voteAccount
          .toLowerCase()
          .includes(searchValue.toLowerCase())
      );
    }

    if (stakeSizeFilter !== "All") {
      const threshold = parseFloat(stakeSizeFilter) * 1e9;
      filtered = filtered.filter((row) => row.activeStake >= threshold);
    }

    return filtered;
  }, [data, searchValue, stakeSizeFilter]);

  // Table Setup
  const table = useReactTable({
    data: filteredData,
    columns,
    state: { sorting, columnFilters },
    onSortingChange: setSorting,
    onColumnFiltersChange: setColumnFilters,
    getCoreRowModel: getCoreRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
    initialState: {
      pagination: { pageSize: 10 },
    },
  });

  // Handlers
  const handleReset = () => {
    setSearchValue("");
    setStakeSizeFilter("All");
    setColumnFilters([]);
    setSorting([]);
  };

  const handleRowClick = (rowData: OldVoteAccountData) => {
    // Only on mobile (check if screen width is less than sm breakpoint)
    if (window.innerWidth < 640) {
      setSelectedRow(rowData);
      setDrawerOpen(true);
    }
  };

  return (
    <div className="space-y-4">
      {/* Search and Filters */}
      <TableFilters
        title="Vote Accounts"
        searchValue={searchValue}
        onSearchChange={setSearchValue}
        searchPlaceholder="Search vote accounts..."
        filters={[
          {
            label: "Stake Size",
            value: stakeSizeFilter,
            onChange: setStakeSizeFilter,
            options: stakeSizeOptions,
            placeholder: "Stake Size",
          },
        ]}
        onReset={handleReset}
      />

      {/* Table */}
      <div className="rounded-2xl border glass-card overflow-hidden">
        <div className="sm:overflow-x-auto">
          <Table className="w-full table-auto sm:table-fixed">
            <TableHeader>
              {table.getHeaderGroups().map((headerGroup) => (
                <TableRow
                  key={headerGroup.id}
                  className="hover:bg-transparent border-white/10"
                >
                  {headerGroup.headers.map((header) => {
                    const columnId = header.column.id;
                    const isMobileHidden = [
                      "identity",
                      "commission",
                      "lastVote",
                      "credits",
                    ].includes(columnId);

                    return (
                      <TableHead
                        key={header.id}
                        className={`text-xs font-semibold uppercase tracking-wide text-white/50 text-center px-2 sm:px-4
                          ${isMobileHidden ? "hidden sm:table-cell" : ""}
                          ${
                            columnId === "vote_account" ? "w-3/5 sm:w-auto" : ""
                          }
                          ${
                            columnId === "active_stake" ? "w-2/5 sm:w-auto" : ""
                          }
                        `}
                      >
                        {header.isPlaceholder
                          ? null
                          : flexRender(
                              header.column.columnDef.header,
                              header.getContext()
                            )}
                      </TableHead>
                    );
                  })}
                </TableRow>
              ))}
            </TableHeader>
            <TableBody>
              {(() => {
                if (isLoading) {
                  return (
                    <>
                      {[...Array(4)].map((_, i) => (
                        <TableRow
                          key={`skeleton-${i}`}
                          className="animate-pulse"
                        >
                          {table.getAllColumns().map((col) => (
                            <TableCell
                              key={col.id}
                              className="py-5 px-6 text-center"
                            >
                              <div className="mx-auto h-4 w-3/4 rounded bg-white/10" />
                            </TableCell>
                          ))}
                        </TableRow>
                      ))}
                    </>
                  );
                }
                if (table.getRowModel().rows.length === 0) {
                  return (
                    <TableRow className="hover:bg-transparent">
                      <TableCell
                        colSpan={columns.length}
                        className="h-24 text-center text-white/60"
                      >
                        No results.
                      </TableCell>
                    </TableRow>
                  );
                }

                return table.getRowModel().rows.map((row) => (
                  <TableRow
                    key={row.id}
                    className="border-white/10 hover:bg-white/5 sm:hover:bg-transparent cursor-pointer sm:cursor-default"
                    onClick={() => handleRowClick(row.original)}
                  >
                    {row.getVisibleCells().map((cell) => {
                      const columnId = cell.column.id;
                      const isMobileHidden = [
                        "identity",
                        "commission",
                        "lastVote",
                        "credits",
                      ].includes(columnId);

                      return (
                        <TableCell
                          key={cell.id}
                          className={`py-4 text-white/80 text-center px-2 sm:px-4
                            ${isMobileHidden ? "hidden sm:table-cell" : ""}
                          `}
                        >
                          {flexRender(
                            cell.column.columnDef.cell,
                            cell.getContext()
                          )}
                        </TableCell>
                      );
                    })}
                  </TableRow>
                ));
              })()}
            </TableBody>
          </Table>
        </div>
      </div>

      {/* Pagination */}
      <TablePaginationMobile
        table={table}
        totalLabel="Total"
        totalCount={filteredData.length}
      />
      <TablePaginationDesktop
        table={table}
        totalLabel="Total Validators"
        totalCount={filteredData.length}
      />

      {/* Mobile Row Details Drawer */}
      <MobileRowDrawer
        open={drawerOpen}
        onOpenChange={setDrawerOpen}
        title="Vote Account Details"
      >
        {selectedRow && (
          <>
            <DetailRow
              label="Vote Account"
              value={
                <CopyableAddress
                  address={selectedRow.voteAccount}
                  shortenedLength={8}
                  copyLabel="Copy full address"
                />
              }
              fullWidth
            />
            <DetailRow
              label="Identity"
              value={
                selectedRow.identity
                  ? formatAddress(selectedRow.identity, 8)
                  : "-"
              }
              fullWidth
            />
            <DetailRow
              label="Delegated Stake"
              value={formatLamportsDisplay(selectedRow.activeStake).value}
            />
            <DetailRow
              label="Commission"
              value={formatCommission(selectedRow.commission)}
            />
            <DetailRow
              label="Last Vote"
              value={formatOptionalCount(selectedRow.lastVote)}
            />
            <DetailRow
              label="Credits"
              value={formatOptionalCount(selectedRow.credits)}
            />
          </>
        )}
      </MobileRowDrawer>
    </div>
  );
}
