import { fetchProposalDocument, GithubApiError } from "../fetchProposalDocument";
import { SGP_REPO, SIMD_REPO } from "../proposalUrl";

const HEAD_SHA = "27bca51e5c0fc34ddbea6904faf86f5098225316";

const SIMD_DOC = `---
simd: '0022'
title: Multi Delegation Stake Account
---

## Summary

Multi delegation, summarized.
`;

const SGP_DOC = `---
sgp: 0001
title: The Solana Constitution
---

## Summary

Ratifies the Constitution.
`;

function textResponse(body: string): Response {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
    text: async () => body,
    headers: new Headers(),
  } as unknown as Response;
}

function jsonResponse(body: unknown, headers: Record<string, string> = {}): Response {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
    json: async () => body,
    headers: new Headers(headers),
  } as unknown as Response;
}

function errorResponse(
  status: number,
  headers: Record<string, string> = {},
): Response {
  return {
    ok: false,
    status,
    statusText: "Error",
    headers: new Headers(headers),
  } as unknown as Response;
}

function prFile(filename: string, status: string, repo = SGP_REPO) {
  return {
    filename,
    status,
    contents_url: `https://api.github.com/repos/${repo}/contents/${encodeURIComponent(filename)}?ref=${HEAD_SHA}`,
  };
}

describe("fetchProposalDocument - blob URLs", () => {
  it("fetches a SIMD proposal from the correct raw URL", async () => {
    const fetchImpl = jest.fn().mockResolvedValue(textResponse(SIMD_DOC));

    const result = await fetchProposalDocument(
      `https://github.com/${SIMD_REPO}/blob/main/proposals/0022-multi-stake.md`,
      { fetchImpl },
    );

    expect(fetchImpl).toHaveBeenCalledTimes(1);
    expect(fetchImpl.mock.calls[0][0]).toBe(
      `https://raw.githubusercontent.com/${SIMD_REPO}/main/proposals/0022-multi-stake.md`,
    );
    expect(result).toMatchObject({
      status: "ok",
      document: {
        ref: { label: "SIMD-0022" },
        summary: "Multi delegation, summarized.",
      },
    });
  });

  // Regression test: the previous implementation hardcoded the raw URL to
  // solana-improvement-documents and ignored the owner/repo in the input, so an SGP blob
  // link silently fetched the wrong repository.
  it("honors the owner/repo in the URL instead of assuming the SIMD repo", async () => {
    const fetchImpl = jest.fn().mockResolvedValue(textResponse(SGP_DOC));

    const result = await fetchProposalDocument(
      `https://github.com/${SGP_REPO}/blob/main/proposals/sgp-0001-solana-constitution.md`,
      { fetchImpl },
    );

    expect(fetchImpl.mock.calls[0][0]).toBe(
      `https://raw.githubusercontent.com/${SGP_REPO}/main/proposals/sgp-0001-solana-constitution.md`,
    );
    expect(fetchImpl.mock.calls[0][0]).not.toContain(
      "solana-improvement-documents",
    );
    expect(result).toMatchObject({
      status: "ok",
      document: { ref: { label: "SGP-0001" } },
    });
  });

  it("treats a 404 as terminal rather than throwing", async () => {
    const fetchImpl = jest.fn().mockResolvedValue(errorResponse(404));

    await expect(
      fetchProposalDocument(
        `https://github.com/${SIMD_REPO}/blob/main/proposals/0022-multi-stake.md`,
        { fetchImpl },
      ),
    ).resolves.toMatchObject({ status: "unsupported" });
  });

  it("throws on a server error so the query can retry", async () => {
    const fetchImpl = jest.fn().mockResolvedValue(errorResponse(502));

    await expect(
      fetchProposalDocument(
        `https://github.com/${SIMD_REPO}/blob/main/proposals/0022-multi-stake.md`,
        { fetchImpl },
      ),
    ).rejects.toThrow("502");
  });
});

