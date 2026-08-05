import React from "react";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useProposalDocument } from "../useProposalDocument";
import { SGP_REPO, SIMD_REPO } from "@/lib/github";

const HEAD_SHA = "27bca51e5c0fc34ddbea6904faf86f5098225316";
const STORAGE_KEY = "proposal_docs_cache_v2";

const SIMD_URL = `https://github.com/${SIMD_REPO}/blob/main/proposals/0022-multi-stake.md`;
const SGP_URL = `https://github.com/${SGP_REPO}/blob/main/proposals/sgp-0001-solana-constitution.md`;
const PR_URL = `https://github.com/${SGP_REPO}/pull/3`;

const SIMD_DOC = `---\nsimd: '0022'\ntitle: Multi Delegation Stake Account\n---\n\n## Summary\n\nMulti delegation, summarized.\n`;
const SGP_DOC = `---\nsgp: 0001\ntitle: The Solana Constitution\n---\n\n## Summary\n\nRatifies the Constitution.\n`;

const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: jest.fn((key: string) => store[key] ?? null),
    setItem: jest.fn((key: string, value: string) => {
      store[key] = value;
    }),
    removeItem: jest.fn((key: string) => {
      delete store[key];
    }),
    clear: jest.fn(() => {
      store = {};
    }),
  };
})();

Object.defineProperty(window, "localStorage", { value: localStorageMock });

global.fetch = jest.fn();
const mockFetch = fetch as jest.Mock;

function textResponse(body: string) {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
    text: async () => body,
    headers: new Headers(),
  };
}

function jsonResponse(body: unknown) {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
    json: async () => body,
    headers: new Headers(),
  };
}

function prFile(filename: string, status: string) {
  return {
    filename,
    status,
    contents_url: `https://api.github.com/repos/${SGP_REPO}/contents/${encodeURIComponent(filename)}?ref=${HEAD_SHA}`,
  };
}

// `retry: false` keeps failure cases from waiting out the real retry backoff.
const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

  const Wrapper = ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
  Wrapper.displayName = "QueryClientWrapper";
  return Wrapper;
};

