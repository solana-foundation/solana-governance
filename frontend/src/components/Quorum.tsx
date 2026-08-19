const REQUIRED_QUORUM_PCT = 0.6;
import { VotingRateChart } from "./ui";
import { QuorumLoadingSkeleton } from "./QuorumLoadingSkeleton";

export const Quorum = () => {
  const data = undefined;
  const isLoading = false;

  if (data === undefined || isLoading) {
    return <QuorumLoadingSkeleton />;
  }

  const {
    forVotesPercentage,
    againstVotesPercentage,
    abstainVotesPercentage,
    requiredQuorum,
  } = data;

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
          <div className="text-dao-text-secondary text-sm">{data} SOL</div>
        </div>

        <p className="text-sm text-dao-text-muted leading-relaxed md:max-w-80">
          Quorum in DAO voting is the minimum percentage of members or tokens
          required to validate a proposal.
        </p>
      </div>

      {/* Right Column - Voting Rate */}
      <div className="flex flex-col md:mt-2 col-span-7">
        <div className="text-xs font-medium tracking-wider text-dao-text-secondary uppercase mb-3">
          VOTING RATE
        </div>

        {/* Stacked Bar Chart */}
        <VotingRateChart
          yesPercentage={forVotesPercentage}
          noPercentage={againstVotesPercentage}
          abstainPercentage={abstainVotesPercentage}
          requiredAmount={requiredQuorum}
          currentPosition={REQUIRED_QUORUM_PCT * 100}
        />
      </div>
    </div>
  );
};
