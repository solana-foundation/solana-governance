import { useMemo } from "react";
import { useProposals } from "./useProposals";
import { useGetValidators } from "./useGetValidators";

export type Stat = {
  id: string;
  label: string;
  /** `undefined` when the value is not known — rendered as "-", not as 0. */
  value: number | undefined;
};

export interface Stats {
  stats: Stat[];
  isLoading: boolean;
}

export const useProposalOverviewStats = (): Stats => {
  const { data: proposals, isLoading: isLoadingProposals } = useProposals();

  const { data: validators, isLoading: isLoadingValidators } =
    useGetValidators();

  const isLoading = isLoadingProposals || isLoadingValidators;

  const activeProposals = useMemo(
    () => proposals?.filter((p) => p.status === "voting").length || 0,
    [proposals]
  );
  const supportingProposals = useMemo(
    () => proposals?.filter((p) => p.status === "supporting").length || 0,
    [proposals]
  );

  // Undefined rather than 0 when the validator query has not resolved, so the
  // card can show "-" instead of a count that looks real.
  const numOfValidators = useMemo(() => validators?.length, [validators]);

  return {
    isLoading,
    stats: [
      {
        id: "active-proposals",
        label: "Active Proposals",
        value: activeProposals,
      },
      {
        id: "support-phase-proposals",
        label: "Support Phase Proposals",
        value: supportingProposals,
      },
      {
        id: "number-of-validators",
        label: "Number of Validators",
        value: numOfValidators,
      },
      // {
      //   id: "number-of-stakers",
      //   label: "Number of Stakers",
      //   value: 123232232,
      // },
    ],
  };
};
