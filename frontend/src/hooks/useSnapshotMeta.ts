import { NetworkMetaResponse } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import { fetchNcnJson } from "@/lib/ncnApi";
import { queryOptions, useQuery } from "@tanstack/react-query";
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

/**
 * Metadata for several snapshots, keyed by slot.
 *
 * One request rather than one per slot: a page listing proposals would
 * otherwise open a connection per row and can exhaust the verifier's request
 * burst before it has finished rendering.
 */
export const useSnapshotMetas = (slots: (number | undefined)[]) => {
  const { network } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();

  // Sorted and de-duplicated so reordering the proposals does not change the
  // query key.
  const uniqueSlots = useMemo(
    () =>
      Array.from(
        new Set(slots.filter((slot): slot is number => Boolean(slot))),
      ).sort((a, b) => a - b),
    [slots],
  );

  const { data, isLoading } = useQuery({
    staleTime: SNAPSHOT_META_STALE_TIME_MS,
    enabled: uniqueSlots.length > 0,
    queryKey: ["snapshot_metas", network, ncnApiUrl, uniqueSlots],
    queryFn: ({ signal }) =>
      fetchNcnJson<NetworkMetaResponse[]>(
        `${ncnApiUrl}/metas?network=${network}&slots=${uniqueSlots.join(",")}`,
        { signal, label: "snapshot meta info", resource: "snapshot-meta" },
      ),
  });

  return useMemo(
    () => ({
      // Keyed by the slot each record reports, so a slot the verifier has
      // pruned is simply absent rather than mapped to another snapshot.
      bySlot: new Map((data ?? []).map((meta) => [meta.slot, meta])),
      isLoading,
    }),
    [data, isLoading],
  );
};
