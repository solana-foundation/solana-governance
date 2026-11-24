import { useMemo } from "react";
import { useValidatorVoteAccounts } from "./useValidatorVoteAccounts";

export function useValidatorVotingPower(
  userPubKey: string | undefined,
  enabled = true
) {
  const { data: voteAccount, isLoading } = useValidatorVoteAccounts(
    userPubKey,
    enabled
  );

  const votingPower = useMemo(
    () => voteAccount?.activeStake ?? 0n,
    [voteAccount]
  );

  return { votingPower, isLoading: isLoading };
}
