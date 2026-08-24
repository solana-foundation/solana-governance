"use client";

import { QueryCache, QueryClient } from "@tanstack/react-query";
import { PersistQueryClientProvider } from "@tanstack/react-query-persist-client";
import { createAsyncStoragePersister } from "@tanstack/query-async-storage-persister";
import { GET_GOVERNANCE_CONFIG, GET_PROPOSAL_DOCUMENT } from "@/helpers";
import {
  classifyNcnFailure,
  isNcnProofNotFound,
  NcnApiHttpError,
  NcnApiNetworkError,
} from "@/lib/ncnApi";
import AppWalletProvider from "../components/AppWalletProvider";
import { EndpointProvider } from "../contexts/EndpointContext";
import { GovernanceConfigProvider } from "../contexts/GovernanceConfigContext";
import { NcnApiProvider } from "../contexts/NcnApiContext";
import { captureException } from "@sentry/nextjs";

const GOVERNANCE_CONFIG_PERSIST_MAX_AGE_MS = 60 * 60 * 1000; // 1 hour (aligns with useGovernanceConfig stale time)

/** Host only, never the full URL: voter summary URLs carry a wallet address, and tags are indexed. */
const ncnFailureTags = (error: unknown): Record<string, string> => {
  if (error instanceof NcnApiHttpError) {
    return { ncn_status: String(error.status), ncn_host: error.host };
  }
  if (error instanceof NcnApiNetworkError) return { ncn_host: error.host };
  return {};
};

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 10,
      // NCN requests exhaust their retries inside fetchNcnJson so mutations get the same
      // protection without retrying an entire transaction workflow. Do not multiply those
      // attempts here; retain React Query's former retry count for all other queries.
      retry: (failureCount, error) =>
        classifyNcnFailure(error) === undefined && failureCount < 3,
    },
  },
  queryCache: new QueryCache({
    // Fires once per query after its retries are exhausted, never for cancellations.
    onError: (error, query) => {
      // A proof endpoint uses 404 to say that this account has no leaf in the requested
      // historical snapshot. This is an expected voting outcome, not an application error.
      if (isNcnProofNotFound(error)) return;

      console.error("Query error:", error);

      const queryKey = String(query.queryKey[0]);
      // Proposal documents are fetched from GitHub, which is outside our control and rate
      // limited per IP. A failure there degrades to showing just the raw link, so it is not
      // worth an alert.
      if (queryKey === GET_PROPOSAL_DOCUMENT) return;

      const kind = classifyNcnFailure(error);
      const httpError = error instanceof NcnApiHttpError ? error : undefined;
      const tags = { query_key: queryKey, ...ncnFailureTags(error) };
      // The body is how we tell a router refusal from an operator's, since the status alone
      // never says which hop produced it.
      const extra = httpError?.bodySnippet
        ? { ncn_response_body: httpError.bodySnippet }
        : undefined;

      // Upstream being unreachable or refusing us is an infrastructure event, not a bug in the
      // app. Report it as a warning and collapse it to one issue per kind, otherwise a single
      // upstream outage buries everything else.
      if (kind === "network" || kind === "upstream") {
        captureException(error, {
          level: "warning",
          tags: { ...tags, failure_kind: kind },
          extra,
          fingerprint: httpError
            ? ["ncn-upstream-error", queryKey, String(httpError.status)]
            : // Unchanged from before this branch existed, so the existing issue keeps its history.
              ["network-failure", queryKey],
        });
        return;
      }

      // Present only for kind === "request": the upstream understood us and said no, so our
      // request was wrong (bad network param, or a network with no snapshot). Left at error
      // level so it does not hide among the infrastructure noise.
      if (httpError) {
        captureException(error, {
          tags: { ...tags, failure_kind: "request" },
          extra,
          fingerprint: [
            "ncn-request-error",
            queryKey,
            String(httpError.status),
          ],
        });
        return;
      }

      captureException(error, { tags });
    },
  }),
});

const governanceConfigPersister = createAsyncStoragePersister({
  storage: typeof window === "undefined" ? undefined : window.localStorage,
  key: "REACT_QUERY_GOVERNANCE_CONFIG",
  // GovernanceConfigDto deliberately retains Kit bigint values. TanStack's
  // persisted cache is JSON, so tag and revive only bigints instead of losing
  // precision by coercing them to numbers or leaving them as plain strings.
  serialize: (client) =>
    JSON.stringify(client, (_key, value) =>
      typeof value === "bigint"
        ? { __solanaGovernanceBigint: value.toString() }
        : value,
    ),
  deserialize: (cached) =>
    JSON.parse(cached, (_key, value) =>
      value && typeof value === "object" && "__solanaGovernanceBigint" in value
        ? BigInt(value.__solanaGovernanceBigint)
        : value,
    ),
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
