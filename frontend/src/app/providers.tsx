"use client";

import {
  QueryCache,
  QueryClient,
} from "@tanstack/react-query";
import { PersistQueryClientProvider } from "@tanstack/react-query-persist-client";
import { createAsyncStoragePersister } from "@tanstack/query-async-storage-persister";
import { GET_GOVERNANCE_CONFIG } from "@/helpers";
import AppWalletProvider from "../components/AppWalletProvider";
import { EndpointProvider } from "../contexts/EndpointContext";
import { GovernanceConfigProvider } from "../contexts/GovernanceConfigContext";
import { NcnApiProvider } from "../contexts/NcnApiContext";
import { captureException } from "@sentry/nextjs";

const GOVERNANCE_CONFIG_PERSIST_MAX_AGE_MS = 60 * 60 * 1000; // 1 hour (aligns with useGovernanceConfig stale time)

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: { staleTime: 1000 * 10 },
  },
  queryCache: new QueryCache({
    onError: (error) => {
      console.error("Query error:", error);
      captureException(error);
    },
  }),
});

const governanceConfigPersister = createAsyncStoragePersister({
  storage: typeof window === "undefined" ? undefined : window.localStorage,
  key: "REACT_QUERY_GOVERNANCE_CONFIG",
});

export default function Providers({ children }: { children: React.ReactNode }) {
  return (
    <PersistQueryClientProvider
      client={queryClient}
      persistOptions={{
        persister: governanceConfigPersister,
        maxAge: GOVERNANCE_CONFIG_PERSIST_MAX_AGE_MS,
        dehydrateOptions: {
          shouldDehydrateQuery: (query) =>
            query.queryKey[0] === GET_GOVERNANCE_CONFIG,
        },
      }}
    >
      <EndpointProvider>
        <NcnApiProvider>
          <GovernanceConfigProvider>
            <AppWalletProvider>{children}</AppWalletProvider>
          </GovernanceConfigProvider>
        </NcnApiProvider>
      </EndpointProvider>
    </PersistQueryClientProvider>
  );
}
