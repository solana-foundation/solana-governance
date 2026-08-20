import { useQuery } from "@tanstack/react-query";
import { useGetValidators } from "./useGetValidators";
import { Validator, OldVoteAccountData } from "@/types";
import { useVoteAccounts } from "./useVoteAccounts";

export type VoteValidatorEntry = {
  votePDA: string;
  voteAccount: OldVoteAccountData;
  validator: Validator | undefined;
};

type VotePublicKey = string;
type VoteMap = Record<VotePublicKey, VoteValidatorEntry>; // key = vote public key

type ValidatorIdentity = string;
type ValidatorMap = Record<ValidatorIdentity, VoteValidatorEntry[]>; // key = validator.identity

/**
 * Hashmap by validator.identity key
 */
export interface VoteAccountsWithValidators {
  voteMap: VoteMap;
  validatorMap: ValidatorMap;
}

export const useVoteAccountsWithValidators = () => {
  const { data: validators, isLoading: isLoadingValidators } =
    useGetValidators();

  const { data: votes, isLoading: isLoadingVotes } = useVoteAccounts();

  const enabled =
    !!validators && !isLoadingValidators && !!votes && !isLoadingVotes;

  const isLoadingSubqueries = isLoadingVotes || isLoadingValidators;

  const query = useQuery({
    queryKey: ["vote-accounts-with-validators"],
    queryFn: async (): Promise<VoteAccountsWithValidators> => {
      if (validators === undefined)
        throw new Error("Unable to get validators info");
      if (votes === undefined) throw new Error("Unable to get votes info");

      if (validators.length === 0) throw new Error("No validators found");
      if (votes.length === 0) throw new Error("No votes found");

      const voteMap: VoteMap = {};
      const validatorMap: ValidatorMap = {};

      for (const vote of votes) {
        const validator = validators.find(
          (v) => vote.identity === v.vote_identity,
        );
        const votePk = vote.voteAccount;
        if (validator) {
          const entry: VoteValidatorEntry = {
            votePDA: vote.voteAccount,
            // enrich vote account info with matched validator data
            voteAccount: {
              ...vote,
              identity: undefined,
              name: validator.name,
              credits: validator.credits,
              lastVote: validator.last_vote,
              activeStake: validator.activated_stake,
              epochCredits: validator.epoch_credits,
            },
            validator,
          };

          voteMap[votePk] = entry;

          const valId = validator.vote_identity;
          if (!validatorMap[valId]) {
            validatorMap[valId] = [];
          }
          validatorMap[valId].push(entry);
        } else {
          // console.warn("no validator found");
          const entry: VoteValidatorEntry = {
            votePDA: vote.voteAccount,
            voteAccount: {
              ...vote,
              identity: vote.identity,
            },
            validator: undefined,
          };
          voteMap[votePk] = entry;
        }
      }

      return {
        voteMap,
        validatorMap,
      };
    },
    staleTime: 1000 * 60 * 10, // 10 minutes
    enabled,
  });

  return { ...query, isLoading: isLoadingSubqueries };
};
