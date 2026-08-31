import { cacheLife, cacheTag, revalidateTag } from "next/cache";
import { NextResponse, type NextRequest } from "next/server";
import { getRpcUrlForNetwork, type RpcEnvSource } from "@/lib/getRpcUrls";
import {
  getRpcCachePolicy,
  isRpcCluster,
  isWriteRpcMethod,
  parseJsonRpcRequest,
  RpcRequestError,
  type AllowedRpcMethod,
  type JsonRpcId,
  type JsonValue,
  type RpcCachePolicy,
} from "@/lib/rpcProxy";
import type { RpcNetwork } from "@/types";

const MAX_REQUEST_BYTES = 64 * 1024;
const UPSTREAM_TIMEOUT_MS = 15_000;
const RATE_LIMIT_WINDOW_MS = 60_000;
const MAX_RATE_LIMIT_BUCKETS = 10_000;
const WRITE_BUCKET_KEY = "write";
const READ_BUCKET_KEY = "read";
const WRITE_LIMIT = 30;
const READ_LIMIT = 240;

interface RateLimitEntry {
  count: number;
  resetAt: number;
}

interface UpstreamError {
  code: number;
  message: string;
  data?: JsonValue;
}

class RpcUpstreamError extends Error {
  constructor(readonly rpcError: UpstreamError) {
    super(rpcError.message);
    this.name = "RpcUpstreamError";
  }
}

// Per {ip:bucket} rate limits, where bucket is read/write request
const rateLimits = new Map<string, RateLimitEntry>();

function takeRateLimitToken(
  request: NextRequest,
  method: AllowedRpcMethod,
): boolean {
  const forwarded = request.headers.get("x-forwarded-for")?.split(",")[0];
  const ip = forwarded?.trim() || request.headers.get("x-real-ip") || "unknown";
  const [bucket, limit] = isWriteRpcMethod(method)
    ? [WRITE_BUCKET_KEY, WRITE_LIMIT]
    : [READ_BUCKET_KEY, READ_LIMIT];
  const key = `${ip}:${bucket}`;
  const now = Date.now();
  const existing = rateLimits.get(key);

  if (!existing || existing.resetAt <= now) {
    if (rateLimits.size >= MAX_RATE_LIMIT_BUCKETS) {
      for (const [candidateKey, entry] of rateLimits) {
        if (entry.resetAt <= now) rateLimits.delete(candidateKey);
      }
      if (rateLimits.size >= MAX_RATE_LIMIT_BUCKETS) return false;
    }
    rateLimits.set(key, { count: 1, resetAt: now + RATE_LIMIT_WINDOW_MS });
    return true;
  }
  if (existing.count >= limit) return false;
  existing.count += 1;
  return true;
}

function jsonRpcError(id: JsonRpcId, error: UpstreamError, status = 200) {
  return NextResponse.json(
    { jsonrpc: "2.0", id, error },
    { status, headers: { "Cache-Control": "no-store" } },
  );
}

async function callUpstream(
  cluster: RpcNetwork,
  method: AllowedRpcMethod,
  params: JsonValue[],
): Promise<JsonValue> {
  const rpcUrl = getRpcUrlForNetwork(cluster, process.env as RpcEnvSource);
  const response = await fetch(rpcUrl, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
    cache: "no-store",
    signal: AbortSignal.timeout(UPSTREAM_TIMEOUT_MS),
  });

  if (!response.ok) {
    throw new Error(`RPC upstream returned HTTP ${response.status}`);
  }

  const body = (await response.json()) as {
    result?: JsonValue;
    error?: UpstreamError;
  };
  if (body.error) throw new RpcUpstreamError(body.error);
  if (!("result" in body)) throw new Error("RPC upstream returned no result");
  return body.result as JsonValue;
}

async function callCachedUpstream(
  cluster: RpcNetwork,
  method: AllowedRpcMethod,
  params: JsonValue[],
  policy: RpcCachePolicy,
): Promise<JsonValue> {
  "use cache: remote";
  cacheTag("rpc", `rpc:${cluster}`, `rpc:${cluster}:${method}`);
  cacheLife(policy);
  return callUpstream(cluster, method, params);
}

function requestOriginIsAllowed(request: NextRequest): boolean {
  const origin = request.headers.get("origin");
  return !origin || origin === new URL(request.url).origin;
}

/** Same-origin, unauthenticated Solana JSON-RPC proxy. */
export async function POST(request: NextRequest) {
  let id: JsonRpcId = null;

  try {
    if (!requestOriginIsAllowed(request)) {
      throw new RpcRequestError(
        "Cross-origin requests are not allowed",
        -32600,
        403,
      );
    }

    const contentLength = Number(request.headers.get("content-length") ?? 0);
    if (contentLength > MAX_REQUEST_BYTES) {
      throw new RpcRequestError("RPC request is too large", -32600, 413);
    }

    const clusterParam = request.nextUrl.searchParams.get("cluster");
    if (!isRpcCluster(clusterParam)) {
      throw new RpcRequestError("Unsupported Solana cluster", -32602, 400);
    }

    const rawBody = await request.text();
    if (new TextEncoder().encode(rawBody).byteLength > MAX_REQUEST_BYTES) {
      throw new RpcRequestError("RPC request is too large", -32600, 413);
    }

    let value: unknown;
    try {
      value = JSON.parse(rawBody);
    } catch {
      throw new RpcRequestError("Request body must be valid JSON", -32700, 400);
    }

    const rpcRequest = parseJsonRpcRequest(value);
    id = rpcRequest.id;

    if (!takeRateLimitToken(request, rpcRequest.method)) {
      return jsonRpcError(
        id,
        { code: -32005, message: "Rate limit exceeded" },
        429,
      );
    }

    const policy = getRpcCachePolicy(rpcRequest.method);
    const result = policy
      ? await callCachedUpstream(
          clusterParam,
          rpcRequest.method,
          rpcRequest.params,
          policy,
        )
      : await callUpstream(clusterParam, rpcRequest.method, rpcRequest.params);

    if (rpcRequest.method === "sendTransaction") {
      revalidateTag(`rpc:${clusterParam}`, "max");
    }

    return NextResponse.json(
      { jsonrpc: "2.0", id, result },
      { headers: { "Cache-Control": "no-store" } },
    );
  } catch (error) {
    if (error instanceof RpcRequestError) {
      return jsonRpcError(
        id,
        { code: error.code, message: error.message },
        error.status,
      );
    }
    if (error instanceof RpcUpstreamError) {
      return jsonRpcError(id, error.rpcError);
    }
    const message =
      error instanceof Error && error.name === "TimeoutError"
        ? "RPC upstream timed out"
        : "RPC upstream unavailable";
    return jsonRpcError(id, { code: -32000, message }, 502);
  }
}
