"use client";

import { GovernanceEmptyState } from "./shared/GovernanceEmptyState";
import { GovernanceDashboardLayout } from "@/components/governance/GovernanceDashboardLayout";
import { useWalletSession } from "@/contexts/WalletSessionContext";

export function GovernanceDashboard() {
  const { connected, publicKey, openWalletModal } = useWalletSession();

  const handleConnectWallet = () => {
    openWalletModal();
  };

  // If not connected, only show empty state
  if (!connected || !publicKey) {
    return (
      <div className="space-y-6">
        <GovernanceEmptyState onAction={handleConnectWallet} />
      </div>
    );
  }

  return <GovernanceDashboardLayout userPubKey={publicKey} />;
}