describe("useProposalDocument", () => {
  beforeEach(() => {
    jest.clearAllMocks();
    localStorage.clear();
  });

  it("fetches a SIMD proposal from the correct raw URL", async () => {
    mockFetch.mockResolvedValueOnce(textResponse(SIMD_DOC));

    const { result } = renderHook(() => useProposalDocument(SIMD_URL), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(mockFetch.mock.calls[0][0]).toBe(
      `https://raw.githubusercontent.com/${SIMD_REPO}/main/proposals/0022-multi-stake.md`,
    );
    expect(result.current.data?.summary).toBe("Multi delegation, summarized.");
    expect(result.current.data?.ref?.label).toBe("SIMD-0022");
    expect(localStorage.setItem).toHaveBeenCalled();
  });

  // Regression test for the hardcoded repo in the previous implementation.
  it("fetches an SGP proposal from the SGP repo, not the SIMD repo", async () => {
    mockFetch.mockResolvedValueOnce(textResponse(SGP_DOC));

    const { result } = renderHook(() => useProposalDocument(SGP_URL), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    const fetchedUrl = mockFetch.mock.calls[0][0] as string;
    expect(fetchedUrl).toContain("solana-governance-proposals");
    expect(fetchedUrl).not.toContain("solana-improvement-documents");
    expect(result.current.data?.ref?.label).toBe("SGP-0001");
  });

  it("resolves a pull request link via the files API and the head SHA", async () => {
    mockFetch
      .mockResolvedValueOnce(
        jsonResponse([
          prFile("README.md", "modified"),
          prFile("XXXX-sgp-template.md", "modified"),
          prFile("proposals/sgp-0001-solana-constitution.md", "added"),
        ]),
      )
      .mockResolvedValueOnce(textResponse(SGP_DOC));

    const { result } = renderHook(() => useProposalDocument(PR_URL), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(mockFetch.mock.calls[0][0]).toBe(
      `https://api.github.com/repos/${SGP_REPO}/pulls/3/files?per_page=100`,
    );
    expect(mockFetch.mock.calls[1][0]).toBe(
      `https://raw.githubusercontent.com/${SGP_REPO}/${HEAD_SHA}/proposals/sgp-0001-solana-constitution.md`,
    );
    expect(result.current.data?.ref?.label).toBe("SGP-0001");
    expect(result.current.data?.summary).toBe("Ratifies the Constitution.");
  });

  it("uses a cached document keyed by URL without fetching", async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        [SIMD_URL]: {
          status: "ok",
          fetchedAt: Date.now(),
          document: {
            ref: { number: "0022", kind: "simd", label: "SIMD-0022" },
            summary: "Cached summary",
            sourceUrl: "https://example.invalid/cached.md",
            fetchedAt: Date.now(),
          },
        },
      }),
    );

    const { result } = renderHook(() => useProposalDocument(SIMD_URL), {
      wrapper: createWrapper(),
    });

    expect(result.current.data?.summary).toBe("Cached summary");
    expect(fetch).not.toHaveBeenCalled();
  });

  // The old implementation wrote the cache under the frontmatter-derived number but read it
  // back under the filename-derived one, so a write was never readable. This is the direct
  // end-to-end check of that round trip.
  it.each([
    ["a blob URL", SIMD_URL, [() => textResponse(SIMD_DOC)]],
    [
      "a pull request URL",
      PR_URL,
      [
        () => jsonResponse([prFile("proposals/sgp-0001-x.md", "added")]),
        () => textResponse(SGP_DOC),
      ],
    ],
  ])("round-trips the cache for %s", async (_label, url, responses) => {
    for (const response of responses as Array<() => unknown>) {
      mockFetch.mockResolvedValueOnce(response());
    }

    const first = renderHook(() => useProposalDocument(url as string), {
      wrapper: createWrapper(),
    });
    await waitFor(() => expect(first.result.current.isSuccess).toBe(true));

    mockFetch.mockClear();

    // A fresh QueryClient means only localStorage can supply the data.
    const second = renderHook(() => useProposalDocument(url as string), {
      wrapper: createWrapper(),
    });

    expect(second.result.current.data?.summary).toBeTruthy();
    expect(fetch).not.toHaveBeenCalled();
  });

  it("returns null without erroring for an unparseable URL", async () => {
    const { result } = renderHook(
      () => useProposalDocument("https://github.com/org/repo/tree/main/proposals"),
      { wrapper: createWrapper() },
    );

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toBeNull();
    expect(result.current.isError).toBe(false);
    expect(fetch).not.toHaveBeenCalled();
  });

  // These use the real retry policy (not the wrapper's `retry: false`) because the point of
  // the test is that a non-retryable GitHub response is not retried.
  const retryingWrapper = () => {
    const queryClient = new QueryClient();
    const Wrapper = ({ children }: { children: React.ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
    Wrapper.displayName = "RetryingQueryClientWrapper";
    return Wrapper;
  };

  it.each([
    ["an exhausted quota", 403, { "x-ratelimit-remaining": "0" }],
    ["a 429", 429, {}],
    ["a secondary rate limit", 403, { "retry-after": "60" }],
  ])("does not retry %s, spending only one request", async (_l, status, headers) => {
    mockFetch.mockResolvedValue({
      ok: false,
      status,
      statusText: "denied",
      headers: new Headers(headers),
    });

    const { result } = renderHook(() => useProposalDocument(PR_URL), {
      wrapper: retryingWrapper(),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));
    expect((result.current.error as Error).name).toBe("GithubApiError");
    expect(mockFetch).toHaveBeenCalledTimes(1);
  });

  it("caches a negative result so a dud link stops costing an API call per load", async () => {
    // A PR that changes no proposal document — the shape that previously re-hit the API on
    // every single page load because only successes were cached.
    mockFetch.mockResolvedValueOnce(
      jsonResponse([prFile("README.md", "modified")]),
    );

    const first = renderHook(() => useProposalDocument(PR_URL), {
      wrapper: createWrapper(),
    });
    await waitFor(() => expect(first.result.current.isSuccess).toBe(true));
    expect(first.result.current.data).toBeNull();
    expect(mockFetch).toHaveBeenCalledTimes(1);

    mockFetch.mockClear();

    // A fresh QueryClient means only localStorage can answer.
    const second = renderHook(() => useProposalDocument(PR_URL), {
      wrapper: createWrapper(),
    });

    expect(second.result.current.data).toBeNull();
    expect(mockFetch).not.toHaveBeenCalled();
  });

  it("re-checks a negative once its shorter TTL expires", async () => {
    const twoHoursAgo = Date.now() - 1000 * 60 * 60 * 2;
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        [PR_URL]: {
          status: "unsupported",
          reason: "Pull request #3 does not change a proposal document",
          fetchedAt: twoHoursAgo,
        },
      }),
    );
    mockFetch
      .mockResolvedValueOnce(
        jsonResponse([prFile("proposals/sgp-0001-x.md", "added")]),
      )
      .mockResolvedValueOnce(textResponse(SGP_DOC));

    const { result } = renderHook(() => useProposalDocument(PR_URL), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.data?.ref?.label).toBe("SGP-0001"));
    expect(mockFetch).toHaveBeenCalled();
  });

  it("does not resurrect a cache entry written in an older shape", async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        [SIMD_URL]: {
          ref: { number: "0022", kind: "simd", label: "SIMD-0022" },
          summary: "Legacy shape",
          fetchedAt: Date.now(),
        },
      }),
    );
    mockFetch.mockResolvedValueOnce(textResponse(SIMD_DOC));

    const { result } = renderHook(() => useProposalDocument(SIMD_URL), {
      wrapper: createWrapper(),
    });

    await waitFor(() =>
      expect(result.current.data?.summary).toBe("Multi delegation, summarized."),
    );
  });

  it("ignores a cache entry that has outlived its TTL", async () => {
    const eightDaysAgo = Date.now() - 1000 * 60 * 60 * 24 * 8;
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        [SIMD_URL]: {
          status: "ok",
          fetchedAt: eightDaysAgo,
          document: {
            ref: { number: "0022", kind: "simd", label: "SIMD-0022" },
            summary: "Stale summary",
            sourceUrl: "https://example.invalid/cached.md",
            fetchedAt: eightDaysAgo,
          },
        },
      }),
    );
    mockFetch.mockResolvedValueOnce(textResponse(SIMD_DOC));

    const { result } = renderHook(() => useProposalDocument(SIMD_URL), {
      wrapper: createWrapper(),
    });

    await waitFor(() =>
      expect(result.current.data?.summary).toBe("Multi delegation, summarized."),
    );
    expect(fetch).toHaveBeenCalled();
  });
});
