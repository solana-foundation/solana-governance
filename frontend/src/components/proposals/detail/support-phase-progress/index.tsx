"use client";

import { useMemo } from "react";
import { formatSOL } from "@/lib/governance/formatters";
import { ProposalRecord } from "@/types";
import {
  buildSupportFilters,
  useGetValidators,
  useSupportAccounts,
  useEpochToDate,
} from "@/hooks";
import { useGovernanceConfigContext } from "@/contexts/GovernanceConfigContext";
import {
  epochConstantsFromGovernanceConfig,
  getProposalPhaseEpochs,
  supportThresholdPercentFromConfig,
} from "@/lib/proposals";
import { NotificationButton } from "./NotificationButton";
import { PhaseStatusBadge } from "./PhaseStatusBadge";
import { SupportDonut } from "./SupportDonut";
import { StatBadge, StatCard } from "./StatCard";
import { TimeRemainingCarousel } from "./TimeRemainingCarousel";
import { computeSupportStats } from "./stats";

/** Mock total active staked SOL across the network (in lamports) */
// const MOCK_TOTAL_STAKED_LAMPORTS = 316_010_000 * LAMPORTS_PER_SOL; // 316.01M SOL

/** Mock total number of validators in the network */
// const MOCK_TOTAL_VALIDATORS = 2300;

interface SupportPhaseProgressProps {
  proposal: ProposalRecord;
}

