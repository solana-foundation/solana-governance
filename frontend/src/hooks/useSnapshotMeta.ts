import {
  deserializeNetworkMeta,
  type NetworkMetaResponse,
  type NetworkMetaWireResponse,
} from "@/lib/ncnProofs";
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
    queryKey: ["snapshot_meta", network, ncnApiUrl],
    queryFn: ({ signal }): Promise<NetworkMetaResponse> => {
      requireKnownSnapshotNetwork(network);
      const url = `${ncnApiUrl}/meta?network=${network}`;

      return fetchNcnJson<NetworkMetaWireResponse>(url, {
        signal,
        label: "snapshot meta info",
        resource: "snapshot-meta",
      }).then(deserializeNetworkMeta);
    },
  });
};
