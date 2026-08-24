import { useQuery } from "@tanstack/react-query";
import { useEndpoint } from "@/contexts/EndpointContext";
import { Connection, EpochInfo, EpochSchedule } from "@solana/web3.js";
import { GET_EPOCH_INFO } from "@/helpers";

export interface EpochInfoData {
  epochInfo: EpochInfo;
  epochSchedule: EpochSchedule;
}

/**
 * Hook to fetch Solana epoch info and epoch schedule
 * Caches the data and connection for reuse across all proposals
 * @returns The epoch info and schedule, or undefined if loading/error
 */
export function useEpochInfo() {
  const { endpointUrl } = useEndpoint();

  return useQuery<EpochInfoData>({
    queryKey: [GET_EPOCH_INFO, endpointUrl],
    queryFn: async () => {
      const connection = new Connection(endpointUrl, "confirmed");

      // Fetch both epoch info and schedule in parallel
      const [epochInfo, epochSchedule] = await Promise.all([
        connection.getEpochInfo(),
        connection.getEpochSchedule(),
      ]);

      return {
        epochInfo,
        epochSchedule,
      };
    },
    staleTime: 5 * 60 * 1000, // 5 minutes - epoch info doesn't change frequently
    refetchInterval: 5 * 60 * 1000,
    refetchOnWindowFocus: false,
  });
}
