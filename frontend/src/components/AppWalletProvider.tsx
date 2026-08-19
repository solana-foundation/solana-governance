"use client";

import { ReactNode } from "react";
import { WalletSessionProvider } from "@/contexts/WalletSessionContext";
import { ConnectorProvider } from "@/components/ConnectorProvider";

export default function AppWalletProvider({
  children,
}: {
  children: ReactNode;
}) {
  return (
    <ConnectorProvider>
      <WalletSessionProvider>{children}</WalletSessionProvider>
    </ConnectorProvider>
  );
}
