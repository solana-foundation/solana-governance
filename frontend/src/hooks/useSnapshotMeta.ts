import { NetworkMetaResponse } from "@/chain";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import { fetchNcnJson } from "@/lib/ncnApi";
import { useQuery } from "@tanstack/react-query";

export const useSnapshotMeta = () => {
  const { endpointType } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();

  return useQuery({
    staleTime: 1000 * 120, // 2 minutes
    // A custom RPC has no corresponding snapshot on the NCN API, so the request can only
    // fail. Skip it rather than spending three retries proving that.
    enabled: endpointType !== "custom",
    // Retry count comes from the query client default, which also skips retries once the
    // upstream has answered definitively (see isPermanentNcnFailure).
    //
    // Jittered so a fleet of clients retrying at once does not hammer an already-stalling
    // router in lockstep.
    retryDelay: (attempt) =>
      Math.min(1000 * 2 ** attempt, 8000) + Math.random() * 250,
    queryKey: ["snapshot_meta", endpointType, ncnApiUrl],
    queryFn: ({ signal }): Promise<NetworkMetaResponse> => {
      const network = endpointType;
      const url = `${ncnApiUrl}/meta?network=${network}`;

      return fetchNcnJson<NetworkMetaResponse>(url, {
        signal,
        label: "snapshot meta info",
      });
    },
  });
};
