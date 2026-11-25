import { useGetValidators } from "./useGetValidators";
import { useVotes } from "./useVotes";
import { useQuery } from "@tanstack/react-query";
import { bigintToSafeNumber } from "@/helpers/bigint";

type ValidatorVoteIdentity = string;

export type VoteType = "yes" | "no" | "abstain" | "undecided";

export type VoteSplitAnalytics = Record<VoteType, number>;

export const useValidatorsVoterSplits = () => {
  const { data: validators, isLoading: isLoadingValidators } =
    useGetValidators();
  const { data: votes, isLoading: isLoadindVotes } = useVotes();

  const validatorsReady =
    !isLoadingValidators && validators && validators.length > 0;
  const votesReady = !isLoadindVotes && votes && votes.length > 0;
  const enabled = validatorsReady && votesReady;

  const isLoadingSubqueries = isLoadindVotes || isLoadingValidators;

  const query = useQuery({
    staleTime: 1000 * 120, // 2 minutes
    queryKey: ["validatorsVoterSplits"],
    enabled,
    queryFn: async () => {
      if (!validators || !votes) return null;

      // we need to compute each validators Voter Split (average yes/no/abstain/undecided of ALL votes)
      const voteSums: Record<
        ValidatorVoteIdentity,
        { for: bigint; against: bigint; abstain: bigint; count: bigint }
      > = {};
      const votesLatestTimestamp: Record<ValidatorVoteIdentity, number> = {};
      const votesCount: Record<ValidatorVoteIdentity, number> = {};

      for (const vote of votes) {
        for (const validator of validators) {
          if (validator.vote_identity === vote.data.validator) {
            const data = vote.data;
            const vote_identity = validator.vote_identity;

            // compute latest timestamp for this validator and vote
            const { voteTimestamp } = data;
            const parsedVoteTimestamp = bigintToSafeNumber(voteTimestamp, "vote timestamp") * 1000;
            if (
              !votesLatestTimestamp[vote_identity] ||
              parsedVoteTimestamp > votesLatestTimestamp[vote_identity]
            ) {
              votesLatestTimestamp[vote_identity] = parsedVoteTimestamp;
            }

            // sum votes

            if (!voteSums[vote_identity]) {
              voteSums[vote_identity] = {
                for: 0n,
                against: 0n,
                abstain: 0n,
                count: 0n,
              };
            }

            voteSums[vote_identity].for += data.forVotesBp;
            voteSums[vote_identity].against += data.againstVotesBp;
            voteSums[vote_identity].abstain += data.abstainVotesBp;
            voteSums[vote_identity].count += 1n;

            if (!votesCount[vote_identity]) {
              votesCount[vote_identity] = 0;
            }
            votesCount[vote_identity]++;

            break;
          }
        }
        // if (!matchedValidator)
        // console.error("found no validator for this vote", vote);
      }

      const result: Record<string, VoteSplitAnalytics> = {};

      for (const [
        vote_identity,
        { for: f, against: a, abstain: ab, count },
      ] of Object.entries(voteSums)) {
        // Each sum is at most 10,000 bps per vote, so the averages are
        // bounded percentages that a number holds exactly enough to display.
        const averagePercent = (sumBp: bigint) =>
          Number(sumBp) / Number(count) / 100;
        const yes = averagePercent(f);
        const no = averagePercent(a);
        const abstain = averagePercent(ab);

        result[vote_identity] = {
          yes,
          no,
          abstain,
          undecided: 100 - (yes + no + abstain),
        };
      }

      return { voterSplits: result, votesLatestTimestamp, votesCount };
    },
  });

  return { ...query, isLoading: isLoadingSubqueries || query.isLoading };
};
