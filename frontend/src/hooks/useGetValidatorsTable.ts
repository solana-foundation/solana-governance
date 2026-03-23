import { useQuery } from "@tanstack/react-query";
import { useValidatorsVoterSplits } from "./useValidatorsVoterSplits";
import { Validator } from "@/types";
import { roundDecimals } from "@/lib/helpers";
import { useGetValidators } from "./useGetValidators";

export type SortBy = "weight" | "name" | "percentage" | "date";

export interface ValidatorsTableRow extends Validator {
  voterSplits: {
    yes: string | number;
    no: string | number;
    abstain: string | number;
    undecided: string | number;
  };
  percentage: number;
  voteDate: string;
}

export const useGetValidatorsTable = (sortBy: SortBy) => {
  const { data: validators, isLoading: isLoadingValidators } =
    useGetValidators();

  const { data, isLoading: isLoadingVoterSplits } = useValidatorsVoterSplits();

  const voterSplits = data?.voterSplits;
  const votesLatestTimestamp = data?.votesLatestTimestamp;

  const validatorsReady =
    !isLoadingValidators &&
    validators &&
    validators.length > 0 &&
    !isLoadingVoterSplits;
  const enabled = validatorsReady;

  return useQuery({
    staleTime: 1000 * 120, // 2 minutes
    queryKey: ["validatorsTable", sortBy],
    enabled,
    queryFn: async (): Promise<ValidatorsTableRow[] | null> => {
      if (!validators) return null;

      let sortByProp: keyof (typeof validators)[0] | "voteDate" | "percentage" =
        "activated_stake";
      if (sortBy === "weight") sortByProp = "activated_stake";
      else if (sortBy === "name") sortByProp = "name";
      else if (sortBy === "percentage") sortByProp = "percentage";
      else if (sortBy === "date") sortByProp = "voteDate";

      const totalStake = validators.reduce((acc, curr) => {
        return (acc += curr.activated_stake);
      }, 0);

      return (
        validators?.map((v) => ({
          ...v,
          percentage: roundDecimals(
            ((v.activated_stake * 100) / totalStake).toString(),
          ),
          voterSplits: {
            yes: voterSplits?.[v.vote_identity]
              ? voterSplits[v.vote_identity]?.yes
              : "-",
            no: voterSplits?.[v.vote_identity]
              ? voterSplits[v.vote_identity]?.no
              : "-",
            abstain: voterSplits?.[v.vote_identity]
              ? voterSplits[v.vote_identity]?.abstain
              : "-",
            undecided: voterSplits?.[v.vote_identity]
              ? voterSplits[v.vote_identity]?.undecided
              : "-",
          },
          voteDate: votesLatestTimestamp?.[v.vote_identity]
            ? new Date(votesLatestTimestamp?.[v.vote_identity]).toLocaleString()
            : "-",
        })) || []
      ).sort((a, b) => +b[sortByProp] - +a[sortByProp]);
    },
  });
};

// Proposal Account:  2GbFTkSmBkxWXVWa252XDTVptU4hqhP5BHvNkMKa3vL1

// Validator 1 Public Key:  AHYic562KhgtAEkb1rSesqS87dFYRcfXb4WwWus3Zc9C
// Validator 1 Vote Account:  EJU89DJtDsTb6LyYLiaU4urVxQxFGhbKshorjB3owHnr

// Validator 2 Public Key:  5WYbFiL3p2vDmFq299Lf236zkUb7VfJafXuaoS5YfV1p
// Validator 2 Vote Account:  CjnKBR9XZv2a3JgUA2GU6aJJVS1tJ5ytZcfMCCwJydBw

// Validator 3 Public Key:  E5bjdQKxNBLo6DkyDFzD3xCEyF7ZYiXq5YFHuKbo7APu
// Validator 3 Vote Account:  6yjgnHT1u1pvApccRxTybA7WLSKu1cYtS8BfmyQL9F4b
