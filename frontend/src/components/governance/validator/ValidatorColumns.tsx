"use client";

import { ColumnDef } from "@tanstack/react-table";
import {
  formatAddress,
  formatCommission,
  formatLamportsDisplay,
  formatOptionalCount,
} from "@/lib/governance/formatters";
import { CopyableAddress } from "@/components/governance/shared/CopyableAddress";
import { SortableHeaderButton } from "@/components/governance/shared/SortableHeaderButton";
import { OldVoteAccountData } from "@/types";

export const columns: ColumnDef<OldVoteAccountData>[] = [
  {
    accessorKey: "voteAccount",
    header: "VOTE ACCOUNT",
    cell: ({ row }) => {
      const account = row.original.voteAccount;
      return (
        <>
          {/* Mobile: Simple text without copy button */}
          <div className="sm:hidden">
            <p className="font-mono text-white/90 text-xs">
              {formatAddress(account.toBase58(), 4)}
            </p>
          </div>
          {/* Desktop: Full CopyableAddress component */}
          <div className="hidden sm:block">
            <CopyableAddress
              address={account.toBase58()}
              shortenedLength={4}
              copyLabel="Copy full address"
            />
          </div>
        </>
      );
    },
  },
  {
    accessorKey: "identity",
    header: () => <span className="hidden sm:inline">IDENTITY</span>,
    cell: ({ row }) => {
      const { name, identity } = row.original;
      // The name where the validator is known, the address otherwise; the
      // column is too narrow for both.
      const cellValue =
        name ?? (identity ? formatAddress(identity.toBase58(), 6) : undefined);

      return (
        <p className="hidden sm:block font-mono text-xs lg:text-sm">
          {cellValue || "-"}
        </p>
      );
    },
  },
  {
    accessorKey: "activeStake",
    header: "DELEGATED STAKE",
    cell: ({ row }) => {
      const stake = row.getValue("activeStake") as number;
      const solAmount = formatLamportsDisplay(stake);
      return <div>{solAmount.value}</div>;
    },
  },
  {
    accessorKey: "commission",
    header: () => <span className="hidden sm:inline">COMMISSION</span>,
    cell: ({ row }) => {
      const commission = row.getValue("commission") as number | undefined;
      const formattedCommission = formatCommission(commission);
      return <div className="hidden sm:block">{formattedCommission}</div>;
    },
  },
  {
    accessorKey: "lastVote",
    header: ({ column }) => (
      <div className="hidden sm:block">
        <SortableHeaderButton column={column} label="LAST VOTE" />
      </div>
    ),
    cell: ({ row }) => {
      const lastVote = row.getValue("lastVote") as number | undefined;
      const formattedLastVote = formatOptionalCount(lastVote);
      return <div className="hidden sm:block">{formattedLastVote}</div>;
    },
  },
  {
    accessorKey: "credits",
    header: ({ column }) => (
      <div className="hidden sm:block">
        <SortableHeaderButton column={column} label="CREDITS" />
      </div>
    ),
    cell: ({ row }) => {
      const credits = row.getValue("credits") as number | undefined;
      const formattedCredits = formatOptionalCount(credits);
      return <div className="hidden sm:block">{formattedCredits}</div>;
    },
  },
];
