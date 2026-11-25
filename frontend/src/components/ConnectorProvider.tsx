"use client";

import {
  AppProvider,
  getDefaultConfig,
  getDefaultMobileConfig,
  useCluster,
} from "@solana/connector/react";
import { useEffect, useMemo, type ReactNode } from "react";
import { useEndpoint } from "@/contexts/EndpointContext";
import { getRpcProxyUrl } from "@/lib/getRpcProxyUrl";

function appUrl() {
  return typeof window === "undefined" ? "http://localhost:3000" : window.location.origin;
}

/**
 * Moves the connector to the app's network whenever it changes. The connector
 * reads its configuration once, so without this a wallet keeps being asked to
 * sign for the cluster the page first loaded with.
 */
function ConnectorClusterSync() {
  const { network } = useEndpoint();
  const { cluster, setCluster } = useCluster();
  const clusterId = cluster?.id;

  useEffect(() => {
    const target = `solana:${network}` as const;
    if (clusterId === undefined || clusterId === target) return;
    setCluster(target).catch((error: unknown) => {
      console.warn(`Could not switch the wallet connector to ${network}`, error);
    });
  }, [clusterId, network, setCluster]);

  return null;
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

  return (
    <AppProvider connectorConfig={connectorConfig} mobile={mobile}>
      <ConnectorClusterSync />
      {children}
    </AppProvider>
  );
}
