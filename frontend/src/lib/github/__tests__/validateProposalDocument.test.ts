import {
  assertValidProposalDocument,
  ProposalDocumentValidationError,
} from "../validateProposalDocument";

const PROPOSAL_URL =
  "https://github.com/solana-foundation/solana-governance-proposals/blob/27bca51e5c0fc34ddbea6904faf86f5098225316/proposals/sgp-0001-title.md";

function waitForAbort<T>(signal: AbortSignal | null | undefined): Promise<T> {
  return new Promise((_resolve, reject) => {
    signal?.addEventListener(
      "abort",
      () => reject(new DOMException("Aborted", "AbortError")),
      { once: true },
    );
  });
}

describe("assertValidProposalDocument", () => {
  const originalFetch = global.fetch;

  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    global.fetch = originalFetch;
    jest.useRealTimers();
  });

  it.each(["request", "response body"] as const)(
    "reports a bypassable timeout when the %s stalls",
    async (stage) => {
      let requestSignal: AbortSignal | null | undefined;
      global.fetch = jest.fn(
        (_input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
          requestSignal = init?.signal;
          if (stage === "request") return waitForAbort<Response>(requestSignal);
          return Promise.resolve({
            ok: true,
            status: 200,
            text: () => waitForAbort<string>(requestSignal),
          } as Response);
        },
      );

      const pending = assertValidProposalDocument(PROPOSAL_URL).catch(
        (error: unknown) => error,
      );
      await jest.advanceTimersByTimeAsync(5_000);

      expect(requestSignal?.aborted).toBe(true);
      const error = await pending;
      expect(error).toBeInstanceOf(ProposalDocumentValidationError);
      expect(error).toMatchObject({ code: "timeout" });
      expect(jest.getTimerCount()).toBe(0);
    },
  );

  it("verifies a valid document and clears the timeout", async () => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      status: 200,
      text: async () => "---\nsgp: 0001\n---\n# Title",
    });

    await expect(
      assertValidProposalDocument(PROPOSAL_URL),
    ).resolves.toBeUndefined();
    expect(jest.getTimerCount()).toBe(0);
  });

  it("preserves the frontmatter failure and clears the timeout", async () => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      status: 200,
      text: async () => "---\n---\nBody\n---",
    });

    await expect(
      assertValidProposalDocument(PROPOSAL_URL),
    ).rejects.toMatchObject({
      code: "missing-frontmatter",
    });
    expect(jest.getTimerCount()).toBe(0);
  });
});
