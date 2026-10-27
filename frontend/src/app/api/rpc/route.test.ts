/** @jest-environment node */

jest.mock("server-only", () => ({}));
jest.mock("next/cache", () => ({
  cacheLife: jest.fn(),
  cacheTag: jest.fn(),
  revalidateTag: jest.fn(),
}));

import { NextRequest } from "next/server";
import { POST } from "./route";

const UPSTREAM_URL = "https://rpc.internal.example/?api-key=server-secret";

function rpcRequest(
  body: unknown,
  options: { cluster?: string; origin?: string } = {},
) {
  const cluster = options.cluster ?? "mainnet";
  const headers = new Headers({ "Content-Type": "application/json" });
  if (options.origin) headers.set("Origin", options.origin);
  return new NextRequest(`https://governance.example/api/rpc?cluster=${cluster}`, {
    method: "POST",
    headers,
    body: JSON.stringify(body),
  });
}

describe("POST /api/rpc", () => {
  beforeEach(() => {
    jest.restoreAllMocks();
    process.env.SOLANA_RPC_URL_MAINNET = UPSTREAM_URL;
  });

  afterAll(() => {
    delete process.env.SOLANA_RPC_URL_MAINNET;
  });

  it("forwards an allowed request without exposing the upstream URL", async () => {
    const fetchMock = jest.spyOn(global, "fetch").mockResolvedValue(
      new Response(JSON.stringify({ jsonrpc: "2.0", id: 1, result: { epoch: 9 } }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    const response = await POST(
      rpcRequest({ jsonrpc: "2.0", id: 42, method: "getEpochInfo", params: [] }),
    );

    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body).toEqual({
      jsonrpc: "2.0",
      id: 42,
      result: { epoch: 9 },
    });
    expect(fetchMock).toHaveBeenCalledWith(
      UPSTREAM_URL,
      expect.objectContaining({ method: "POST", cache: "no-store" }),
    );
    expect(JSON.stringify(body)).not.toContain("server-secret");
  });

  it("rejects a method outside the allowlist before calling upstream", async () => {
    const fetchMock = jest.spyOn(global, "fetch");
    const response = await POST(
      rpcRequest({ jsonrpc: "2.0", id: 1, method: "getBlock", params: [123] }),
    );

    expect(response.status).toBe(403);
    expect(await response.json()).toMatchObject({
      error: { code: -32601, message: "RPC method is not allowed" },
    });
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("rejects cross-origin browser requests", async () => {
    const response = await POST(
      rpcRequest(
        { jsonrpc: "2.0", id: 1, method: "getEpochInfo", params: [] },
        { origin: "https://attacker.example" },
      ),
    );

    expect(response.status).toBe(403);
    expect(await response.json()).toMatchObject({
      error: { message: "Cross-origin requests are not allowed" },
    });
  });
});
