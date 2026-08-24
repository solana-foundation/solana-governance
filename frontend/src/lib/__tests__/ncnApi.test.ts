import {
  classifyNcnFailure,
  fetchNcnJson,
  isNetworkFailure,
  isNcnProofNotFound,
  isPermanentNcnFailure,
  NcnApiHttpError,
  NcnApiNetworkError,
} from "../ncnApi";

const URL_UNDER_TEST = "https://ncn-governance.solana.com/meta?network=mainnet";

describe("fetchNcnJson", () => {
  const originalFetch = global.fetch;

  afterEach(() => {
    global.fetch = originalFetch;
    jest.useRealTimers();
    jest.restoreAllMocks();
  });

  it("returns the parsed body on success", async () => {
    const meta = { network: "mainnet", slot: 422497000 };
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => meta,
    }) as unknown as typeof fetch;

    await expect(
      fetchNcnJson(URL_UNDER_TEST, { label: "snapshot meta info" }),
    ).resolves.toEqual(meta);
  });

  it("preserves selected u64 fields beyond Number.MAX_SAFE_INTEGER", async () => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      status: 200,
      text: async () =>
        '{"meta_merkle_leaf":{"active_stake":16054077974334921},"snapshot_slot":440641000}',
    }) as unknown as typeof fetch;

    await expect(
      fetchNcnJson<{
        meta_merkle_leaf: { active_stake: string };
        snapshot_slot: number;
      }>(URL_UNDER_TEST, {
        label: "vote account proof",
        losslessIntegerFields: ["active_stake"],
      }),
    ).resolves.toEqual({
      meta_merkle_leaf: { active_stake: "16054077974334921" },
      snapshot_slot: 440641000,
    });
  });

  it("throws NcnApiHttpError naming the status, the operator that answered, and its body", async () => {
    // HTTP/2 has no reason phrase, so statusText is empty in practice — the numeric status
    // is the only thing that identifies the failure. The router redirects, so response.url is
    // the operator that refused us, not the host we asked.
    global.fetch = jest.fn().mockResolvedValue({
      ok: false,
      status: 503,
      statusText: "",
      url: "https://ncn.brewlabs.so/meta?network=mainnet",
      text: async () => '{"error":"snapshot not ready"}',
      json: async () => ({}),
    }) as unknown as typeof fetch;

    const error = await fetchNcnJson(URL_UNDER_TEST, {
      label: "snapshot meta info",
      maxRetries: 0,
    }).catch((e: unknown) => e);

    expect(error).toBeInstanceOf(NcnApiHttpError);
    expect((error as NcnApiHttpError).status).toBe(503);
    expect((error as NcnApiHttpError).host).toBe("ncn.brewlabs.so");
    expect((error as NcnApiHttpError).bodySnippet).toBe(
      '{"error":"snapshot not ready"}',
    );
    expect((error as Error).message).toContain("503");
    expect((error as Error).message).toContain("ncn.brewlabs.so");
  });

  it("truncates a long error body", async () => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: false,
      status: 403,
      statusText: "Forbidden",
      url: URL_UNDER_TEST,
      // A Cloudflare block page is kilobytes of HTML; only the head of it is worth uploading.
      text: async () => "x".repeat(1000),
    }) as unknown as typeof fetch;

    const error = await fetchNcnJson(URL_UNDER_TEST, {
      label: "snapshot meta info",
      maxRetries: 0,
    }).catch((e: unknown) => e);

    expect((error as NcnApiHttpError).bodySnippet).toHaveLength(200);
  });

  it("still reports the status when the error body cannot be read", async () => {
    // The internal timeout stays armed while the body is read, so the read itself can abort.
    // Losing a known 403 to an opaque AbortError is the failure this client exists to prevent.
    global.fetch = jest.fn().mockResolvedValue({
      ok: false,
      status: 403,
      statusText: "Forbidden",
      url: URL_UNDER_TEST,
      text: () => Promise.reject(new DOMException("Aborted", "AbortError")),
    }) as unknown as typeof fetch;

    const error = await fetchNcnJson(URL_UNDER_TEST, {
      label: "snapshot meta info",
      maxRetries: 0,
    }).catch((e: unknown) => e);

    expect(error).toBeInstanceOf(NcnApiHttpError);
    expect((error as NcnApiHttpError).status).toBe(403);
    expect((error as NcnApiHttpError).bodySnippet).toBe("");
  });

  it("falls back to the requested host when the response has no url", async () => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: false,
      status: 500,
      statusText: "",
      url: "",
      text: async () => "",
    }) as unknown as typeof fetch;

    const error = await fetchNcnJson(URL_UNDER_TEST, {
      label: "snapshot meta info",
      maxRetries: 0,
    }).catch((e: unknown) => e);

    expect((error as NcnApiHttpError).host).toBe("ncn-governance.solana.com");
  });

  it("wraps a network-level TypeError, naming the host", async () => {
    // What Safari actually throws when a cross-origin response fails the CORS check.
    global.fetch = jest
      .fn()
      .mockRejectedValue(
        new TypeError("Load failed"),
      ) as unknown as typeof fetch;

    const error = await fetchNcnJson(URL_UNDER_TEST, {
      label: "snapshot meta info",
      maxRetries: 0,
    }).catch((e: unknown) => e);

    expect(error).toBeInstanceOf(NcnApiNetworkError);
    expect((error as Error).message).toContain("ncn-governance.solana.com");
    expect((error as Error).message).toContain("snapshot meta info");
    expect((error as NcnApiNetworkError).cause).toBeInstanceOf(TypeError);
  });

  it("throws NcnApiNetworkError when the request exceeds timeoutMs", async () => {
    // Never settles on its own; only the internal timeout can abort it.
    global.fetch = jest.fn(
      (_url: unknown, init?: { signal?: AbortSignal }) =>
        new Promise((_resolve, reject) => {
          init?.signal?.addEventListener("abort", () =>
            reject(new DOMException("Aborted", "AbortError")),
          );
        }),
    ) as unknown as typeof fetch;

    const pending = fetchNcnJson(URL_UNDER_TEST, {
      label: "snapshot meta info",
      timeoutMs: 50,
      maxRetries: 0,
    }).catch((e: unknown) => e);

    const error = await pending;

    expect(error).toBeInstanceOf(NcnApiNetworkError);
    expect((error as Error).message).toContain("Timed out after 50ms");
    expect((error as NcnApiNetworkError).host).toBe(
      "ncn-governance.solana.com",
    );
    // The only cause available is our own abort. Attaching it just adds a second, contentless
    // "signal is aborted without reason" exception to the Sentry event.
    expect((error as NcnApiNetworkError).cause).toBeUndefined();
  });

  it("retries a transient operator response and returns the next successful response", async () => {
    const meta = { network: "mainnet", slot: 422497000 };
    global.fetch = jest
      .fn()
      .mockResolvedValueOnce({
        ok: false,
        status: 503,
        statusText: "",
        url: "https://unhealthy-operator.example/meta?network=mainnet",
        text: async () => "unavailable",
      })
      .mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => meta,
      }) as unknown as typeof fetch;

    await expect(
      fetchNcnJson(URL_UNDER_TEST, {
        label: "snapshot meta info",
        retryDelayMs: () => 0,
      }),
    ).resolves.toEqual(meta);
    expect(global.fetch).toHaveBeenCalledTimes(2);
  });

  it("retries after an attempt times out", async () => {
    const meta = { network: "mainnet", slot: 422497000 };
    global.fetch = jest
      .fn()
      .mockImplementationOnce(
        (_url: unknown, init?: { signal?: AbortSignal }) =>
          new Promise((_resolve, reject) => {
            init?.signal?.addEventListener("abort", () =>
              reject(new DOMException("Aborted", "AbortError")),
            );
          }),
      )
      .mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => meta,
      }) as unknown as typeof fetch;

    await expect(
      fetchNcnJson(URL_UNDER_TEST, {
        label: "snapshot meta info",
        timeoutMs: 10,
        retryDelayMs: () => 0,
      }),
    ).resolves.toEqual(meta);
    expect(global.fetch).toHaveBeenCalledTimes(2);
  });

  it("retries a network failure three times before returning the final error", async () => {
    global.fetch = jest
      .fn()
      .mockRejectedValue(
        new TypeError("Load failed"),
      ) as unknown as typeof fetch;

    const error = await fetchNcnJson(URL_UNDER_TEST, {
      label: "snapshot meta info",
      retryDelayMs: () => 0,
    }).catch((e: unknown) => e);

    expect(error).toBeInstanceOf(NcnApiNetworkError);
    expect(global.fetch).toHaveBeenCalledTimes(4);
  });

  it("does not retry a definitive request failure", async () => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: false,
      status: 404,
      statusText: "Not Found",
      url: URL_UNDER_TEST,
      text: async () => "missing",
    }) as unknown as typeof fetch;

    const error = await fetchNcnJson(URL_UNDER_TEST, {
      label: "snapshot meta info",
      retryDelayMs: () => 0,
    }).catch((e: unknown) => e);

    expect(error).toBeInstanceOf(NcnApiHttpError);
    expect((error as NcnApiHttpError).status).toBe(404);
    expect(global.fetch).toHaveBeenCalledTimes(1);
  });

  it("marks a missing stake proof as an expected snapshot outcome", async () => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: false,
      status: 404,
      statusText: "Not Found",
      url: URL_UNDER_TEST,
      text: async () => "missing",
    }) as unknown as typeof fetch;

    const error = await fetchNcnJson(URL_UNDER_TEST, {
      label: "stake account proof",
      resource: "stake-account-proof",
      retryDelayMs: () => 0,
    }).catch((e: unknown) => e);

    expect(error).toBeInstanceOf(NcnApiHttpError);
    expect((error as NcnApiHttpError).resource).toBe("stake-account-proof");
    expect(isNcnProofNotFound(error)).toBe(true);
    expect(global.fetch).toHaveBeenCalledTimes(1);
  });

  it("does not treat non-proof 404s as expected snapshot outcomes", async () => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: false,
      status: 404,
      statusText: "Not Found",
      url: URL_UNDER_TEST,
      text: async () => "missing",
    }) as unknown as typeof fetch;

    const error = await fetchNcnJson(URL_UNDER_TEST, {
      label: "snapshot meta info",
      resource: "snapshot-meta",
      retryDelayMs: () => 0,
    }).catch((e: unknown) => e);

    expect(isNcnProofNotFound(error)).toBe(false);
  });

  it("does not start work when the caller's signal is already aborted", async () => {
    global.fetch = jest.fn(
      (_url: unknown, init?: { signal?: AbortSignal }) =>
        new Promise((resolve, reject) => {
          if (init?.signal?.aborted) {
            reject(new DOMException("Aborted", "AbortError"));
            return;
          }
          resolve({ ok: true, status: 200, json: async () => ({}) });
        }),
    ) as unknown as typeof fetch;

    const error = await fetchNcnJson(URL_UNDER_TEST, {
      label: "snapshot meta info",
      signal: AbortSignal.abort(),
    }).catch((e: unknown) => e);

    expect((error as Error).name).toBe("AbortError");
  });

  it("rethrows the caller's AbortError untouched so React Query sees a cancellation", async () => {
    global.fetch = jest.fn(
      (_url: unknown, init?: { signal?: AbortSignal }) =>
        new Promise((_resolve, reject) => {
          init?.signal?.addEventListener("abort", () =>
            reject(new DOMException("Aborted", "AbortError")),
          );
        }),
    ) as unknown as typeof fetch;

    const controller = new AbortController();
    const pending = fetchNcnJson(URL_UNDER_TEST, {
      label: "snapshot meta info",
      signal: controller.signal,
    }).catch((e: unknown) => e);

    controller.abort();
    const error = await pending;

    expect(error).not.toBeInstanceOf(NcnApiNetworkError);
    expect((error as Error).name).toBe("AbortError");
  });
});

