"use client";

import { createContext, useContext, useMemo, type ReactNode } from "react";
import {
  useAnchorWallet,
  useConnection,
  useWallet,
  type AnchorWallet,
  type WalletContextState,
} from "@solana/wallet-adapter-react";
import { useWalletModal } from "@solana/wallet-adapter-react-ui";
import type { Connection } from "@solana/web3.js";

export type WalletSession = WalletContextState & {
  anchorWallet: AnchorWallet | undefined;
  connection: Connection;
  openWalletModal: () => void;
};

const WalletSessionContext = createContext<WalletSession | null>(null);

export function WalletSessionProvider({ children }: { children: ReactNode }) {
  const wallet = useWallet();
  const anchorWallet = useAnchorWallet();
  const { connection } = useConnection();
  const { setVisible } = useWalletModal();

  const value = useMemo<WalletSession>(
    () => ({
      ...wallet,
      anchorWallet,
      connection,
      openWalletModal: () => setVisible(true),
    }),
    [anchorWallet, connection, setVisible, wallet],
  );

  return (
    <WalletSessionContext.Provider value={value}>
      {children}
    </WalletSessionContext.Provider>
  );
}

export function useWalletSession(): WalletSession {
  const session = useContext(WalletSessionContext);
  if (!session) {
    throw new Error("useWalletSession must be used inside AppWalletProvider");
  }
  return session;
}
