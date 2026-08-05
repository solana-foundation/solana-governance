"use client";

import { ColumnDef } from "@tanstack/react-table";
import { SortableHeaderButton } from "@/components/governance/shared/SortableHeaderButton";
import { ValidatorLogo } from "./ValidatorLogo";
import {
  formatAddress,
  formatLamportsDisplay,
} from "@/lib/governance/formatters";
import { TopSupporterRecord } from "@/types";
import { CopyableAddressIcon } from "@/components/governance/shared/CopyableAddressIcon";

/** Shown when validator stake could not be loaded, so an unknown value is not
 * rendered as a real-looking zero. */
const UNKNOWN_VALUE = "—";

export const topSupporterColumns: ColumnDef<TopSupporterRecord>[] = [
  {
    accessorKey: "validatorName",
    header: ({ column }) => (
      <SortableHeaderButton
        column={column}
        label="Supporter"
        className="flex items-center justify-start gap-1.5 hover:text-white transition-colors"
      />
    ),
    cell: ({ row }) => {
      const { validatorName, validatorIdentity, validatorImage, accentColor } =
        row.original;

      return (
        <div className="flex items-center gap-4">
          <ValidatorLogo
            validatorName={validatorName}
            validatorImage={validatorImage}
            accentColor={accentColor}
          />
          <div className="flex flex-col text-left">
            <span className="text-sm font-medium text-white/60">
              {validatorName}
            </span>
            <span className="flex gap-1 text-xs font-mono text-white/30">
              {formatAddress(validatorIdentity, 6)}
              <CopyableAddressIcon
                size={13}
                address={validatorIdentity}
                copyLabel="Copy full validator address"
              />
            </span>
          </div>
        </div>
      );
    },
    sortingFn: "alphanumeric",
    enableHiding: false,
  },
  {
    accessorKey: "stakedLamports",
    header: ({ column }) => (
      <SortableHeaderButton column={column} label="Staked" />
    ),
    cell: ({ row }) => (
      <div className="text-sm text-white/60">
        {row.original.stakedLamports === undefined
          ? UNKNOWN_VALUE
          : formatLamportsDisplay(row.original.stakedLamports).value}
      </div>
    ),
    sortingFn: "basic",
  },
  {
    accessorKey: "stakePercentage",
    header: ({ column }) => (
      <SortableHeaderButton column={column} label="Percentage" />
    ),
    cell: ({ row }) => (
      <span className="text-sm text-white/60">
        {row.original.stakePercentage === undefined
          ? UNKNOWN_VALUE
          : `${row.original.stakePercentage.toFixed(2)}%`}
      </span>
    ),
    sortingFn: "basic",
  },
];
