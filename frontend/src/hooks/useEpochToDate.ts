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
    // The epoch, not `absoluteSlot`: a key that moves every refresh makes each
    // one a new cache entry rather than a refetch, so `staleTime` never applies.
    // The epoch still belongs here because `epochToDate` switches from
    // projecting to reading actual block time once the target is no longer
    // ahead, which should not wait for the interval below.
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
    // Refetching in place keeps the cadence the slot-keyed version reached by
    // accident, without minting an entry each time.
    staleTime: EPOCH_TO_DATE_REFRESH_MS,
    refetchInterval: EPOCH_TO_DATE_REFRESH_MS,
    refetchOnWindowFocus: false,
  });
}
