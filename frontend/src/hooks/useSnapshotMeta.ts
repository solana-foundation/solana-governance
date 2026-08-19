import type { NetworkMetaResponse } from "@/lib/ncnProofs";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import { fetchNcnJson } from "@/lib/ncnApi";
import {
  isKnownSnapshotNetwork,
  requireKnownSnapshotNetwork,
} from "@/lib/snapshotNetwork";
import { useQuery } from "@tanstack/react-query";

export const useSnapshotMeta = () => {
  const { network } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();

  return useQuery({
    staleTime: 1000 * 120, // 2 minutes
    // The NCN API needs a known cluster. Skip until EndpointContext has resolved one,
    // rather than spending three retries on a custom or unrecognized RPC.
    enabled: isKnownSnapshotNetwork(network),
    // Retry count comes from the query client default, which also skips retries once the
    // upstream has answered definitively (see isPermanentNcnFailure).
    //
    // Jittered so a fleet of clients retrying at once does not hammer an already-stalling
    // router in lockstep.
    retryDelay: (attempt) =>
      Math.min(1000 * 2 ** attempt, 8000) + Math.random() * 250,
    queryKey: ["snapshot_meta", network, ncnApiUrl],
    queryFn: ({ signal }): Promise<NetworkMetaResponse> => {
      requireKnownSnapshotNetwork(network);
      const url = `${ncnApiUrl}/meta?network=${network}`;

      return fetchNcnJson<NetworkMetaResponse>(url, {
        signal,
        label: "snapshot meta info",
      });
    },
  });
};
