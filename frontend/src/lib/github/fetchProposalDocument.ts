import {
  pickProposalFile,
  type PullRequestFile,
} from "./pickProposalFile";
import { parseProposalMarkdown } from "./proposalMarkdown";
import {
  parseProposalUrl,
  rawContentUrl,
  resolveRepoConfig,
  type GithubRepoRef,
  type ProposalRef,
} from "./proposalUrl";

export interface ProposalDocument {
  ref: ProposalRef | undefined;
  summary: string;
  /** The raw.githubusercontent.com URL the markdown actually came from. */
  sourceUrl: string;
  fetchedAt: number;
}

export type ProposalDocumentResult =
  | { status: "ok"; document: ProposalDocument }
  /** Terminal: the description cannot resolve to a document. Retrying will not help. */
  | { status: "unsupported"; reason: string };

/**
 * A non-2xx response from api.github.com.
 *
 * `retryable` is false when another attempt cannot change the outcome — an exhausted quota, a
 * secondary rate limit, or denied access. Retrying those spends the very budget that is
 * already short, and GitHub's secondary limit specifically escalates when hammered.
 */
export class GithubApiError extends Error {
  readonly status: number;
  readonly retryable: boolean;

  constructor(status: number, message: string, retryable: boolean) {
    super(message);
    this.name = "GithubApiError";
    this.status = status;
    this.retryable = retryable;
  }
}

export interface FetchProposalDocumentOptions {
  signal?: AbortSignal;
  /**
   * Injection seam. Swapping in a server-route-backed fetch is the whole migration if the
   * unauthenticated per-IP rate limit ever becomes a problem.
   */
  fetchImpl?: typeof fetch;
}

const GITHUB_API_HEADERS = {
  Accept: "application/vnd.github+json",
  "X-GitHub-Api-Version": "2022-11-28",
} as const;

/** Realistic proposal PRs are far smaller; the cap is a runaway guard, and it is logged. */
const MAX_PULL_FILE_PAGES = 3;

/**
 * Resolves a proposal's on-chain `description` URL into its rendered content.
 *
 * Failure is split deliberately. Transport and server errors THROW, so react-query retries
 * them and `isError` means something. A URL that simply cannot name a proposal returns
 * `unsupported`, which is terminal and stays out of Sentry — the previous implementation
 * collapsed both into `console.warn` + `null`, so a GitHub blip was indistinguishable from a
 * malformed link and nothing was ever retried.
 */
export async function fetchProposalDocument(
  url: string,
  options: FetchProposalDocumentOptions = {},
): Promise<ProposalDocumentResult> {
  const doFetch = options.fetchImpl ?? fetch;
  const parsed = parseProposalUrl(url);

  if (parsed.kind === "unsupported") {
    return { status: "unsupported", reason: parsed.reason };
  }

  if (parsed.kind === "blob") {
    const sourceUrl = rawContentUrl(parsed.repo, parsed.gitRef, parsed.path);
    const text = await fetchRawMarkdown(sourceUrl, doFetch, options.signal);
    if (text === undefined) {
      return { status: "unsupported", reason: `File not found: ${url}` };
    }
    return buildDocument(text, parsed.ref, sourceUrl);
  }

  const config = resolveRepoConfig(parsed.repo);
  const files = await listPullRequestFiles(
    parsed.repo,
    parsed.pullNumber,
    doFetch,
    options.signal,
  );

  if (!files) {
    return {
      status: "unsupported",
      reason: `Pull request #${parsed.pullNumber} was not found`,
    };
  }

  const picked = pickProposalFile(files, config);

  if (picked.status === "none") {
    return {
      status: "unsupported",
      reason: `Pull request #${parsed.pullNumber} does not change a proposal document`,
    };
  }

  if (picked.status === "ambiguous") {
    return {
      status: "unsupported",
      reason: `Pull request #${parsed.pullNumber} changes ${picked.paths.length} proposal documents (${picked.paths.join(", ")}), so the description does not identify one`,
    };
  }

  // The head may live on a fork, but GitHub serves fork commits from the base repo's object
  // network, so the base repo path at the head SHA resolves for both cases.
  const sourceUrl = rawContentUrl(
    parsed.repo,
    picked.file.headSha,
    picked.file.path,
  );
  const text = await fetchRawMarkdown(sourceUrl, doFetch, options.signal);
  if (text === undefined) {
    return {
      status: "unsupported",
      reason: `Proposal file from pull request #${parsed.pullNumber} is no longer available`,
    };
  }
  return buildDocument(text, picked.file.ref, sourceUrl);
}

