"use client";

import { Fragment } from "react";
import type { ProposalRecord } from "@/types";
import { PHASES } from "./constants";
import { PhaseNode } from "./PhaseNode";
import { ConnectorLine } from "./ConnectorLine";
import { PhaseDetail } from "./PhaseDetail";
import { DesktopPhaseTimeline } from "./DesktopPhaseTimeline";
import { MobilePhaseTimeline } from "./MobilePhaseTimeline";
import { useGovernanceConfigContext } from "@/contexts/GovernanceConfigContext";
import { useEpochInfo, useEpochToDate } from "@/hooks";
import {
  epochConstantsFromGovernanceConfig,
  getProposalPhaseEpochs,
} from "@/lib/proposals";
import { calculateVotingEndsIn } from "@/helpers";

interface PhaseTimelineProps {
  proposal: ProposalRecord | undefined;
  isLoading: boolean;
}

function getActivePhaseEndEpoch(
  phase: ProposalRecord["status"] | undefined,
  supportEndEpoch: number | undefined,
  discussionEndEpoch: number | undefined,
  votingEndEpoch: number | undefined
) {
  switch (phase) {
    case "supporting":
      return supportEndEpoch;
    case "discussion":
      return discussionEndEpoch;
    case "voting":
    case "finalized":
      return votingEndEpoch;
    default:
      return undefined;
  }
}

export default function PhaseTimeline({
  proposal,
  isLoading,
}: PhaseTimelineProps) {
  const governanceConfigQuery = useGovernanceConfigContext();
  const { data: epochData } = useEpochInfo();
  const currentPhase = proposal?.status;
  const epochConstants = governanceConfigQuery.data
    ? epochConstantsFromGovernanceConfig(governanceConfigQuery.data)
    : undefined;
  const phaseEpochs = epochConstants && proposal
    ? getProposalPhaseEpochs(proposal.creationEpoch, epochConstants)
    : undefined;
  const votingEndEpoch = phaseEpochs
    ? phaseEpochs.snapshotEpoch + (epochConstants?.VOTING_EPOCHS ?? 0)
    : undefined;

  const activePhaseEndEpoch = getActivePhaseEndEpoch(
    currentPhase,
    phaseEpochs?.supportEndEpoch,
    phaseEpochs?.discussionEndEpoch,
    votingEndEpoch
  );

  const { data: activePhaseEndsAt } = useEpochToDate(activePhaseEndEpoch);
  const currentEpoch = epochData?.epochInfo.epoch;
  const remainingTime = activePhaseEndsAt
    ? calculateVotingEndsIn(activePhaseEndsAt.toISOString()) ?? "-"
    : "-";

  if (isLoading) return <PhaseTimelineSkeleton />;
  if (!proposal || !currentPhase) return <div>No proposal data...</div>;

  return (
    <div className="glass-card space-y-6 p-8 overflow-hidden">
      <h4 className="h4 font-semibold pb-5">Phase Timeline</h4>

      {/* Desktop: Full horizontal timeline (lg and up) */}
      <div className="hidden lg:block">
        <DesktopPhaseTimeline proposal={proposal} currentPhase={currentPhase} />
      </div>

      <div className="lg:hidden -mx-8">
        <MobilePhaseTimeline proposal={proposal} currentPhase={currentPhase} />
      </div>

      <PhaseDetail
        currentPhase={currentPhase}
        status={proposal.status}
        remainingTime={remainingTime}
        currentEpoch={currentEpoch}
        activePhaseEndEpoch={activePhaseEndEpoch}
      />
    </div>
  );
}

const PhaseTimelineSkeleton = () => {
  return (
    <div className="glass-card space-y-6 p-6">
      <h4 className="h4 font-semibold">Phase Timeline</h4>

      <div className="relative flex w-full justify-center px-2 sm:px-4 md:px-6 lg:px-8 pb-8">
        <div className="flex w-fit max-w-4xl items-center justify-center gap-0 mx-auto">
          {PHASES.map((phase, index) => {
            return (
              <Fragment key={phase.key}>
                <PhaseNode phase={phase} state="upcoming" isLoading={true} />

                {index !== PHASES.length - 1 && (
                  <ConnectorLine
                    variant="upcoming"
                    animate={false}
                    isLoading={true}
                  />
                )}
              </Fragment>
            );
          })}
        </div>
      </div>

      <PhaseDetail
        currentPhase="supporting"
        status="supporting"
        isLoading={true}
      />
    </div>
  );
};
