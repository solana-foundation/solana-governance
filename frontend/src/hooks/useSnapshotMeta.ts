import { NetworkMetaResponse } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import { fetchNcnJson } from "@/lib/ncnApi";
import { queryOptions, useQueries, useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

const SNAPSHOT_META_STALE_TIME_MS = 1000 * 120;

function snapshotMetaQuery(ncnApiUrl: string, network: string, slot?: number) {
  return queryOptions({
    staleTime: SNAPSHOT_META_STALE_TIME_MS,
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

/** Kept at or below `MAX_METAS_SLOTS` in `ncn/verifier-service/src/main.rs`. */
const MAX_SLOTS_PER_REQUEST = 100;

function snapshotMetasQuery(
  ncnApiUrl: string,
  network: string,
  slots: number[],
) {
  return queryOptions({
    staleTime: SNAPSHOT_META_STALE_TIME_MS,
    queryKey: ["snapshot_metas", network, ncnApiUrl, slots],
    queryFn: ({ signal }) =>
      fetchNcnJson<NetworkMetaResponse[]>(
        `${ncnApiUrl}/metas?network=${network}&slots=${slots.join(",")}`,
        { signal, label: "snapshot meta info", resource: "snapshot-meta" },
      ),
  });
}

/**
 * Declared here rather than inline: `useQueries` caches the combined result
 * against this function's identity, so a new closure each render would rebuild
 * the map every time.
 */
function combineSnapshotMetas(
  results: { data?: NetworkMetaResponse[]; isLoading: boolean }[],
) {
  const bySlot = new Map<number, NetworkMetaResponse>();
  for (const { data } of results) {
    // Keyed by the slot each record reports, so a slot the verifier has pruned
    // is simply absent rather than mapped to another snapshot.
    for (const meta of data ?? []) bySlot.set(meta.slot, meta);
  }

  return { bySlot, isLoading: results.some((result) => result.isLoading) };
}

/**
 * Metadata for several snapshots, keyed by slot.
 *
 * Batched rather than one request per slot, which on a page listing proposals
 * would open a connection per row and exhaust the verifier's request burst.
 * Split across requests only when there are more slots than one request accepts,
 * so a long proposal history degrades to a few requests instead of a rejected
 * one that would leave every row without a denominator.
 */
export const useSnapshotMetas = (slots: (number | undefined)[]) => {
  const { network } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();

  // Sorted and de-duplicated so reordering the proposals does not change the
  // query keys.
  const batches = useMemo(() => {
    const unique = Array.from(
      new Set(slots.filter((slot): slot is number => Boolean(slot))),
    ).sort((a, b) => a - b);

    const chunks: number[][] = [];
    for (let i = 0; i < unique.length; i += MAX_SLOTS_PER_REQUEST) {
      chunks.push(unique.slice(i, i + MAX_SLOTS_PER_REQUEST));
    }
    return chunks;
  }, [slots]);

  return useQueries({
    queries: batches.map((batch) =>
      snapshotMetasQuery(ncnApiUrl, network, batch),
    ),
    combine: combineSnapshotMetas,
  });
};
