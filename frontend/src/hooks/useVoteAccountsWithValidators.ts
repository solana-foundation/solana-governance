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

      // A Vote account records the validator's identity, while StakeWiz keys on
      // the vote account address. Index both so a lookup succeeds with either.
      const validatorByKey: Record<string, Validator> = {};
      for (const v of validators) {
        validatorByKey[v.vote_identity] = v;
        if (v.identity) {
          validatorByKey[v.identity] = v;
        }
      }

      let unmatched = 0;

      for (const vote of votes) {
        const identity = vote.identity?.toBase58();
        const validator = identity ? validatorByKey[identity] : undefined;
        const votePk = vote.voteAccount.toBase58();
        if (validator) {
          const entry: VoteValidatorEntry = {
            votePDA: vote.voteAccount.toBase58(),
            // enrich vote account info with matched validator data
            voteAccount: {
              ...vote,
              // Cleared so the identity column falls back to the name.
              identity: undefined,
              name: validator.name,
              commission: validator.commission,
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
          unmatched += 1;
          const entry: VoteValidatorEntry = {
            votePDA: vote.voteAccount.toBase58(),
            voteAccount: {
              ...vote,
              identity: vote.identity,
            },
            validator: undefined,
          };
          voteMap[votePk] = entry;
        }
      }

      if (unmatched > 0) {
        console.warn(
          `No validator metadata for ${unmatched} of ${votes.length} vote records; those rows report unknown rather than zero.`,
        );
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
