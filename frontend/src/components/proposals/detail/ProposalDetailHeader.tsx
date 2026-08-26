"use client";

import Link from "next/link";
import { calculateTimeAgo } from "@/helpers";
import { bigintToSafeNumber } from "@/helpers/bigint";
import StatusBadge from "@/components/ui/StatusBadge";
import { CheckIcon, CopyIcon, Github } from "lucide-react";
import { useCopyToClipboard } from "@/hooks";
import LifecycleIndicator from "@/components/ui/LifecycleIndicator";
import { formatAddress } from "@/lib/governance/formatters";
import type { ProposalRecord } from "@/types";
import { ProposalDescription } from "../ProposalDescription";
import { ProposalRefLabel } from "../ProposalRefLabel";

interface ProposalDetailHeaderProps {
  proposal: ProposalRecord | undefined;
  isLoading: boolean;
}

function InfoItem({
  label,
  children,
}: {
  label: string;
  isLoading: boolean;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center gap-1 sm:gap-1.5 lg:gap-2">
      <p className="text-white/30 font-semibold text-xs lg:text-sm">{label}</p>
      {children}
    </div>
  );
}

export default function ProposalDetailHeader({
  proposal,
  isLoading,
}: ProposalDetailHeaderProps) {
  const { copied, copyToClipboard } = useCopyToClipboard();

  if (isLoading) return <ProposalDetailSkeleton />;
  if (!proposal) return <div>No proposal data...</div>;

  const createdAgo = calculateTimeAgo(bigintToSafeNumber(proposal.creationTimestamp, "proposal creation timestamp"));
  const votingThrough =
    proposal.endEpoch > 0n ? proposal.endEpoch - 1n : "-";

  return (
    <div className="glass-card space-y-6 p-6">
      <div className="flex flex-col gap-4">
        <div className="flex justify-between items-center space-y-3">
          <h2 className="h2">{proposal.title}</h2>
          <StatusBadge status={proposal.status} variant="pill" />
        </div>

        <ProposalDescription githubUrl={proposal.description} />
      </div>

      <div className="grid grid-cols-2 sm:grid-cols-3 lg:flex lg:flex-wrap items-center gap-2 sm:gap-3 lg:gap-6 border-t border-white/10 pt-3 sm:pt-4 text-sm leading-none lg:leading-normal">
        <InfoItem label="ID:" isLoading={isLoading}>
          {/* No leading "#": the label already carries its own prefix (SIMD-0022 / SGP-0001). */}
          <ProposalRefLabel
            url={proposal.description}
            fallback={proposal.proposalRef}
            className="font-mono text-white/60 text-xs lg:text-sm"
          />
        </InfoItem>

        <InfoItem label="Author:" isLoading={isLoading}>
          <span className="font-mono text-white/60 text-xs lg:text-sm">
            {formatAddress(proposal.author, 4)}
          </span>
          {copied ? (
            <CheckIcon className="size-3 lg:size-4 text-green-500" />
          ) : (
            <CopyIcon
              className="size-3 lg:size-4 text-white/60 hover:cursor-pointer"
              onClick={() => copyToClipboard(proposal.author)}
            />
          )}
        </InfoItem>

        <InfoItem label="Created:" isLoading={isLoading}>
          <p className="text-white/60 text-xs lg:text-sm">{createdAgo}</p>
        </InfoItem>
        <InfoItem label="Voting Available Through:" isLoading={isLoading}>
          <p className="text-white/60 text-xs lg:text-sm">{votingThrough}</p>
        </InfoItem>
        <InfoItem label="Link to proposal:" isLoading={isLoading}>
          <Link
            href={proposal.description}
            target="_blank"
            rel="noreferrer"
            className="hover:cursor-pointer"
          >
            <Github className="size-6 rounded-full lg:round-none lg:size-5 text-white/60 bg-white/10 p-1 lg:p-0 lg:bg-transparent hover:text-primary duration-200 transition-colors" />
          </Link>
        </InfoItem>

        <div className="hidden lg:block lg:ml-auto lg:mr-4">
          <LifecycleIndicator status={proposal.status} />
        </div>
      </div>
    </div>
  );
}

function ProposalDetailSkeleton() {
  return (
    <div className="glass-card space-y-6 p-6">
      <div className="flex flex-col lg:flex-row lg:items-start lg:justify-between gap-4">
        <div className="flex-1 space-y-3">
          {/* title */}
          <div className="h-8 w-1/2 bg-white/10 animate-pulse rounded" />
          <div className="lg:hidden">
            <div className="h-6 w-20 bg-white/10 animate-pulse rounded-full" />
          </div>
          {/* description */}
          <div className="h-9 w-full bg-white/10 animate-pulse rounded" />
        </div>
        <div className="hidden lg:block">
          <div className="h-6 w-20 bg-white/10 animate-pulse rounded-full" />
        </div>
      </div>

      <div className="grid grid-cols-2 sm:grid-cols-3 lg:flex lg:flex-wrap items-center gap-2 sm:gap-3 lg:gap-6 border-t border-white/10 pt-3 sm:pt-4 text-sm leading-none lg:leading-normal">
        <InfoItem label="ID:" isLoading>
          <div className="h-4 w-24 bg-white/10 animate-pulse rounded" />
        </InfoItem>

        <InfoItem label="Author:" isLoading>
          <div className="h-4 w-32 bg-white/10 animate-pulse rounded" />
        </InfoItem>

        <InfoItem label="Created:" isLoading>
          <div className="h-4 w-20 bg-white/10 animate-pulse rounded" />
        </InfoItem>

        <InfoItem label="Voting Available Through:" isLoading>
          <div className="h-4 w-20 bg-white/10 animate-pulse rounded" />
        </InfoItem>

        <InfoItem label="Link to proposal:" isLoading>
          <div className="h-5 w-5 bg-white/10 animate-pulse rounded-full" />
        </InfoItem>

        <div className="hidden lg:block lg:ml-auto lg:mr-4">
          <div className="h-5 w-32 bg-white/10 animate-pulse rounded" />
        </div>
      </div>
    </div>
  );
}
