import { useGetValidators } from "./useGetValidators";
import { useVotes } from "./useVotes";
import { useQuery } from "@tanstack/react-query";

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
        { for: number; against: number; abstain: number; count: number }
      > = {};
      const votesLatestTimestamp: Record<ValidatorVoteIdentity, number> = {};
      const votesCount: Record<ValidatorVoteIdentity, number> = {};

      for (const vote of votes) {
        for (const validator of validators) {
          if (validator.vote_identity === vote.account.validator.toBase58()) {
            const data = vote.account;
            const vote_identity = validator.vote_identity;

            // compute latest timestamp for this validator and vote
            const { voteTimestamp } = data;
            const parsedVoteTimestamp = voteTimestamp.toNumber() * 1000;
            if (
              !votesLatestTimestamp[vote_identity] ||
              parsedVoteTimestamp > votesLatestTimestamp[vote_identity]
            ) {
              votesLatestTimestamp[vote_identity] = parsedVoteTimestamp;
            }

            // sum votes

            if (!voteSums[vote_identity]) {
              voteSums[vote_identity] = {
                for: 0,
                against: 0,
                abstain: 0,
                count: 0,
              };
            }

            voteSums[vote_identity].for += data.forVotesBp.toNumber();
            voteSums[vote_identity].against += data.againstVotesBp.toNumber();
            voteSums[vote_identity].abstain += data.abstainVotesBp.toNumber();
            voteSums[vote_identity].count += 1;

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
        const avgFor = f / count;
        const avgAgainst = a / count;
        const avgAbstain = ab / count;
        const undecided = 10000 - (avgFor + avgAgainst + avgAbstain);

        result[vote_identity] = {
          yes: avgFor / 100,
          no: avgAgainst / 100,
          abstain: avgAbstain / 100,
          undecided: undecided / 100,
        };
      }

      return { voterSplits: result, votesLatestTimestamp, votesCount };
    },
  });

  return { ...query, isLoading: isLoadingSubqueries || query.isLoading };
};
