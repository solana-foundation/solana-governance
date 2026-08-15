import { useEndpoint } from "@/contexts/EndpointContext";
import { useNcnApi } from "@/contexts/NcnApiContext";
import { getVoterWalletSummary } from "@/data";
import { isKnownSnapshotNetwork } from "@/lib/snapshotNetwork";
import { useQuery } from "@tanstack/react-query";
import { useSnapshotMeta } from "./useSnapshotMeta";

export const useVoterWalletSummary = (userPubKey: string | undefined) => {
  const { network } = useEndpoint();
  const { ncnApiUrl } = useNcnApi();
  const { data: meta } = useSnapshotMeta();
  const slot = meta?.slot;

  return useQuery({
    staleTime: 1000 * 120, // 2 minutes
    enabled:
      isKnownSnapshotNetwork(network) && slot !== undefined && !!userPubKey,
    queryKey: ["vote_wallet_summary", network, userPubKey, ncnApiUrl, slot],
    queryFn: ({ signal }) => {
      // Unreachable — `enabled` above already requires a known network and slot.
      if (!isKnownSnapshotNetwork(network) || slot === undefined) {
        throw new Error("Snapshot slot not loaded");
      }

      return getVoterWalletSummary(
        network,
        userPubKey,
        slot,
        ncnApiUrl,
        signal
      );
    },
  });
};
