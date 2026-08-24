"use client";

import { useMemo } from "react";
import type { ProposalRecord } from "@/types";
import { CircleCheck, CircleX } from "lucide-react";
import VoteItem, { VoteItemSkeleton } from "./VoteItem";
import QuorumDonut, { QuorumDonutSkeleton } from "./QuorumDonut";
import {
  formatLamportsDisplay,
} from "@/lib/governance/formatters";
import { useHasUserVoted } from "@/hooks";
import { formatBigintPercentage } from "@/helpers/bigint";
import { useSnapshotMeta } from "@/hooks/useSnapshotMeta";
import {
  computeQuorum,
  QUORUM_DENOMINATOR,
  QUORUM_NUMERATOR,
  resolveQuorumDenominator,
} from "@/chain/quorum";

/** Shown in place of a percentage when the snapshot total is not known. */
const UNKNOWN_PERCENTAGE = "—";

interface VoteBreakdownWrapperProps {
  proposal: ProposalRecord | undefined;
  isLoading: boolean;
}
interface VoteBreakdownProps {
  proposal: ProposalRecord | undefined;
  isLoading: boolean;
}

export default function VoteBreakdownWrapper({
  proposal,
  isLoading,
}: VoteBreakdownWrapperProps) {
  if (!proposal && !isLoading) return <div>No proposal data...</div>;

  return <VoteBreakdown proposal={proposal} isLoading={isLoading} />;
}

const VoteBreakdown = ({
  proposal,
  isLoading: isLoadingParent,
}: VoteBreakdownProps) => {
  const { data: hasUserVoted = false, isLoading: isLoadingHasUserVoted } =
    useHasUserVoted(proposal?.publicKey);

  // The proposal's own snapshot total, not a live cluster sum: Art. IV.2 fixes
  // the distribution for the voting period, so a live total drifts.
  const { data: meta, isLoading: isLoadingMeta } = useSnapshotMeta();
  const totalActiveStake = resolveQuorumDenominator(
    meta,
    proposal?.snapshotSlot,
  );

  const quorum = useMemo(
    () =>
      proposal
        ? computeQuorum({
            forLamports: proposal.forVotesLamports,
            againstLamports: proposal.againstVotesLamports,
            abstainLamports: proposal.abstainVotesLamports,
            totalActiveStake,
          })
        : undefined,
    [proposal, totalActiveStake],
  );

  // `undefined` when the denominator is unknown, so the percentages render as
  // "—" rather than a "0.00%" that looks like a real tally.
  const votePercentages = useMemo(() => {
    if (!proposal || !quorum?.known) {
      return undefined;
    }

    const total = quorum.totalActiveStake;
    if (typeof total !== "bigint") {
      return undefined;
    }

    return {
      forVotesPercentage: formatBigintPercentage(proposal.forVotesLamports, total),
      againstVotesPercentage: formatBigintPercentage(
        proposal.againstVotesLamports,
        total,
      ),
      abstainVotesPercentage: formatBigintPercentage(
        proposal.abstainVotesLamports,
        total,
      ),
    };
  }, [proposal, quorum]);

  const isLoading = isLoadingParent || isLoadingHasUserVoted || isLoadingMeta;

  if (!proposal && !isLoadingParent) return <div>No proposal info</div>;

  return (
    <div className="glass-card flex h-full flex-col p-6 md:p-6 lg:p-8">
      <div className="flex flex-1 flex-col items-center gap-4 sm:gap-4 md:flex-col lg:flex-row md:items-stretch">
        {/* Quorum Donut Chart */}
        <div className="flex flex-1 items-center justify-center">
          {isLoading || !proposal ? (
            <QuorumDonutSkeleton />
          ) : (
            <QuorumDonut
              forLamports={proposal.forVotesLamports}
              againstLamports={proposal.againstVotesLamports}
              abstainLamports={proposal.abstainVotesLamports}
              totalLamports={
                quorum?.known ? quorum.totalActiveStake : undefined
              }
            />
          )}
        </div>

        {/* Vote Breakdown Section */}
        <div className="flex flex-1 flex-col">
          <div className="mb-4 space-y-2">
            <h4 className="h4 text-center font-semibold lg:text-left">
              Vote Breakdown
            </h4>
            <p className="text-center text-sm text-white/60 lg:text-left">
              Current distribution of recorded votes for this proposal.
            </p>
            {isLoading || !quorum ? (
              <div className="mx-auto h-4 w-52 animate-pulse rounded bg-white/10 lg:mx-0" />
            ) : (
              <p className="text-center text-sm lg:text-left">
                <span className="text-white/60">
                  Participation ({QUORUM_NUMERATOR}/{QUORUM_DENOMINATOR}{" "}
                  needed):{" "}
                </span>
                {quorum.known ? (
                  <span
                    className={
                      quorum.isMet ? "text-emerald-400" : "text-foreground"
                    }
                  >
                    {quorum.participationPercent.toFixed(2)}%
                    {quorum.isMet ? " — quorum met" : ""}
                  </span>
                ) : (
                  // Not "not recorded": the total may exist, just for another
                  // snapshot, which `/meta` cannot be asked for.
                  <span className="text-white/60">
                    {UNKNOWN_PERCENTAGE} (snapshot total unavailable)
                  </span>
                )}
              </p>
            )}
          </div>
          <div className="flex-1 space-y-2 md:space-y-3 lg:space-y-4 mt-1 lg:mt-0">
            {isLoading || proposal === undefined ? (
              <>
                <VoteItemSkeleton label="For" color="bg-primary" />
                <VoteItemSkeleton label="Against" color="bg-destructive" />
                <VoteItemSkeleton label="Abstain" color="bg-white/30" />
              </>
            ) : (
              <>
                <VoteItem
                  label="For"
                  amount={
                    formatLamportsDisplay(proposal.forVotesLamports).value
                  }
                  percentage={
                    votePercentages
                      ? `(${votePercentages.forVotesPercentage}%)`
                      : UNKNOWN_PERCENTAGE
                  }
                  color="bg-primary"
                />
                <VoteItem
                  label="Against"
                  amount={
                    formatLamportsDisplay(proposal.againstVotesLamports).value
                  }
                  percentage={
                    votePercentages
                      ? `(${votePercentages.againstVotesPercentage}%)`
                      : UNKNOWN_PERCENTAGE
                  }
                  color="bg-destructive"
                />
                <VoteItem
                  label="Abstain"
                  amount={
                    formatLamportsDisplay(proposal.abstainVotesLamports).value
                  }
                  percentage={
                    votePercentages
                      ? `(${votePercentages.abstainVotesPercentage}%)`
                      : UNKNOWN_PERCENTAGE
                  }
                  color="bg-white/30"
                />
              </>
            )}
          </div>
          {isLoading ? (
            <div className="h-4 w-20 bg-white/10 animate-pulse rounded" />
          ) : (
            <span className="mt-auto flex items-center gap-2 pt-4 -ml-0.5">
              {hasUserVoted ? (
                <CircleCheck className="size-4 text-emerald-400" />
              ) : (
                <CircleX className="size-4 text-destructive/50" />
              )}
              <p className="text-xs lg:text-sm text-center text-white/60">
                You have {hasUserVoted ? "" : "not "}voted for this proposal.
              </p>
            </span>
          )}
        </div>
      </div>
    </div>
  );
};
