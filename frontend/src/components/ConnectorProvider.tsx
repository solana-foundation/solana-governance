"use client";

import { AppProvider, getDefaultConfig, getDefaultMobileConfig } from "@solana/connector/react";
import { useMemo, type ReactNode } from "react";
import { useEndpoint } from "@/contexts/EndpointContext";
import { getRpcProxyUrl } from "@/lib/getRpcProxyUrl";

function appUrl() {
  return typeof window === "undefined" ? "http://localhost:3000" : window.location.origin;
}

export function ConnectorProvider({ children }: { children: ReactNode }) {
  const { network } = useEndpoint();
  const connectorConfig = useMemo(
    () =>
      getDefaultConfig({
        appName: "Solana Governance",
        appUrl: appUrl(),
        autoConnect: true,
        clusters: [
          { id: "solana:mainnet", label: "Mainnet", url: getRpcProxyUrl("mainnet") },
          { id: "solana:devnet", label: "Devnet", url: getRpcProxyUrl("devnet") },
          { id: "solana:testnet", label: "Testnet", url: getRpcProxyUrl("testnet") },
        ],
        network,
        persistClusterSelection: false,
        walletConnect: false,
      }),
    [network],
  );
  const mobile = useMemo(
    () => getDefaultMobileConfig({ appName: "Solana Governance", appUrl: appUrl() }),
    [],
  );

  return <AppProvider connectorConfig={connectorConfig} mobile={mobile}>{children}</AppProvider>;
}
