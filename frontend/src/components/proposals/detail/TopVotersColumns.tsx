"use client";

import { ColumnDef } from "@tanstack/react-table";
import { SortableHeaderButton } from "@/components/governance/shared/SortableHeaderButton";
import { ValidatorLogo } from "./ValidatorLogo";
import {
  formatAddress,
  formatLamportsDisplay,
} from "@/lib/governance/formatters";
import { formatDate } from "@/helpers";
import { TopVoterRecord } from "@/types";
import { humanizeText } from "@/lib/helpers";
import { CopyableAddressIcon } from "@/components/governance/shared/CopyableAddressIcon";

const LABELS = {
  for: "For",
  against: "Against",
  abstain: "Abstain",
};

export const topVoterColumns: ColumnDef<TopVoterRecord>[] = [
  {
    accessorKey: "validatorName",
    header: ({ column }) => (
      <SortableHeaderButton
        column={column}
        label="Voter"
        className="flex items-center justify-start gap-1.5 hover:text-white transition-colors"
      />
    ),
    cell: ({ row }) => {
      const {
        validatorName,
        validatorIdentity,
        validatorImage,
        accentColor,
      } = row.original;

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
    accessorKey: "walletType",
    header: "Voted as",
    cell: ({ row }) => {
      const { walletType, stakeAccount } = row.original;

      return (
        <div className="flex items-center gap-4 justify-center">
          <div className="flex flex-col">
            <span className="text-sm font-medium text-white/60">
              {humanizeText(walletType)}
            </span>
            {stakeAccount && (
              <span className="flex gap-1 text-xs font-mono text-white/30">
                {formatAddress(stakeAccount, 6)}
                <CopyableAddressIcon
                  size={13}
                  address={stakeAccount}
                  copyLabel="Copy full stake account address"
                />
              </span>
            )}
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
        {formatLamportsDisplay(row.original.stakedLamports).value}
      </div>
    ),
    sortingFn: "basic",
  },
  {
    accessorKey: "voteOutcome",
    header: "Voter Split",
    cell: ({ row }) => {
      const { forVotesBp, againstVotesBp, abstainVotesBp } =
        row.original.voteData;
      return (
        <div className="flex items-center justify-center gap-4">
          {/* <div className="h-1.5 w-24 overflow-hidden rounded-full">
            <div
              className="h-full w-full"
              style={{ backgroundImage: BAR_GRADIENTS[outcome] }}
            />
          </div> */}
          <span className="text-xs font-medium text-primary">
            <b>{forVotesBp.toNumber() / 100}%</b> {LABELS.for}
          </span>
          <span className="text-xs font-medium text-destructive">
            <b>{againstVotesBp.toNumber() / 100}%</b> {LABELS.against}
          </span>
          <span className="text-xs font-medium text-white/30">
            <b>{abstainVotesBp.toNumber() / 100}%</b> {LABELS.abstain}
          </span>
        </div>
      );
    },
    sortingFn: "alphanumeric",
    enableSorting: false,
  },
  {
    accessorKey: "votePercentage",
    header: ({ column }) => (
      <SortableHeaderButton column={column} label="Percentage" />
    ),
    cell: ({ row }) => (
      <span className="text-sm text-white/60">
        {row.original.votePercentage.toFixed(2)}%
      </span>
    ),
  },
  {
    accessorKey: "voteTimestamp",
    header: ({ column }) => (
      <SortableHeaderButton column={column} label="Vote Date" />
    ),
    cell: ({ row }) => (
      <span className="text-sm text-white/60">
        {formatDate(row.original.voteTimestamp)}
      </span>
    ),
    sortingFn: "datetime",
  },
];