function buildDocument(
  text: string,
  fallbackRef: ProposalRef | undefined,
  sourceUrl: string,
): ProposalDocumentResult {
  const { ref, summary } = parseProposalMarkdown(text, fallbackRef);
  return {
    status: "ok",
    document: { ref, summary, sourceUrl, fetchedAt: Date.now() },
  };
}

/** Returns `undefined` on 404 (terminal); throws on anything else that is not a 2xx. */
async function fetchRawMarkdown(
  sourceUrl: string,
  doFetch: typeof fetch,
  signal?: AbortSignal,
): Promise<string | undefined> {
  const response = await doFetch(sourceUrl, { signal });
  if (response.status === 404) return undefined;
  if (!response.ok) {
    throw new Error(
      `Failed to fetch ${sourceUrl}: ${response.status} ${response.statusText}`,
    );
  }
  return response.text();
}

/** Returns `undefined` when the pull request does not exist — terminal, and worth caching. */
async function listPullRequestFiles(
  repo: GithubRepoRef,
  pullNumber: number,
  doFetch: typeof fetch,
  signal?: AbortSignal,
): Promise<PullRequestFile[] | undefined> {
  const files: PullRequestFile[] = [];
  let next: string | undefined =
    `https://api.github.com/repos/${repo.owner}/${repo.repo}/pulls/${pullNumber}/files?per_page=100`;

  for (let page = 0; page < MAX_PULL_FILE_PAGES && next; page += 1) {
    const response = await doFetch(next, {
      headers: GITHUB_API_HEADERS,
      signal,
    });

    if (response.status === 404 && page === 0) return undefined;

    if (!response.ok) {
      throw errorForApiResponse(response, pullNumber);
    }

    const batch = (await response.json()) as PullRequestFile[];
    files.push(...(Array.isArray(batch) ? batch : []));
    next = nextPageUrl(response.headers.get("link"));
  }

  if (next) {
    console.warn(
      `Pull request #${pullNumber} has more changed files than we page through; stopped after ${MAX_PULL_FILE_PAGES} pages`,
    );
  }

  return files;
}

function errorForApiResponse(
  response: Response,
  pullNumber: number,
): GithubApiError {
  const { status } = response;
  const remaining = response.headers.get("x-ratelimit-remaining");
  const retryAfter = response.headers.get("retry-after");

  if (status === 429 || (status === 403 && remaining === "0")) {
    return new GithubApiError(
      status,
      "GitHub API rate limit exceeded (60 requests/hour for unauthenticated clients)",
      false,
    );
  }

  // A secondary (abuse) rate limit also answers 403, but reports remaining quota and a
  // Retry-After instead — so it has to be recognized separately or it would be retried.
  if (status === 403 && retryAfter) {
    return new GithubApiError(
      status,
      `GitHub secondary rate limit reached; retry after ${retryAfter}s`,
      false,
    );
  }

  if (status === 403 || status === 404) {
    return new GithubApiError(
      status,
      `GitHub returned ${status} for pull request #${pullNumber}; the repository may be private or removed`,
      false,
    );
  }

  return new GithubApiError(
    status,
    `Failed to list files for pull request #${pullNumber}: ${status} ${response.statusText}`,
    true,
  );
}

/** Minimal RFC 5988 `Link` header reader — we only ever want `rel="next"`. */
function nextPageUrl(linkHeader: string | null): string | undefined {
  if (!linkHeader) return undefined;
  for (const part of linkHeader.split(",")) {
    const match = part.match(/<([^>]+)>\s*;\s*rel="next"/);
    if (match) return match[1];
  }
  return undefined;
}
