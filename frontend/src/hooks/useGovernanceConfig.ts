import type { GovernanceConfigDto } from "@/lib/getGovernanceConfig";
import { useQuery } from "@tanstack/react-query";
import { GET_GOVERNANCE_CONFIG } from "@/helpers";

const GOVERNANCE_CONFIG_STALE_MS = 60 * 60 * 1000; // 1 hour (matches API revalidate)

async function fetchGovernanceConfig(): Promise<GovernanceConfigDto> {
  const res = await fetch("/api/governance/config");
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    const message =
      (body as { error?: string })?.error ?? `Failed to fetch config (${res.status})`;
    throw new Error(message);
  }
  return res.json() as Promise<GovernanceConfigDto>;
}

/**
 * Fetches the on-chain governance config from the API (client-side).
 * Cached for 1 hour. Safe to use in any client component.
 */
export function useGovernanceConfig() {
  return useQuery<GovernanceConfigDto>({
    queryKey: [GET_GOVERNANCE_CONFIG],
    queryFn: fetchGovernanceConfig,
    staleTime: GOVERNANCE_CONFIG_STALE_MS,
    gcTime: GOVERNANCE_CONFIG_STALE_MS,
    refetchOnWindowFocus: false,
  });
}
