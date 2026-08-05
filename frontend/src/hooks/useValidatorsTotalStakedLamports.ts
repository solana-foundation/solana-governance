import { useMemo } from "react";
import { useGetValidators } from "./useGetValidators";

/**
 * Total activated stake across all validators, or `undefined` when it is not
 * known — the query is in flight, or it failed.
 *
 * Deliberately not `0` for the unknown case: callers divide by this to produce
 * percentages, and a zero denominator renders as "0%", which is
 * indistinguishable from a real result.
 */
export const useValidatorsTotalStakedLamports = () => {
  const { data: validators, isLoading, isError, error } = useGetValidators();

  const totalStakedLamports = useMemo(() => {
    if (!validators) {
      return undefined;
    }

    return validators.reduce(
      (sum, validator) => sum + (validator.activated_stake || 0),
      0
    );
  }, [validators]);

  return {
    totalStakedLamports,
    isLoading,
    isError,
    error,
  };
};
