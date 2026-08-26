"use client";

import { GovernanceEmptyState } from "./shared/GovernanceEmptyState";
import { GovernanceDashboardLayout } from "@/components/governance/GovernanceDashboardLayout";
import { useConnector } from "@solana/connector/react";
import { useWallet } from "@/contexts/WalletContext";

export function GovernanceDashboard() {
  const { account, isConnected: connected } = useConnector();
  const publicKey = account ?? undefined;
  const { openModal: openWalletModal } = useWallet();

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
