import { Fragment } from "react";
import type { ProposalRecord } from "@/types";
import { PHASES } from "./constants";
import { PhaseNode } from "./PhaseNode";
import { ConnectorLine } from "./ConnectorLine";
import { PhaseDetail } from "./PhaseDetail";
import { DesktopPhaseTimeline } from "./DesktopPhaseTimeline";
import { MobilePhaseTimeline } from "./MobilePhaseTimeline";

interface PhaseTimelineProps {
  proposal: ProposalRecord | undefined;
  isLoading: boolean;
}

export default function PhaseTimeline({
  proposal,
  isLoading,
}: PhaseTimelineProps) {
  if (isLoading) return <PhaseTimelineSkeleton />;
  if (!proposal) return <div>No proposal data...</div>;

  const currentPhase = proposal.status;

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

      <PhaseDetail currentPhase={currentPhase} status={proposal.status} />
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
