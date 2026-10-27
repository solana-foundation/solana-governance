import { useQuery } from "@tanstack/react-query";
import { useEndpoint } from "@/contexts/EndpointContext";
import { epochToDate } from "@/helpers/date";
import { useEpochInfo } from "./useEpochInfo";

/** How often the projection is redone to pick up a changed slot rate. */
const EPOCH_TO_DATE_REFRESH_MS = 5 * 60 * 1000;

/**
 * Hook to convert a Solana epoch number to a Date
 * @param epoch - The epoch number to convert
 * @returns The date when the epoch will start, or null if loading/error
 */
export function useEpochToDate(epoch: number | undefined) {
  const { endpointUrl } = useEndpoint();
  const { data: epochData, isLoading: isLoadingEpochInfo } = useEpochInfo();

  return useQuery({
    queryKey: ["epochToDate", epoch, endpointUrl, epochData?.epochInfo.epoch],
    queryFn: async () => {
      if (epoch === undefined || !epochData) return null;
      return epochToDate(
        epoch,
        epochData.epochInfo,
        epochData.epochSchedule,
        endpointUrl,
      );
    },
    enabled: epoch !== undefined && !isLoadingEpochInfo && !!epochData,
    staleTime: EPOCH_TO_DATE_REFRESH_MS,
    refetchInterval: EPOCH_TO_DATE_REFRESH_MS,
    refetchOnWindowFocus: false,
  });
}
