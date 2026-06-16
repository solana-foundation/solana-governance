import { useState, useEffect } from "react";
import { WalletRole, type ViewType } from "@/types";
import {
  determineWalletRole,
  getDefaultView,
} from "@/lib/governance/role-detection";
import { useVoterWalletSummary } from "./useVoterWalletSummary";
import { useChainVoteAccount } from "./useChainVoteAccount";

interface UseWalletRoleReturn {
  walletRole: WalletRole;
  selectedView: ViewType | undefined;
  setSelectedView: (view: ViewType | undefined) => void;
  isLoading: boolean;
}

export function useWalletRole(
  userPubKey: string | undefined
): UseWalletRoleReturn {
  const [walletRole, setWalletRole] = useState<WalletRole>(WalletRole.STAKER);
  const [selectedView, setSelectedView] = useState<ViewType | undefined>(
    "staker"
  );

  const { data, isLoading: isLoadingWalletSummary } =
    useVoterWalletSummary(userPubKey);

  const { data: chainVoteAccount, isLoading: isLoadingChainVoteAccount } =
    useChainVoteAccount(userPubKey);

  const isLoading = isLoadingWalletSummary || isLoadingChainVoteAccount;

  useEffect(() => {
    if (isLoading || data === undefined) return;

    const role = determineWalletRole(
      data.stake_accounts,
      data.vote_accounts,
      chainVoteAccount
    );

    setWalletRole(role);
    setSelectedView(getDefaultView(role));
  }, [isLoading, data, chainVoteAccount]);

  return { walletRole, selectedView, setSelectedView, isLoading };
}
