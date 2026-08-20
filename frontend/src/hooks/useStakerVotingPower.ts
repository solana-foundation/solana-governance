import { useMemo } from "react";
import { useWalletStakeAccounts } from "./useWalletStakeAccounts";

export function useStakerVotingPower(
  userPubKey: string | undefined,
  enabled = true
) {
  const { data: stakeAccounts, isLoading } = useWalletStakeAccounts(
    userPubKey,
    enabled
  );

  const votingPower = useMemo(
    () => stakeAccounts?.reduce((acc, curr) => acc + curr.activeStake, 0n),
    [stakeAccounts]
  );

  return { votingPower, isLoading: isLoading };
}
