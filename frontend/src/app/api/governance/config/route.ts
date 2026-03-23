import { cacheLife, cacheTag } from "next/cache";
import { NextResponse } from "next/server";
import { fetchGovernanceConfigFromChain } from "@/lib/getGovernanceConfig";

const REVALIDATE_SECONDS = 3600; // 1 hour

function getServerRpcUrl(): string {
  return (
    process.env.SOLANA_RPC_URL ??
    process.env.NEXT_PUBLIC_SOLANA_RPC_URL ??
    "https://api.mainnet.solana.com"
  );
}

async function getCachedGovernanceConfig(rpcUrl: string) {
  // Remote: in-memory cache doesn't persist across serverless requests; remote gives shared cache and fewer RPC hits.
  // ~1k MAU ≈ ~2–3k reads + ~1k writes/month → Runtime Cache cost well under $0.05/mo. Revalidate: 1h.
  "use cache: remote";
  cacheTag("governance-config");
  cacheLife({ revalidate: REVALIDATE_SECONDS });
  return fetchGovernanceConfigFromChain(rpcUrl);
}

/**
 * GET /api/governance/config
 * Returns the on-chain governance config (cached server-side). Safe to call from the client and cache there too.
 */
export async function GET() {
  try {
    const config = await getCachedGovernanceConfig(getServerRpcUrl());
    return NextResponse.json(config);
  } catch (e) {
    const message =
      e instanceof Error ? e.message : "Failed to fetch governance config";
    return NextResponse.json({ error: message }, { status: 500 });
  }
}