describe("fetchProposalDocument - pull request URLs", () => {
  it("lists PR files, picks the proposal, and fetches it at the head SHA", async () => {
    const fetchImpl = jest
      .fn()
      .mockResolvedValueOnce(
        jsonResponse([
          prFile("README.md", "modified"),
          prFile("XXXX-sgp-template.md", "modified"),
          prFile("proposals/sgp-0001-solana-constitution.md", "added"),
        ]),
      )
      .mockResolvedValueOnce(textResponse(SGP_DOC));

    const result = await fetchProposalDocument(
      `https://github.com/${SGP_REPO}/pull/3`,
      { fetchImpl },
    );

    expect(fetchImpl).toHaveBeenCalledTimes(2);
    expect(fetchImpl.mock.calls[0][0]).toBe(
      `https://api.github.com/repos/${SGP_REPO}/pulls/3/files?per_page=100`,
    );
    expect(fetchImpl.mock.calls[0][1]).toMatchObject({
      headers: { Accept: "application/vnd.github+json" },
    });
    expect(fetchImpl.mock.calls[1][0]).toBe(
      `https://raw.githubusercontent.com/${SGP_REPO}/${HEAD_SHA}/proposals/sgp-0001-solana-constitution.md`,
    );
    expect(result).toMatchObject({
      status: "ok",
      document: {
        ref: { number: "0001", kind: "sgp", label: "SGP-0001" },
        summary: "Ratifies the Constitution.",
      },
    });
  });

  it("accepts the /files form of a PR URL", async () => {
    const fetchImpl = jest
      .fn()
      .mockResolvedValueOnce(
        jsonResponse([prFile("proposals/sgp-0001-x.md", "added")]),
      )
      .mockResolvedValueOnce(textResponse(SGP_DOC));

    await expect(
      fetchProposalDocument(`https://github.com/${SGP_REPO}/pull/3/files`, {
        fetchImpl,
      }),
    ).resolves.toMatchObject({ status: "ok" });
  });

  it("returns unsupported when a PR changes no proposal document", async () => {
    const fetchImpl = jest
      .fn()
      .mockResolvedValueOnce(jsonResponse([prFile("README.md", "modified")]));

    const result = await fetchProposalDocument(
      `https://github.com/${SGP_REPO}/pull/6`,
      { fetchImpl },
    );

    expect(result).toEqual({
      status: "unsupported",
      reason: "Pull request #6 does not change a proposal document",
    });
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });

  it("returns unsupported when a PR changes several proposal documents", async () => {
    const fetchImpl = jest
      .fn()
      .mockResolvedValueOnce(
        jsonResponse([
          prFile("proposals/sgp-0004-a.md", "added"),
          prFile("proposals/sgp-0009-b.md", "added"),
        ]),
      );

    const result = await fetchProposalDocument(
      `https://github.com/${SGP_REPO}/pull/3`,
      { fetchImpl },
    );

    // Better to show nothing than to attach one proposal's summary to another's vote.
    expect(result).toMatchObject({ status: "unsupported" });
    expect((result as { reason: string }).reason).toContain(
      "does not identify one",
    );
    // Never reaches the raw fetch.
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });

  it("returns unsupported for a pull request that does not exist", async () => {
    const fetchImpl = jest.fn().mockResolvedValue(errorResponse(404));

    const result = await fetchProposalDocument(
      `https://github.com/${SGP_REPO}/pull/999`,
      { fetchImpl },
    );

    // Terminal rather than an error, so the hook can cache it and stop re-asking.
    expect(result).toEqual({
      status: "unsupported",
      reason: "Pull request #999 was not found",
    });
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });

  // Every 403/429 shape must be non-retryable: retrying spends the same scarce budget, and
  // GitHub's secondary limit escalates when hammered.
  it.each([
    [
      "an exhausted quota",
      { "x-ratelimit-remaining": "0" },
      403,
      /rate limit exceeded/,
    ],
    ["a 429", {}, 429, /rate limit exceeded/],
    [
      "a secondary rate limit",
      { "retry-after": "60", "x-ratelimit-remaining": "42" },
      403,
      /secondary rate limit/,
    ],
    [
      "a denied private repository",
      { "x-ratelimit-remaining": "42" },
      403,
      /may be private/,
    ],
  ])("marks %s as non-retryable", async (_label, headers, status, message) => {
    const fetchImpl = jest.fn().mockResolvedValue(errorResponse(status, headers));

    const error = await fetchProposalDocument(
      `https://github.com/${SGP_REPO}/pull/3`,
      { fetchImpl },
    ).catch((e) => e);

    expect(error).toBeInstanceOf(GithubApiError);
    expect(error.retryable).toBe(false);
    expect(error.status).toBe(status);
    expect(error.message).toMatch(message);
  });

  it("marks a server error as retryable", async () => {
    const fetchImpl = jest.fn().mockResolvedValue(errorResponse(500));

    const error = await fetchProposalDocument(
      `https://github.com/${SGP_REPO}/pull/3`,
      { fetchImpl },
    ).catch((e) => e);

    expect(error).toBeInstanceOf(GithubApiError);
    expect(error.retryable).toBe(true);
  });

  it("follows the Link rel=next header across pages", async () => {
    const page2 = `https://api.github.com/repos/${SGP_REPO}/pulls/3/files?per_page=100&page=2`;
    const fetchImpl = jest
      .fn()
      .mockResolvedValueOnce(
        jsonResponse([prFile("README.md", "modified")], {
          link: `<${page2}>; rel="next"`,
        }),
      )
      .mockResolvedValueOnce(
        jsonResponse([prFile("proposals/sgp-0001-x.md", "added")]),
      )
      .mockResolvedValueOnce(textResponse(SGP_DOC));

    const result = await fetchProposalDocument(
      `https://github.com/${SGP_REPO}/pull/3`,
      { fetchImpl },
    );

    expect(fetchImpl.mock.calls[1][0]).toBe(page2);
    expect(result).toMatchObject({ status: "ok" });
  });
});

describe("fetchProposalDocument - unsupported URLs", () => {
  it.each([
    ["https://github.com/org/repo/tree/main/proposals"],
    ["https://gitlab.com/org/repo/blob/main/proposals/0022-x.md"],
    ["not a url"],
    [""],
  ])("returns unsupported for %j without fetching", async (url) => {
    const fetchImpl = jest.fn();
    const result = await fetchProposalDocument(url, { fetchImpl });
    expect(result.status).toBe("unsupported");
    expect(fetchImpl).not.toHaveBeenCalled();
  });
});
