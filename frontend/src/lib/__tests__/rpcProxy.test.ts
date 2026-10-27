import { getRpcCachePolicy, parseJsonRpcRequest } from "../rpcProxy";

const STAKE_PROGRAM_ID = "Stake11111111111111111111111111111111111111";
const PUBLIC_KEY = "11111111111111111111111111111111";

describe("parseJsonRpcRequest", () => {
  it("accepts an RPC method used by the app", () => {
    expect(
      parseJsonRpcRequest({
        jsonrpc: "2.0",
        id: 7,
        method: "getEpochInfo",
        params: [{ commitment: "confirmed" }],
      }),
    ).toEqual({
      jsonrpc: "2.0",
      id: 7,
      method: "getEpochInfo",
      params: [{ commitment: "confirmed" }],
    });
  });

  it("rejects methods outside the allowlist", () => {
    expect(() =>
      parseJsonRpcRequest({
        jsonrpc: "2.0",
        id: 1,
        method: "getBlock",
        params: [123],
      }),
    ).toThrow(expect.objectContaining({ code: -32601, status: 403 }));
  });

  it("rejects JSON-RPC batches", () => {
    expect(() => parseJsonRpcRequest([])).toThrow(
      "Batch requests are not supported",
    );
  });

  it("restricts getProgramAccounts to application programs", () => {
    expect(() =>
      parseJsonRpcRequest({
        jsonrpc: "2.0",
        id: 1,
        method: "getProgramAccounts",
        params: ["11111111111111111111111111111111"],
      }),
    ).toThrow("restricted to application program IDs");
  });

  it("accepts stake scans with the application's restrictive filters", () => {
    expect(
      parseJsonRpcRequest({
        jsonrpc: "2.0",
        id: 1,
        method: "getProgramAccounts",
        params: [
          STAKE_PROGRAM_ID,
          {
            encoding: "jsonParsed",
            filters: [
              { dataSize: 200 },
              { memcmp: { offset: 44, bytes: PUBLIC_KEY } },
            ],
          },
        ],
      }),
    ).toEqual(expect.objectContaining({ method: "getProgramAccounts" }));
  });

  it("rejects unfiltered stake-program scans", () => {
    expect(() =>
      parseJsonRpcRequest({
        jsonrpc: "2.0",
        id: 1,
        method: "getProgramAccounts",
        params: [STAKE_PROGRAM_ID],
      }),
    ).toThrow("Stake account scans require");
  });

  it("rejects stake scans without a recognized public-key filter", () => {
    expect(() =>
      parseJsonRpcRequest({
        jsonrpc: "2.0",
        id: 1,
        method: "getProgramAccounts",
        params: [
          STAKE_PROGRAM_ID,
          {
            filters: [
              { dataSize: 200 },
              { memcmp: { offset: 0, bytes: PUBLIC_KEY } },
            ],
          },
        ],
      }),
    ).toThrow("Stake account scans require");
  });

  it("caps submitted transaction size", () => {
    expect(() =>
      parseJsonRpcRequest({
        jsonrpc: "2.0",
        id: 1,
        method: "sendTransaction",
        params: ["x".repeat(4097)],
      }),
    ).toThrow("at most 4096 characters");
  });
});

describe("RPC cache policy", () => {
  it("caches safe reads and never caches transaction/status methods", () => {
    expect(getRpcCachePolicy("getProgramAccounts")).toBeDefined();
    expect(getRpcCachePolicy("getBlockTime")?.expire).toBe(86400);
    expect(getRpcCachePolicy("getLatestBlockhash")).toBeUndefined();
    expect(getRpcCachePolicy("getSignatureStatuses")).toBeUndefined();
    expect(getRpcCachePolicy("simulateTransaction")).toBeUndefined();
    expect(getRpcCachePolicy("sendTransaction")).toBeUndefined();
  });
});
