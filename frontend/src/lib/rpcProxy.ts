import type { RpcNetwork } from "@/types";

export const RPC_CLUSTERS = ["mainnet", "testnet", "devnet"] as const;

export const RPC_METHODS = [
  "getAccountInfo",
  "getBlockTime",
  "getEpochInfo",
  "getEpochSchedule",
  "getLatestBlockhash",
  "getProgramAccounts",
  "getRecentPerformanceSamples",
  "getSignatureStatuses",
  "getVoteAccounts",
  "sendTransaction",
  "simulateTransaction",
] as const;

export type AllowedRpcMethod = (typeof RPC_METHODS)[number];
export type JsonRpcId = string | number | null;
export type JsonValue =
  null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };

export interface JsonRpcRequest {
  jsonrpc: "2.0";
  id: JsonRpcId;
  method: AllowedRpcMethod;
  params: JsonValue[];
}

export interface RpcCachePolicy {
  stale: number;
  revalidate: number;
  expire: number;
}

const METHOD_SET = new Set<string>(RPC_METHODS);
const CLUSTER_SET = new Set<string>(RPC_CLUSTERS);

// Restrict the expensive getProgramAccounts scan to programs this app queries.
const ALLOWED_PROGRAM_IDS = new Set([
  "govYkyQ3ePtGULAtY6V75qjWE8UH4vCUVQ1W4HdCAZU",
  "ncnwF8AgynRcdEnGLcprSQNaKvgSMTgk3yPRc8cf9Zf",
  "Stake11111111111111111111111111111111111111",
]);
const STAKE_PROGRAM_ID = "Stake11111111111111111111111111111111111111";
const STAKE_ACCOUNT_DATA_SIZE = 200;
const STAKE_ACCOUNT_MEMCMP_OFFSETS = new Set([44, 124]);
const BASE58_PUBLIC_KEY = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/;

const CACHE_POLICIES: Partial<Record<AllowedRpcMethod, RpcCachePolicy>> = {
  getAccountInfo: { stale: 1, revalidate: 2, expire: 10 },
  getEpochInfo: { stale: 2, revalidate: 5, expire: 15 },
  getProgramAccounts: { stale: 10, revalidate: 30, expire: 120 },
  getRecentPerformanceSamples: { stale: 10, revalidate: 30, expire: 120 },
  getVoteAccounts: { stale: 15, revalidate: 60, expire: 300 },
  // These values are immutable for a given slot/cluster or practically static.
  getBlockTime: { stale: 3600, revalidate: 3600, expire: 86400 },
  getEpochSchedule: { stale: 3600, revalidate: 3600, expire: 86400 },
};

export class RpcRequestError extends Error {
  constructor(
    message: string,
    readonly code: number,
    readonly status: number,
  ) {
    super(message);
    this.name = "RpcRequestError";
  }
}

export function isRpcCluster(value: string | null): value is RpcNetwork {
  return value !== null && CLUSTER_SET.has(value);
}

export function isWriteRpcMethod(method: AllowedRpcMethod): boolean {
  return method === "sendTransaction" || method === "simulateTransaction";
}

export function getRpcCachePolicy(
  method: AllowedRpcMethod,
): RpcCachePolicy | undefined {
  return CACHE_POLICIES[method];
}

function isJsonValue(value: unknown, depth = 0): value is JsonValue {
  if (depth > 12) return false;
  if (
    value === null ||
    typeof value === "string" ||
    typeof value === "boolean"
  ) {
    return true;
  }
  if (typeof value === "number") return Number.isFinite(value);
  if (Array.isArray(value)) {
    return value.every((item) => isJsonValue(item, depth + 1));
  }
  if (typeof value === "object") {
    return Object.values(value).every((item) => isJsonValue(item, depth + 1));
  }
  return false;
}

function validateMethodParams(
  method: AllowedRpcMethod,
  params: JsonValue[],
): void {
  if (method === "getProgramAccounts") {
    const programId = params[0];
    if (typeof programId !== "string" || !ALLOWED_PROGRAM_IDS.has(programId)) {
      throw new RpcRequestError(
        "getProgramAccounts is restricted to application program IDs",
        -32602,
        403,
      );
    }

    const config = params[1];
    if (config && !Array.isArray(config) && typeof config === "object") {
      const filters = config.filters;
      if (Array.isArray(filters) && filters.length > 8) {
        throw new RpcRequestError("Too many account filters", -32602, 400);
      }
    }

    if (programId === STAKE_PROGRAM_ID) {
      const filters =
        config && !Array.isArray(config) && typeof config === "object"
          ? config.filters
          : undefined;
      const hasDataSizeFilter =
        Array.isArray(filters) &&
        filters.some(
          (filter) =>
            filter !== null &&
            !Array.isArray(filter) &&
            typeof filter === "object" &&
            filter.dataSize === STAKE_ACCOUNT_DATA_SIZE,
        );
      const hasRecognizedMemcmpFilter =
        Array.isArray(filters) &&
        filters.some((filter) => {
          if (
            filter === null ||
            Array.isArray(filter) ||
            typeof filter !== "object"
          ) {
            return false;
          }

          const memcmp = filter.memcmp;
          return (
            memcmp !== null &&
            !Array.isArray(memcmp) &&
            typeof memcmp === "object" &&
            typeof memcmp.offset === "number" &&
            STAKE_ACCOUNT_MEMCMP_OFFSETS.has(memcmp.offset) &&
            typeof memcmp.bytes === "string" &&
            BASE58_PUBLIC_KEY.test(memcmp.bytes)
          );
        });

      if (!hasDataSizeFilter || !hasRecognizedMemcmpFilter) {
        throw new RpcRequestError(
          "Stake account scans require data-size and recognized public-key filters",
          -32602,
          400,
        );
      }
    }
  }

  if (isWriteRpcMethod(method)) {
    const transaction = params[0];
    if (typeof transaction !== "string" || transaction.length > 4096) {
      throw new RpcRequestError(
        "A base64 transaction of at most 4096 characters is required",
        -32602,
        400,
      );
    }
  }
}

export function parseJsonRpcRequest(value: unknown): JsonRpcRequest {
  if (Array.isArray(value)) {
    throw new RpcRequestError("Batch requests are not supported", -32600, 400);
  }
  if (!value || typeof value !== "object") {
    throw new RpcRequestError("Invalid JSON-RPC request", -32600, 400);
  }

  const request = value as Record<string, unknown>;
  if (request.jsonrpc !== "2.0") {
    throw new RpcRequestError("jsonrpc must be 2.0", -32600, 400);
  }
  if (
    request.id !== null &&
    typeof request.id !== "string" &&
    typeof request.id !== "number"
  ) {
    throw new RpcRequestError("A JSON-RPC id is required", -32600, 400);
  }
  if (typeof request.method !== "string" || !METHOD_SET.has(request.method)) {
    throw new RpcRequestError("RPC method is not allowed", -32601, 403);
  }

  const params = request.params ?? [];
  if (!Array.isArray(params) || !isJsonValue(params)) {
    throw new RpcRequestError("params must be a JSON array", -32602, 400);
  }

  const parsed: JsonRpcRequest = {
    jsonrpc: "2.0",
    id: request.id as JsonRpcId,
    method: request.method as AllowedRpcMethod,
    params,
  };
  validateMethodParams(parsed.method, parsed.params);
  return parsed;
}
