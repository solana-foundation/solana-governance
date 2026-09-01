import { NetworkMetaResponse } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import { fetchNcnJson } from "@/lib/ncnApi";
import { queryOptions, useQueries, useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

/** Shared so the table and a detail page reuse one cache entry per slot. */
function snapshotMetaQuery(ncnApiUrl: string, network: string, slot?: number) {
  return queryOptions({
    staleTime: 1000 * 120, // 2 minutes
    queryKey: ["snapshot_meta", network, ncnApiUrl, slot],
    queryFn: ({ signal }) => {
      const url =
        slot === undefined
          ? `${ncnApiUrl}/meta?network=${network}`
          : `${ncnApiUrl}/meta?network=${network}&slot=${slot}`;

      return fetchNcnJson<NetworkMetaResponse>(url, {
        signal,
        label: "snapshot meta info",
        resource: "snapshot-meta",
      });
    },
  });
}

/**
 * Metadata for one snapshot; the newest when `slot` is omitted.
 *
 * Anything measured against a proposal takes that proposal's `snapshotSlot`: it
 * votes against the snapshot frozen at activation, which stops being the newest
 * as soon as another is uploaded.
 */
export const useSnapshotMeta = (slot?: number) => {
  const { network } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();

  return useQuery(snapshotMetaQuery(ncnApiUrl, network, slot));
};

/**
 * Declared here rather than inline: `useQueries` caches the combined result
 * against this function's identity, so a new closure each render would rebuild
 * the map every time.
 */
function combineSnapshotMetas(
  results: { data?: NetworkMetaResponse; isLoading: boolean }[],
) {
  const bySlot = new Map<number, NetworkMetaResponse>();
  for (const { data } of results) {
    // Keyed by the slot the response reports, so a verifier that does not
    // support the parameter files its newest snapshot under its own slot rather
    // than under the one that was asked for.
    if (data) bySlot.set(data.slot, data);
  }

  return { bySlot, isLoading: results.some((result) => result.isLoading) };
}

/** Metadata for several snapshots, keyed by slot. */
export const useSnapshotMetas = (slots: (number | undefined)[]) => {
  const { network } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();

  // Sorted and de-duplicated so reordering the proposals does not change the
  // query list.
  const uniqueSlots = useMemo(
    () =>
      Array.from(
        new Set(slots.filter((slot): slot is number => Boolean(slot))),
      ).sort((a, b) => a - b),
    [slots],
  );

  return useQueries({
    queries: uniqueSlots.map((slot) =>
      snapshotMetaQuery(ncnApiUrl, network, slot),
    ),
    combine: combineSnapshotMetas,
  });
};
