"use client";

import * as React from "react";
import { Search, ListFilter } from "lucide-react";
import { AppButton } from "@/components/ui/AppButton";
import { FilterState } from "./ProposalFilterModal";
import { ProposalFilterModal } from "./ProposalFilterModal";

interface FilterPanelProps {
  searchQuery: string;
  onSearchQueryChange: (query: string) => void;
  filterState: FilterState;
  onFilterStateChange: (filters: FilterState) => void;
  disabled?: boolean;
}

function countActiveFilters(filterState: FilterState) {
  let count = 0;

  if (filterState.onlyEligible) count++;
  if (filterState.status !== "all") count++;
  if (filterState.lifecycle !== "all") count++;
  if (filterState.timeWindow !== "all") count++;
  if (filterState.minimumSOL > 0) count++;

  return count;
}

export default function FilterPanel({
  searchQuery,
  onSearchQueryChange,
  filterState,
  onFilterStateChange,
  disabled,
}: FilterPanelProps) {
  const [isFilterModalOpen, setIsFilterModalOpen] = React.useState(false);

  const activeFilterCount = countActiveFilters(filterState);

  return (
    <>
      <div className="glass-card p-6 space-y-4">
        <div className="flex items-center gap-3">
          <div className="relative flex-1">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-white/40" />
            <input
              type="text"
              placeholder="Search proposals..."
              value={searchQuery}
              onChange={(e) => onSearchQueryChange(e.target.value)}
              className="w-full pl-10 pr-4 py-3 input"
              disabled={disabled}
            />
          </div>
          <div className="relative">
            <AppButton
              aria-label="Filter"
              variant="outline"
              size="sm"
              className="flex size-11 items-center justify-center"
              icon={<ListFilter className="size-4" />}
              onClick={() => setIsFilterModalOpen(true)}
              disabled={disabled}
            />
            {activeFilterCount > 0 && (
              <span className="absolute -top-1 -right-1 flex size-5 items-center justify-center rounded-full bg-gradient-to-r from-primary to-secondary text-[10px] font-bold text-foreground">
                {activeFilterCount}
              </span>
            )}
          </div>
        </div>
      </div>

      {/* Filter Modal - responsive for all screen sizes */}
      {isFilterModalOpen && (
        <ProposalFilterModal
          isOpen={isFilterModalOpen}
          onClose={() => setIsFilterModalOpen(false)}
          onApply={onFilterStateChange}
          initialFilters={filterState}
        />
      )}
    </>
  );
}
