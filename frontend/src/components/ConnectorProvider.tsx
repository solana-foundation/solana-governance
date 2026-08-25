"use client";

import { AppProvider, getDefaultConfig, getDefaultMobileConfig } from "@solana/connector/react";
import { useMemo, type ReactNode } from "react";
import { useEndpoint, RPC_URLS } from "@/contexts/EndpointContext";

function appUrl() {
  return typeof window === "undefined" ? "http://localhost:3000" : window.location.origin;
}

export function ConnectorProvider({ children }: { children: ReactNode }) {
  const { endpointUrl, endpointType } = useEndpoint();
  const connectorConfig = useMemo(
    () =>
      getDefaultConfig({
        appName: "Solana Governance",
        appUrl: appUrl(),
        autoConnect: true,
        clusters: [
          { id: "solana:mainnet", label: "Mainnet", url: RPC_URLS.mainnet },
          { id: "solana:devnet", label: "Devnet", url: RPC_URLS.devnet },
          { id: "solana:testnet", label: "Testnet", url: RPC_URLS.testnet },
        ],
        customClusters: [{ id: "solana:custom", label: "Custom", url: endpointUrl }],
        network: endpointType === "custom" ? undefined : endpointType,
        persistClusterSelection: false,
        walletConnect: true,
      }),
    [endpointType, endpointUrl],
  );
  const mobile = useMemo(
    () => getDefaultMobileConfig({ appName: "Solana Governance", appUrl: appUrl() }),
    [],
  );

  return <AppProvider connectorConfig={connectorConfig} mobile={mobile}>{children}</AppProvider>;
}
