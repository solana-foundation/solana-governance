"use client";

import { AnimatePresence, motion } from "framer-motion";
import {
  ColumnFiltersState,
  ExpandedState,
  SortingState,
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getPaginationRowModel,
  getSortedRowModel,
  useReactTable,
  type OnChangeFn,
} from "@tanstack/react-table";
import { Check, ChevronDown } from "lucide-react";

import { columns } from "./Columns";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Pagination } from "@/components/ui/AppPagniation";
import { AppButton } from "@/components/ui/AppButton";
import ExternalProposalPanel from "./ExternalProposalPanel";
import { ProposalStatus } from "@/types";
import { useProposals } from "@/hooks";
import { Fragment, useCallback, useEffect, useMemo, useState } from "react";

const TABLE_COLUMNS = columns;

type StatusFilter = "all" | ProposalStatus;

const STATUS_FILTER_LABELS: Record<StatusFilter, string> = {
  all: "Filter by",
  supporting: "Supporting",
  discussion: "Discussion",
  voting: "Voting",
  finalized: "Finalized",
  failed: "Failed",
};

const filterOptions: StatusFilter[] = [
  "all",
  "supporting",
  "discussion",
  "voting",
  "finalized",
  "failed",
];

const getIsExpanded = (state: ExpandedState, rowId: string) =>
  Boolean((state as Record<string, boolean>)[rowId]);

