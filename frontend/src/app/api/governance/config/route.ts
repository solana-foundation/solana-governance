import { cacheLife, cacheTag } from "next/cache";
import { type NextRequest, NextResponse } from "next/server";
import { z } from "zod";
import { fetchGovernanceConfigFromChain } from "@/lib/getGovernanceConfig";
import {
  getRpcUrlForEndpoint,
  type RpcEnvSource,
} from "@/lib/getRpcUrls";

const REVALIDATE_SECONDS = 3600; // 1 hour

/**
 * Preset clusters use env-backed RPC URLs (`getRpcUrlForEndpoint`).
 * `endpoint=custom` requires `rpcUrl` (validated URL string).
 */
const governanceConfigQuerySchema = z
  .object({
    endpoint: z
      .enum(["mainnet", "testnet", "devnet", "custom"])
      .default("mainnet"),
    /** Only read when `endpoint` is `custom`; preset requests may omit or ignore this. */
    rpcUrl: z.string().optional(),
  })
  .superRefine((data, ctx) => {
    if (data.endpoint !== "custom") return;
    const trimmed = data.rpcUrl?.trim();
    if (!trimmed) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "rpcUrl query parameter is required when endpoint=custom",
        path: ["rpcUrl"],
      });
      return;
    }
    try {
      new URL(trimmed);
    } catch {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "Invalid rpcUrl",
        path: ["rpcUrl"],
      });
    }
  });

async function getCachedGovernanceConfig(rpcUrl: string, cacheKey: string) {
  // Remote: in-memory cache doesn't persist across serverless requests; remote gives shared cache and fewer RPC hits.
  // Cache key includes network so mainnet/testnet/devnet have separate entries.
  "use cache: remote";
  cacheTag("governance-config", cacheKey);
  cacheLife({ revalidate: REVALIDATE_SECONDS });
  return fetchGovernanceConfigFromChain(rpcUrl);
}

/**
 * GET /api/governance/config?endpoint=mainnet|testnet|devnet
 * GET /api/governance/config?endpoint=custom&rpcUrl=https%3A%2F%2F...
 * Returns the on-chain governance config (cached server-side). Preset RPC URLs come from env
 * (`getRpcUrlForEndpoint`); custom uses the provided `rpcUrl`.
 */
export async function GET(request: NextRequest) {
  try {
    const { searchParams } = new URL(request.url);
    const parsed = governanceConfigQuerySchema.safeParse({
      endpoint: searchParams.get("endpoint") ?? "mainnet",
      rpcUrl: searchParams.get("rpcUrl") ?? undefined,
    });

    if (!parsed.success) {
      const flat = parsed.error.flatten();
      const message =
        flat.fieldErrors.endpoint?.[0] ??
        flat.fieldErrors.rpcUrl?.[0] ??
        flat.formErrors[0] ??
        parsed.error.message;
      return NextResponse.json({ error: message }, { status: 400 });
    }

    const { endpoint, rpcUrl: rpcUrlParam } = parsed.data;
    const env = process.env as RpcEnvSource;
    let rpcUrl: string;
    try {
      rpcUrl = getRpcUrlForEndpoint(
        endpoint,
        endpoint === "custom" ? rpcUrlParam?.trim() : undefined,
        env,
      );
    } catch (e) {
      const message = e instanceof Error ? e.message : "Invalid RPC URL";
      return NextResponse.json({ error: message }, { status: 400 });
    }

    const cacheKey = endpoint === "custom" ? `custom:${rpcUrl}` : endpoint;
    const config = await getCachedGovernanceConfig(rpcUrl, cacheKey);
    return NextResponse.json(config);
  } catch (e) {
    const message =
      e instanceof Error ? e.message : "Failed to fetch governance config";
    return NextResponse.json({ error: message }, { status: 500 });
  }
}