export function SupportPhaseProgress({ proposal }: SupportPhaseProgressProps) {
  const governanceConfigQuery = useGovernanceConfigContext();
  const epochs = governanceConfigQuery.data
    ? epochConstantsFromGovernanceConfig(governanceConfigQuery.data)
    : undefined;
  const hasEnded =
    proposal.status === "failed" || proposal.status === "finalized";

  const phaseEpochs =
    epochs !== undefined
      ? getProposalPhaseEpochs(proposal.creationEpoch, epochs, {
          voting: proposal.voting,
          startEpoch: proposal.startEpoch,
        })
      : undefined;

  const { data: supportEndsAt, isLoading: isLoadingEpochDate } =
    useEpochToDate(phaseEpochs?.supportEndEpoch);

  const { data: discussionEndsAt, isLoading: isLoadingDiscussionEpochDate } =
    useEpochToDate(phaseEpochs?.discussionEndEpoch);

  const supportFilters = buildSupportFilters(
    proposal.publicKey.toBase58(),
    null,
  );

  const fetchSupportAccountsEnabled = supportFilters.length > 0; // at least one filter is required

  const { data: supportAccounts = [], isLoading: isLoadingSupportAccounts } =
    useSupportAccounts(supportFilters, fetchSupportAccountsEnabled);

  const { data: validators, isLoading: isLoadingValidators } =
    useGetValidators();

  const numOfValidators = useMemo(() => validators?.length || 0, [validators]);
  const validatorsStake = useMemo(
    () => validators?.reduce((acc, curr) => acc + curr.activated_stake, 0) || 0,
    [validators],
  );

  const isLoading =
    isLoadingValidators ||
    isLoadingSupportAccounts ||
    isLoadingEpochDate ||
    isLoadingDiscussionEpochDate ||
    governanceConfigQuery.isLoading ||
    governanceConfigQuery.isPending;

  const configData = governanceConfigQuery.data;

  // The on-chain record that the support threshold was crossed. The program
  // measured it against the epoch-stakes total of the crossing epoch, which the
  // RPC does not expose, so recomputing from live stake can disagree (a
  // proposal that barely crossed renders as ~99.9%). The account's verdict wins.
  const thresholdCrossed = proposal.voting || proposal.finalized;

  const stats = useMemo(
    () =>
      computeSupportStats({
        currentSupportLamports: proposal.clusterSupportLamports,
        totalStakedLamports: validatorsStake,
        // Support threshold comes from the on-chain GlobalConfig; falls back to
        // the current mainnet default until the config loads.
        thresholdPercent: supportThresholdPercentFromConfig(configData),
        validatorCount: supportAccounts.length,
        numOfValidators,
        thresholdCrossed,
      }),
    [
      configData,
      numOfValidators,
      proposal.clusterSupportLamports,
      supportAccounts.length,
      thresholdCrossed,
      validatorsStake,
    ],
  );

  // Determine banner state
  const showBanner = stats.progressPercent >= 80 || stats.isThresholdMet;
  const bannerMessage = stats.isThresholdMet
    ? "Support threshold reached! Proposal advancing to next phase."
    : `This proposal is nearing its support threshold. Only ${formatSOL(
      stats.remainingLamports,
    )} SOL needed!`;

  return (
    <div className="glass-card flex h-full flex-col p-6 md:p-6 lg:p-8">
      {/* Header - Mobile: title full width, status left + icon right below */}
      {/* Desktop: title + icon left, status right */}
      <div className="mb-6 flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        {/* Desktop: title + notification button together */}
        <div className="flex items-center gap-3">
          <h3 className="text-lg font-semibold text-foreground">
            Support Phase Progress
          </h3>
          {/* Notification button - hidden on mobile, shown on desktop */}
          <div className="hidden sm:block">
            <NotificationButton
              isVisible={showBanner}
              isThresholdMet={stats.isThresholdMet}
              message={bannerMessage}
            />
          </div>
        </div>
        {/* Mobile: status left, icon right / Desktop: status only */}
        <div className="flex items-center justify-between gap-3 sm:justify-end">
          <PhaseStatusBadge status={proposal.status} />
          {/* Notification button - shown on mobile only */}
          <div className="sm:hidden">
            <NotificationButton
              isVisible={showBanner}
              isThresholdMet={stats.isThresholdMet}
              message={bannerMessage}
            />
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex flex-1 flex-col gap-6 md:flex-row md:items-stretch">
        {/* Donut Chart */}
        <div className="flex flex-1 items-center justify-center">
          <SupportDonut
            progressPercent={stats.progressPercent}
            isThresholdMet={stats.isThresholdMet}
            remainingLamports={stats.remainingLamports}
          />
        </div>

        {/* Stats Grid */}
        <div className="flex flex-1 flex-col gap-3">
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
            {/* Current Support */}
            <StatCard
              label="Current Support"
              value={
                isLoading ? (
                  <div className="my-1 w-14 h-6 animate-pulse bg-white/10 rounded-full" />
                ) : (
                  `${formatSOL(stats.currentSupportLamports)} SOL`
                )
              }
              badge={
                isLoading ? (
                  <div className="my-1 w-10 h-4 animate-pulse bg-white/10 rounded-full" />
                ) : (
                  <StatBadge variant="primary">
                    {stats.supportPercentOfTotal.toFixed(2)}%
                  </StatBadge>
                )
              }
              progressBar={{
                percent: stats.supportPercentOfTotal,
                colorClass: "bg-gradient-to-r from-primary to-emerald-500",
              }}
            />

            {/* Required Threshold */}
            <StatCard
              label="Required Threshold"
              value={
                isLoading ? (
                  <div className="my-1 w-14 h-6 animate-pulse bg-white/10 rounded-full" />
                ) : (
                  `${formatSOL(stats.requiredThresholdLamports)} SOL`
                )
              }
              badge={
                isLoading ? (
                  <div className="my-1 w-10 h-4 animate-pulse bg-white/10 rounded-full" />
                ) : (
                  <StatBadge variant="purple">
                    {stats.thresholdPercent}%
                  </StatBadge>
                )
              }
              secondaryText={
                isLoading ? (
                  <div className="my-1 w-20 h-2 animate-pulse bg-white/10 rounded-full" />
                ) : (
                  `${formatSOL(stats.totalStakedLamports)} total staked`
                )
              }
            />

            {/* Time Remaining Carousel */}
            {supportEndsAt &&
              discussionEndsAt && <TimeRemainingCarousel
                lifecycleStage={proposal.status}
                supportToDiscussionEnd={supportEndsAt}
                discussionToVotingEnd={discussionEndsAt}
                hasEnded={hasEnded}
              />}

            {/* Validator Participation */}
            <StatCard
              label="Validator Participation"
              value={
                isLoading ? (
                  <div className="my-1 w-14 h-6 animate-pulse bg-white/10 rounded-full" />
                ) : (
                  `${stats.participationPercent.toFixed(1) || 0}%`
                )
              }
              badge={
                isLoading ? (
                  <div className="my-1 w-10 h-4 animate-pulse bg-white/10 rounded-full" />
                ) : (
                  <StatBadge variant="primary">
                    {stats.validatorCount} validators
                  </StatBadge>
                )
              }
              secondaryText={
                isLoading ? (
                  <div className="my-1 w-20 h-2 animate-pulse bg-white/10 rounded-full" />
                ) : (
                  `${formatSOL(
                    stats.avgStakePerValidator,
                  )} SOL avg per validator`
                )
              }
            />
          </div>
        </div>
      </div>
    </div>
  );
}
