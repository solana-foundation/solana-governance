"use client";

import { useConnector } from "@solana/connector/react";
import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { AppButton } from "@/components/ui/AppButton";

export type WalletSession = {
  connected: boolean;
  connecting: boolean;
  publicKey: string | undefined;
  disconnect: () => Promise<void>;
  openWalletModal: () => void;
};

const WalletSessionContext = createContext<WalletSession | null>(null);

export function WalletSessionProvider({ children }: { children: ReactNode }) {
  const [modalOpen, setModalOpen] = useState(false);
  const { account, connectWallet, connectors, disconnectWallet, isConnected, isConnecting } = useConnector();
  const openWalletModal = useCallback(() => setModalOpen(true), []);
  const value = useMemo<WalletSession>(
    () => ({
      connected: isConnected,
      connecting: isConnecting,
      publicKey: account ?? undefined,
      disconnect: disconnectWallet,
      openWalletModal,
    }),
    [account, disconnectWallet, isConnected, isConnecting, openWalletModal],
  );

  return (
    <WalletSessionContext.Provider value={value}>
      {children}
      <Dialog open={modalOpen} onOpenChange={setModalOpen}>
        <DialogContent className="app-modal-content">
          <DialogHeader><DialogTitle>Connect wallet</DialogTitle></DialogHeader>
          <div className="space-y-2">
            {connectors.filter((connector) => connector.ready).map((connector) => (
              <AppButton
                key={connector.id}
                className="w-full justify-start"
                disabled={isConnecting}
                onClick={async () => {
                  await connectWallet(connector.id);
                  setModalOpen(false);
                }}
              >
                {connector.name}
              </AppButton>
            ))}
          </div>
        </DialogContent>
      </Dialog>
    </WalletSessionContext.Provider>
  );
}

export function useWalletSession(): WalletSession {
  const session = useContext(WalletSessionContext);
  if (!session) throw new Error("useWalletSession must be used inside AppWalletProvider");
  return session;
}