describe("isNetworkFailure", () => {
  it.each([
    ["Load failed", true], // Safari / WebKit
    ["Failed to fetch", true], // Chrome
    ["NetworkError when attempting to fetch resource.", true], // Firefox
    ["fetch failed", true], // Node / undici
    ["Cannot read properties of undefined", false],
  ])("classifies TypeError(%s) as %s", (message, expected) => {
    expect(isNetworkFailure(new TypeError(message))).toBe(expected);
  });

  it("classifies NcnApiNetworkError as a network failure", () => {
    expect(
      isNetworkFailure(new NcnApiNetworkError("unreachable", URL_UNDER_TEST)),
    ).toBe(true);
  });

  it("does not classify an HTTP error as a network failure", () => {
    expect(
      isNetworkFailure(
        new NcnApiHttpError("meta", 503, { url: URL_UNDER_TEST }),
      ),
    ).toBe(false);
  });
});

describe("classifyNcnFailure", () => {
  it.each([
    // A hop refused or shed the request: infrastructure, and a retry may land on a healthy operator.
    [403, "upstream"],
    [408, "upstream"],
    [429, "upstream"],
    [500, "upstream"],
    [502, "upstream"],
    [503, "upstream"],
    [522, "upstream"],
    // The upstream understood us and said no — asking again changes nothing.
    [400, "request"],
    [404, "request"],
    // Unrecognized statuses stay loud rather than being assumed to be someone else's problem.
    [418, "request"],
  ])("classifies HTTP %i as %s", (status, expected) => {
    expect(
      classifyNcnFailure(
        new NcnApiHttpError("meta", status, { url: URL_UNDER_TEST }),
      ),
    ).toBe(expected);
  });

  it.each([
    ["NcnApiNetworkError", new NcnApiNetworkError("nope", URL_UNDER_TEST)],
    ["a network TypeError", new TypeError("Load failed")],
  ])("classifies %s as network", (_label, error) => {
    expect(classifyNcnFailure(error)).toBe("network");
  });

  it.each([
    ["an unrelated Error", new Error("boom")],
    ["a non-Error", "nope"],
  ])("returns undefined for %s", (_label, error) => {
    expect(classifyNcnFailure(error)).toBeUndefined();
  });
});

describe("isPermanentNcnFailure", () => {
  it.each([
    [400, true],
    [404, true],
    [403, false],
    [503, false],
  ])("HTTP %i => %s", (status, expected) => {
    expect(
      isPermanentNcnFailure(
        new NcnApiHttpError("meta", status, { url: URL_UNDER_TEST }),
      ),
    ).toBe(expected);
  });

  it.each([
    ["a network failure", new NcnApiNetworkError("nope", URL_UNDER_TEST)],
    ["an unrelated error", new Error("boom")],
  ])("does not treat %s as permanent", (_label, error) => {
    expect(isPermanentNcnFailure(error)).toBe(false);
  });
});
