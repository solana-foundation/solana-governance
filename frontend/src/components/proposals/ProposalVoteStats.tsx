"use client";

import type { NetworkMetaResponse } from "@/chain";
import { computeProposalVoteStats, resolveQuorumDenominator } from "@/chain";
import { cn } from "@/lib/utils";
import type { ProposalRecord } from "@/types";

/**
 * Vote share and quorum participation for a proposal, measured against the
 * metadata of the snapshot the proposal votes on.
 */
export function getProposalVoteStats(
  proposal: ProposalRecord,
  snapshotMeta: NetworkMetaResponse | undefined,
) {
  return computeProposalVoteStats({
    forLamports: proposal.forVotesLamports,
    againstLamports: proposal.againstVotesLamports,
    abstainLamports: proposal.abstainVotesLamports,
    totalActiveStake: resolveQuorumDenominator(
      snapshotMeta,
      proposal.snapshotSlot,
    ),
  });
}

const VOTE_SEGMENTS = [
  {
    key: "forPercent",
    label: "For",
    dotClassName: "bg-primary",
    barClassName: "bg-primary",
  },
  {
    key: "againstPercent",
    label: "Against",
    dotClassName: "bg-destructive",
    barClassName: "bg-destructive",
  },
  {
    key: "abstainPercent",
    label: "Abstain",
    dotClassName: "bg-white/40",
    barClassName: "bg-white/40",
  },
] as const;

interface ProposalVoteShareProps {
  proposal: ProposalRecord;
  snapshotMeta: NetworkMetaResponse | undefined;
  className?: string;
}

/**
 * Stacked bar of the For / Against / Abstain share of votes cast, with each
 * share labelled underneath. Renders a dash when no votes have been cast.
 */
export function ProposalVoteShare({
  proposal,
  snapshotMeta,
  className,
}: ProposalVoteShareProps) {
  const { voteShare } = getProposalVoteStats(proposal, snapshotMeta);

  if (!voteShare.known) {
    return <span className="text-sm font-semibold text-white/40">—</span>;
  }

  return (
    <div className={cn("space-y-2 tabular-nums", className)}>
      <div
        className="flex h-1.5 overflow-hidden rounded-full bg-white/8"
        aria-label={`Share of votes cast: For ${voteShare.forPercent.toFixed(2)}%, Against ${voteShare.againstPercent.toFixed(2)}%, Abstain ${voteShare.abstainPercent.toFixed(2)}%`}
        role="img"
      >
        {VOTE_SEGMENTS.map((segment) => (
          <span
            key={segment.key}
            className={segment.barClassName}
            style={{ width: `${voteShare[segment.key]}%` }}
          />
        ))}
      </div>
      <div className="grid grid-cols-3 gap-2 text-[10px] text-white/55">
        {VOTE_SEGMENTS.map((segment) => (
          <span
            key={segment.key}
            className="flex flex-col items-center gap-0.5 whitespace-nowrap"
          >
            <span className="inline-flex items-center gap-1">
              <span
                className={`size-1.5 rounded-full ${segment.dotClassName}`}
                aria-hidden
              />
              {segment.label}
            </span>
            <span className="font-medium text-white/75">
              {voteShare[segment.key].toFixed(2)}%
            </span>
          </span>
        ))}
      </div>
    </div>
  );
}