export default function ProposalsTable({ title }: { title: string }) {
  const [sorting, setSorting] = useState<SortingState>([]);
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);
  const [expanded, setExpanded] = useState<ExpandedState>({});
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("all");
  const [showEligibleOnly, setShowEligibleOnly] = useState(false);

  const { data: proposalsData, isLoading: isLoadingProposals } = useProposals();

  const data = useMemo(() => proposalsData || [], [proposalsData]);

  const handleExpandedChange = useCallback<OnChangeFn<ExpandedState>>(
    (updater) => {
      setExpanded((previous) => {
        const nextState =
          typeof updater === "function" ? updater(previous) : updater ?? {};

        const openEntries = Object.entries(nextState).filter(([, isOpen]) =>
          Boolean(isOpen)
        );

        if (openEntries.length === 0) return {};

        const newlyOpened = openEntries.find(
          ([rowId]) => !getIsExpanded(previous, rowId)
        );
        const [rowId] = newlyOpened ?? openEntries[0];

        return { [rowId]: true } satisfies ExpandedState;
      });
    },
    []
  );

  const handleRowToggle = useCallback((rowId: string) => {
    setExpanded((previous) =>
      getIsExpanded(previous, rowId)
        ? {}
        : ({ [rowId]: true } satisfies ExpandedState)
    );
  }, []);

  const getDefaultExpanded = useCallback((): ExpandedState => {
    if (data.length === 0) {
      return {};
    }

    return { [data[0].id]: true } satisfies ExpandedState;
  }, [data]);

  useEffect(() => {
    // Only expand first row on client side after mount
    if (data.length > 0 && Object.keys(expanded).length === 0) {
      setExpanded(getDefaultExpanded());
    }
  }, [data, expanded, getDefaultExpanded]);

  const table = useReactTable({
    data,
    columns: TABLE_COLUMNS,
    state: {
      sorting,
      columnFilters,
      expanded,
    },
    getCoreRowModel: getCoreRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
    getSortedRowModel: getSortedRowModel(),
    onSortingChange: setSorting,
    onColumnFiltersChange: setColumnFilters,
    onExpandedChange: handleExpandedChange,
    getRowId: (row) => row.id,
    initialState: {
      pagination: {
        pageSize: 5,
      },
    },
  });

  const handleReset = useCallback(() => {
    setSorting([]);
    setColumnFilters([]);
    setStatusFilter("all");
    setShowEligibleOnly(false);
    setExpanded(getDefaultExpanded());
    table.setPageIndex(0);
  }, [getDefaultExpanded, table]);

  useEffect(() => {
    if (statusFilter !== "all") {
      table.getColumn("status")?.setFilterValue(statusFilter);
    } else {
      table.getColumn("status")?.setFilterValue(undefined);
    }
  }, [statusFilter, table]);

  const statusFilterLabel = STATUS_FILTER_LABELS[statusFilter] ?? "Filter by";

  return (
    <div className="space-y-8">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div className="flex flex-wrap items-center gap-5 text-xs text-white/70">
          <h3 className="h3 font-semibold tracking-wide text-white">{title}</h3>
          <label className="inline-flex items-center gap-2 font-medium text-sm">
            <span className="relative flex size-4 items-center justify-center">
              <input
                type="checkbox"
                checked={showEligibleOnly}
                onChange={(e) => setShowEligibleOnly(e.target.checked)}
                className="peer size-full appearance-none rounded border border-white/20 bg-transparent transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50 checked:border-primary/80 checked:bg-primary/90"
              />
              <Check
                strokeWidth={3}
                className="pointer-events-none absolute size-3 text-black opacity-0 transition-opacity peer-checked:opacity-100"
              />
            </span>
            My Eligible Proposals
          </label>
        </div>
        <div className="flex flex-wrap items-center gap-3 text-xs text-white/70">
          <div className="hidden sm:block">
            <Pagination
              page={table.getState().pagination.pageIndex + 1}
              totalPages={table.getPageCount()}
              onPageChange={(page) => table.setPageIndex(page - 1)}
            />
          </div>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <AppButton
                variant="outline"
                size="sm"
                text={statusFilterLabel}
                icon={<ChevronDown className="size-3.5" strokeWidth={2.5} />}
                iconPosition="right"
                className="text-xs"
              />
            </DropdownMenuTrigger>
            <DropdownMenuContent
              align="end"
              className="w-44 border-white/10 bg-background/40 text-xs text-white/80 backdrop-blur"
            >
              <DropdownMenuRadioGroup
                value={statusFilter}
                onValueChange={(value) =>
                  setStatusFilter(value as StatusFilter)
                }
              >
                {filterOptions.map((option) => (
                  <DropdownMenuRadioItem
                    key={option}
                    value={option}
                    className="text-white/80"
                  >
                    {STATUS_FILTER_LABELS[option]}
                  </DropdownMenuRadioItem>
                ))}
              </DropdownMenuRadioGroup>
            </DropdownMenuContent>
          </DropdownMenu>
          <AppButton
            variant="outline"
            size="sm"
            text="Reset"
            onClick={handleReset}
            className="text-xs"
          />
        </div>
      </div>
      <div className="overflow-hidden rounded-2xl border border-white/10 glass-card">
        <Table>
          <TableHeader>
            {table.getHeaderGroups().map((headerGroup) => (
              <TableRow key={headerGroup.id} className="hover:bg-transparent">
                {headerGroup.headers.map((header) => (
                  <TableHead
                    key={header.id}
                    className="text-xs font-semibold uppercase tracking-wide text-white/50 text-center"
                  >
                    {header.isPlaceholder
                      ? null
                      : flexRender(
                          header.column.columnDef.header,
                          header.getContext()
                        )}
                  </TableHead>
                ))}
              </TableRow>
            ))}
          </TableHeader>
          <TableBody>
            {(() => {
              if (isLoadingProposals) {
                return (
                  <>
                    {[...Array(4)].map((_, i) => (
                      <TableRow
                        key={`skeleton-${i}`}
                        className="animate-pulse hover:bg-transparent"
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
                      colSpan={table.getAllColumns().length}
                      className="h-24 text-center text-sm text-white/60"
                    >
                      No proposals available.
                    </TableCell>
                  </TableRow>
                );
              }

              return table.getRowModel().rows.map((row) => (
                <Fragment key={row.id}>
                  <TableRow
                    data-state={row.getIsExpanded() ? "open" : undefined}
                    className="cursor-pointer select-none transition hover:bg-white/3 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40"
                    onClick={() => handleRowToggle(row.id)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter" || event.key === " ") {
                        event.preventDefault();
                        handleRowToggle(row.id);
                      }
                    }}
                    tabIndex={0}
                  >
                    {row.getVisibleCells().map((cell) => (
                      <TableCell
                        key={cell.id}
                        className={`py-5 px-6 ${
                          cell.column.id === "simd"
                            ? "text-left"
                            : "text-center"
                        }`}
                      >
                        {flexRender(
                          cell.column.columnDef.cell,
                          cell.getContext()
                        )}
                      </TableCell>
                    ))}
                  </TableRow>
                  <AnimatePresence initial={false} mode="wait">
                    {row.getIsExpanded() && (
                      <TableRow
                        key={`expanded-${row.id}`}
                        className="hover:bg-transparent"
                      >
                        <TableCell
                          colSpan={row.getVisibleCells().length}
                          className="p-0 bg-black/5"
                        >
                          <motion.div
                            initial={{ height: 0, opacity: 0 }}
                            animate={{
                              height: "auto",
                              opacity: 1,
                              transition: {
                                height: { duration: 0.25, ease: "easeInOut" },
                                opacity: { duration: 0.2, delay: 0.1 },
                              },
                            }}
                            exit={{
                              height: 0,
                              opacity: 0,
                              transition: {
                                height: { duration: 0.25, ease: "easeInOut" },
                                opacity: { duration: 0.2 },
                              },
                            }}
                            style={{ overflow: "hidden" }}
                          >
                            <div className="bg-black/5">
                              <ExternalProposalPanel proposal={row.original} />
                            </div>
                          </motion.div>
                        </TableCell>
                      </TableRow>
                    )}
                  </AnimatePresence>
                </Fragment>
              ));
            })()}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}
