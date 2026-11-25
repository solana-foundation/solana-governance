"use client";

import { ReactNode } from "react";
import { WalletProvider } from "@/contexts/WalletContext";
import { ConnectorProvider } from "@/components/ConnectorProvider";

export default function AppWalletProvider({
  children,
}: {
  children: ReactNode;
}) {
  return (
    <ConnectorProvider>
      <WalletProvider>{children}</WalletProvider>
    </ConnectorProvider>
  );
}
