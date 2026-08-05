import { useMemo } from "react";
import { PublicKey } from "@solana/web3.js";
import { TopSupporterRecord, Validator } from "@/types";
import { accentColors } from "@/types/topVoters";
import { buildSupportFilters, useSupportAccounts } from "./useSupportAccounts";
import { useGetValidators } from "./useGetValidators";
import { useValidatorsTotalStakedLamports } from "./useValidatorsTotalStakedLamports";

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

  // Same query as above (react-query dedupes), but the denominator lives in one
  // place so this pane cannot disagree with the rest of the UI about it.
  const { totalStakedLamports } = useValidatorsTotalStakedLamports();

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

    return supportAccounts.map((support) => {
      const identity = support.validator.toBase58();
      const validator = validatorMap[identity];
      const validatorName = validator?.name || "Unknown Validator";
      const stakedLamports = validator?.activated_stake;
      const stakePercentage =
        totalStakedLamports && stakedLamports !== undefined
          ? (stakedLamports / totalStakedLamports) * 100
          : undefined;

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
  }, [supportAccounts, totalStakedLamports, validators]);

  return {
    data: supporters,
    isLoading: isLoadingSupportAccounts || isLoadingValidators,
  };
};
