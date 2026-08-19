const REQUIRED_QUORUM_PCT = 0.6;
import { VotingRateChartLoadingSkeleton } from "./ui/VotingRateChartLoadingSkeleton";

export const QuorumLoadingSkeleton = () => {
  return (
    <div className="grid md:grid-cols-12 gap-14">
      {/* Left Column - Quorum */}
      <div className="md:border-b-0 border-b col-span-5">
        <div className="text-xs font-medium tracking-wider text-dao-text-secondary uppercase mb-1">
          QUORUM
        </div>

        <div className="flex items-baseline gap-3 mb-1">
          <div className="text-xl font-bold text-dao-text-primary">
            {REQUIRED_QUORUM_PCT * 100}%
          </div>
          <div className="h-3 w-24 bg-gray animate-pulse rounded-full" />
        </div>

        {/* <p className="text-sm text-dao-text-muted leading-relaxed md:max-w-80">
          Quorum in DAO voting is the minimum percentage of members or tokens
          required to validate a proposal.
        </p> */}
        <div className="md:max-w-80 mt-3 mb-1">
          <div className="h-3 w-60 bg-gray animate-pulse rounded-full" />
          <div className="h-3 w-70 bg-gray animate-pulse rounded-full mt-2.5" />
          <div className="h-3 w-32 bg-gray animate-pulse rounded-full mt-2.5" />
        </div>
      </div>

      {/* Right Column - Voting Rate */}
      <div className="flex flex-col md:mt-2 col-span-7">
        <div className="text-xs font-medium tracking-wider text-dao-text-secondary uppercase mb-3">
          VOTING RATE
        </div>

        {/* Stacked Bar Chart */}
        <VotingRateChartLoadingSkeleton />
      </div>
    </div>
  );
};
