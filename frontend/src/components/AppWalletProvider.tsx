"use client";

import { ReactNode, useMemo } from "react";
import {
  ConnectionProvider,
  WalletProvider,
} from "@solana/wallet-adapter-react";
import { WalletModalProvider } from "@solana/wallet-adapter-react-ui";
// import { UnsafeBurnerWalletAdapter } from "@solana/wallet-adapter-wallets";
import { useEndpoint } from "@/contexts/EndpointContext";
import { WalletSessionProvider } from "@/contexts/WalletSessionContext";

// Default styles that can be overridden by your app
import "@solana/wallet-adapter-react-ui/styles.css";

export default function AppWalletProvider({
  children,
}: {
  children: ReactNode;
}) {
  const { endpointUrl } = useEndpoint();

  const wallets = useMemo(
    () => [
      // manually add any legacy wallet adapters here
      // new UnsafeBurnerWalletAdapter(),
    ],
    []
  );

  return (
    <ConnectionProvider endpoint={endpointUrl}>
      <WalletProvider wallets={wallets} autoConnect>
        <WalletModalProvider>
          <WalletSessionProvider>{children}</WalletSessionProvider>
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}
