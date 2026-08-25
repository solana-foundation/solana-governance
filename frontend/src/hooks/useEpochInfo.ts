import { useQuery } from "@tanstack/react-query";
import { createSolanaRpc } from "@solana/kit";
import { useEndpoint } from "@/contexts/EndpointContext";
import { GET_EPOCH_INFO } from "@/helpers";

export interface EpochInfoData {
  epochInfo: {
    absoluteSlot: bigint;
    epoch: bigint;
    slotIndex: bigint;
    slotsInEpoch: bigint;
  };
  epochSchedule: {
    firstNormalEpoch: bigint;
    firstNormalSlot: bigint;
    leaderScheduleSlotOffset: bigint;
    slotsPerEpoch: bigint;
    warmup: boolean;
  };
}

export function useEpochInfo() {
  const { endpointUrl } = useEndpoint();

  return useQuery<EpochInfoData>({
    queryKey: [GET_EPOCH_INFO, endpointUrl],
    queryFn: async () => {
      const rpc = createSolanaRpc(endpointUrl);
      const [epochInfo, epochSchedule] = await Promise.all([
        rpc.getEpochInfo().send(),
        rpc.getEpochSchedule().send(),
      ]);
      return { epochInfo, epochSchedule };
    },
    staleTime: 5 * 60 * 1000,
    refetchOnWindowFocus: false,
  });
}
