import { cacheLife, cacheTag } from "next/cache";
import { type NextRequest, NextResponse } from "next/server";
import { z } from "zod";
import { fetchGovernanceConfigFromChain } from "@/lib/getGovernanceConfig";
import {
  getRpcUrlForEndpoint,
  type RpcEnvSource,
} from "@/lib/getRpcUrls";

const REVALIDATE_SECONDS = 3600; // 1 hour

const governanceConfigQuerySchema = z
  .object({
    endpoint: z
      .enum(["mainnet", "testnet", "devnet", "custom"])
      .default("mainnet"),
    rpcUrl: z.string().url().optional(),
  })
  .refine(
    (data) =>
      data.endpoint !== "custom" || (data.rpcUrl != null && data.rpcUrl !== ""),
    { message: "rpcUrl is required when endpoint is custom", path: ["rpcUrl"] },
  );

async function getCachedGovernanceConfig(rpcUrl: string, cacheKey: string) {
  // Remote: in-memory cache doesn't persist across serverless requests; remote gives shared cache and fewer RPC hits.
  // Cache key includes network so mainnet/testnet/devnet have separate entries.
  "use cache: remote";
  cacheTag("governance-config", cacheKey);
  cacheLife({ revalidate: REVALIDATE_SECONDS });
  return fetchGovernanceConfigFromChain(rpcUrl);
}

/**
 * GET /api/governance/config?endpoint=mainnet|testnet|devnet|custom&rpcUrl=...
 * Returns the on-chain governance config for the given RPC endpoint (cached server-side).
 * When endpoint=custom, rpcUrl query param is required.
 */
export async function GET(request: NextRequest) {
  try {
    const { searchParams } = new URL(request.url);
    const parsed = governanceConfigQuerySchema.safeParse({
      endpoint: searchParams.get("endpoint") ?? "mainnet",
      rpcUrl: searchParams.get("rpcUrl") ?? undefined,
    });

    if (!parsed.success) {
      const first = parsed.error.flatten().fieldErrors;
      const message =
        first.rpcUrl?.[0] ??
        first.endpoint?.[0] ??
        parsed.error.message;
      return NextResponse.json({ error: message }, { status: 400 });
    }

    const { endpoint, rpcUrl: customRpcUrl } = parsed.data;
    const rpcUrl = getRpcUrlForEndpoint(
      endpoint,
      customRpcUrl,
      process.env as RpcEnvSource,
    );
    const cacheKey = endpoint === "custom" ? `custom:${rpcUrl}` : endpoint;
    const config = await getCachedGovernanceConfig(rpcUrl, cacheKey);
    return NextResponse.json(config);
  } catch (e) {
    const message =
      e instanceof Error ? e.message : "Failed to fetch governance config";
    return NextResponse.json({ error: message }, { status: 500 });
  }
}
