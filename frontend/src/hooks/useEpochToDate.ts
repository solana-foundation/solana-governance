import { useQuery } from "@tanstack/react-query";
import { useEndpoint } from "@/contexts/EndpointContext";
import { epochToDate } from "@/helpers/date";
import { useEpochInfo } from "./useEpochInfo";

/**
 * Hook to convert a Solana epoch number to a Date
 * @param epoch - The epoch number to convert
 * @returns The date when the epoch will start, or null if loading/error
 */
export function useEpochToDate(epoch: bigint | undefined) {
  const { endpointUrl } = useEndpoint();
  const { data: epochData, isLoading: isLoadingEpochInfo } = useEpochInfo();

  return useQuery({
    queryKey: [
      "epochToDate",
      epoch?.toString(),
      endpointUrl,
      epochData?.epochInfo.absoluteSlot.toString(),
    ],
    queryFn: async () => {
      if (epoch === undefined || !epochData) return null;
      return epochToDate(
        epoch,
        epochData.epochInfo,
        epochData.epochSchedule,
        endpointUrl
      );
    },
    enabled: epoch !== undefined && !isLoadingEpochInfo && !!epochData,
    staleTime: 5 * 60 * 1000, // 5 minutes - epoch info doesn't change frequently
    refetchOnWindowFocus: false,
  });
}
