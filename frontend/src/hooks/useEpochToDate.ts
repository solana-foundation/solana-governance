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
    // Keyed on the current epoch, not the current slot. `absoluteSlot` advances
    // constantly, so keying on it made every refresh a new cache entry rather
    // than a refetch: `staleTime` never applied, remounts always re-fetched, and
    // superseded entries piled up until garbage collection.
    //
    // The projection still has to be redone periodically — that is what
    // `refetchInterval` is for, and it recomputes in place. Including the epoch
    // covers the one input change the interval should not wait for: once the
    // target epoch is no longer in the future, `epochToDate` switches from
    // projecting to reading the epoch's actual block time.
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
    // Matches the cadence the slot-keyed version refreshed at in practice, since
    // it re-ran whenever useEpochInfo refetched. Each run re-reads the recent
    // slot rate, which is what keeps a multi-day countdown from drifting.
    staleTime: EPOCH_TO_DATE_REFRESH_MS,
    refetchInterval: EPOCH_TO_DATE_REFRESH_MS,
    refetchOnWindowFocus: false,
  });
}
