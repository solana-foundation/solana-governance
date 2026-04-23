import type { GovernanceConfigDto } from "@/lib/getGovernanceConfig";
import { useQuery } from "@tanstack/react-query";
import { GET_GOVERNANCE_CONFIG } from "@/helpers";
import { useEndpoint } from "@/contexts/EndpointContext";
import type { RPCEndpoint } from "@/types";

const GOVERNANCE_CONFIG_STALE_MS = 60 * 60 * 1000; // 1 hour (matches API revalidate)

function isPresetEndpoint(
  t: RPCEndpoint,
): t is Exclude<RPCEndpoint, "custom"> {
  return t !== "custom";
}

/**
 * Fetches the on-chain governance config from the API (client-side) for the current RPC endpoint.
 * Cached per rpc endpoint for 1 hour. Safe to use in any client component within EndpointProvider.
 */
export function useGovernanceConfig() {
  const { endpointType, endpointUrl } = useEndpoint();

  return useQuery<GovernanceConfigDto>({
    queryKey: isPresetEndpoint(endpointType)
      ? [GET_GOVERNANCE_CONFIG, endpointType]
      : [GET_GOVERNANCE_CONFIG, endpointType, endpointUrl],
    queryFn: async () => {
      const params = new URLSearchParams();
      if (isPresetEndpoint(endpointType)) {
        params.set("endpoint", endpointType);
      } else {
        params.set("endpoint", "custom");
        params.set("rpcUrl", endpointUrl);
      }
      const res = await fetch(`/api/governance/config?${params.toString()}`);
      if (!res.ok) {
        const body = await res.json().catch(() => ({}));
        const message =
          (body as { error?: string })?.error ??
          `Failed to fetch config (${res.status})`;
        throw new Error(message);
      }
      return res.json() as Promise<GovernanceConfigDto>;
    },
    staleTime: GOVERNANCE_CONFIG_STALE_MS,
    gcTime: GOVERNANCE_CONFIG_STALE_MS,
    refetchOnWindowFocus: false,
  });
}
