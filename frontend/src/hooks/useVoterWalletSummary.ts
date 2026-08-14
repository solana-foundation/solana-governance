import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import { getVoterWalletSummary } from "@/data";
import { useQuery } from "@tanstack/react-query";
import { useSnapshotMeta } from "./useSnapshotMeta";

export const useVoterWalletSummary = (userPubKey: string | undefined) => {
  const { endpointType } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();
  const { data: meta } = useSnapshotMeta();
  const slot = meta?.slot;

  return useQuery({
    staleTime: 1000 * 120, // 2 minutes
    enabled: slot !== undefined && !!userPubKey,
    queryKey: ["vote_wallet_summary", endpointType, userPubKey, ncnApiUrl, slot],
    queryFn: ({ signal }) => {
      // Unreachable — `enabled` above already requires a slot. Present only to narrow the type.
      if (slot === undefined) throw new Error("Snapshot slot not loaded");

      return getVoterWalletSummary(
        endpointType,
        userPubKey,
        slot,
        ncnApiUrl,
        signal
      );
    },
  });
};
