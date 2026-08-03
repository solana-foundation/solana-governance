import { useMemo } from "react";
import { PublicKey } from "@solana/web3.js";
import { TopSupporterRecord, Validator } from "@/types";
import { accentColors } from "@/types/topVoters";
import { buildSupportFilters, useSupportAccounts } from "./useSupportAccounts";
import { useGetValidators } from "./useGetValidators";

const getColorFromString = (str: string): string => {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    const char = str.charCodeAt(i);
    hash = (hash << 5) - hash + char;
    hash = hash & hash;
  }
  const index = Math.abs(hash) % accentColors.length;
  return accentColors[index];
};

/**
 * Fetches the support accounts for a proposal and joins them with validator
 * metadata (name, image, stake) so they can be rendered like top voters.
 */
export const useProposalSupporters = (
  proposalPublicKey: PublicKey | undefined,
) => {
  const supportFilters = buildSupportFilters(
    proposalPublicKey?.toBase58(),
    null,
  );

  const { data: supportAccounts, isLoading: isLoadingSupportAccounts } =
    useSupportAccounts(supportFilters, supportFilters.length > 0);

  const { data: validators, isLoading: isLoadingValidators } =
    useGetValidators();

  const supporters = useMemo((): TopSupporterRecord[] => {
    if (!supportAccounts) {
      return [];
    }

    // Support accounts store the validator node identity (the signer);
    // StakeWiz has both identity and vote_identity, so map by both.
    const validatorMap: Record<string, Validator> = {};
    if (validators) {
      for (const v of validators) {
        validatorMap[v.vote_identity] = v;
        if (v.identity) {
          validatorMap[v.identity] = v;
        }
      }
    }

    const totalStakedLamports = validators
      ? validators.reduce((sum, v) => sum + (v.activated_stake || 0), 0)
      : 0;

    return supportAccounts.map((support) => {
      const identity = support.validator.toBase58();
      const validator = validatorMap[identity];
      const validatorName = validator?.name || "Unknown Validator";
      const stakedLamports = validator?.activated_stake || 0;
      const stakePercentage =
        totalStakedLamports > 0 && stakedLamports > 0
          ? (stakedLamports / totalStakedLamports) * 100
          : 0;

      return {
        id: support.publicKey.toBase58(),
        validatorName,
        validatorIdentity: identity,
        validatorImage: validator?.image ?? null,
        stakedLamports,
        stakePercentage,
        accentColor: getColorFromString(validatorName),
      };
    });
  }, [supportAccounts, validators]);

  return {
    data: supporters,
    isLoading: isLoadingSupportAccounts || isLoadingValidators,
  };
};
