import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { GET_PROPOSAL_DOCUMENT } from "@/helpers";
import {
  fetchProposalDocument,
  GithubApiError,
  type ProposalDocument,
  type ProposalRef,
} from "@/lib/github";

const STORAGE_KEY = "proposal_docs_cache_v2";
const CACHE_TTL = 1000 * 60 * 60 * 24 * 7; // 7 days

/**
 * Negative results expire far sooner than documents. A pull request can gain its proposal file
 * minutes after the proposal is created, and — more importantly — without caching them at all,
 * a link that resolves to nothing costs a GitHub API call on every single page load, forever.
 * The unauthenticated budget is only 60/hour per IP.
 */
const NEGATIVE_CACHE_TTL = 1000 * 60 * 60; // 1 hour

const MAX_CACHE_ENTRIES = 100;

type CacheEntry =
  | { status: "ok"; document: ProposalDocument; fetchedAt: number }
  | { status: "unsupported"; reason: string; fetchedAt: number };

type DocumentCache = Record<string, CacheEntry>;

/**
 * Resolves a proposal's on-chain `description` URL into its number and summary.
 *
 * Handles both a direct link to the markdown file and a link to the pull request that adds it
 * — the latter needs a GitHub API round-trip, which is why this cannot be synchronous.
 *
 * Returns `null` when the URL cannot name a proposal document. That is a normal outcome, not
 * an error: callers fall back to showing just the raw GitHub link.
 */
export function useProposalDocument(githubUrl: string) {
  const cached = useMemo(() => readCache(githubUrl), [githubUrl]);

  return useQuery<ProposalDocument | null>({
    queryKey: [GET_PROPOSAL_DOCUMENT, githubUrl],
    queryFn: async ({ signal }) => {
      const result = await fetchProposalDocument(githubUrl, { signal });
      const fetchedAt = Date.now();

      if (result.status === "unsupported") {
        writeCache(githubUrl, {
          status: "unsupported",
          reason: result.reason,
          fetchedAt,
        });
        return null;
      }

      writeCache(githubUrl, {
        status: "ok",
        document: result.document,
        fetchedAt,
      });
      return result.document;
    },
    // A negative goes stale much sooner than a resolved document.
    staleTime: (query) =>
      query.state.data === null ? NEGATIVE_CACHE_TTL : CACHE_TTL,
    gcTime: CACHE_TTL * 2,
    // Transport failures are worth another try; an exhausted quota, a secondary rate limit,
    // or denied access are not — retrying those only spends more of the same budget.
    retry: (failureCount, error) => {
      if (error instanceof GithubApiError && !error.retryable) return false;
      return failureCount < 2;
    },
    initialData: cached && (cached.status === "ok" ? cached.document : null),
    // Without this react-query treats a six-day-old cache entry as fresh as of *now*, which
    // combined with `staleTime` means it would never refetch.
    initialDataUpdatedAt: cached?.fetchedAt,
  });
}

/**
 * The proposal's number, preferring the authoritative frontmatter value once it loads.
 *
 * `fallback` is the number parsed synchronously from the URL, which is available immediately
 * for a direct file link and lets the common case render without a flash of "-".
 */
export function useProposalRef(
  githubUrl: string,
  fallback?: ProposalRef,
): ProposalRef | undefined {
  const { data } = useProposalDocument(githubUrl);
  return data?.ref ?? fallback;
}

function readCache(githubUrl: string): CacheEntry | undefined {
  if (typeof window === "undefined") return undefined;

  const entry = loadCache()[cacheKey(githubUrl)];
  // Anything not matching the current entry shape is treated as a miss and overwritten.
  if (entry?.status !== "ok" && entry?.status !== "unsupported") return undefined;

  const ttl = entry.status === "ok" ? CACHE_TTL : NEGATIVE_CACHE_TTL;
  return Date.now() - entry.fetchedAt < ttl ? entry : undefined;
}

function writeCache(githubUrl: string, entry: CacheEntry): void {
  if (typeof window === "undefined") return;
  const cache = loadCache();
  cache[cacheKey(githubUrl)] = entry;
  saveCache(evictOldest(cache));
}

/**
 * Keyed on the description URL rather than the proposal number.
 *
 * The URL is the only value available on both the read and the write path — the previous
 * implementation wrote under the frontmatter-derived number but read under the
 * filename-derived one, so entries were effectively unreachable, and a pull-request link has
 * no filename to derive from at all.
 */
function cacheKey(githubUrl: string): string {
  return githubUrl.trim();
}

function evictOldest(cache: DocumentCache): DocumentCache {
  const entries = Object.entries(cache);
  if (entries.length <= MAX_CACHE_ENTRIES) return cache;
  // URL keys grow without bound where the old numeric keys did not.
  entries.sort((a, b) => b[1].fetchedAt - a[1].fetchedAt);
  return Object.fromEntries(entries.slice(0, MAX_CACHE_ENTRIES));
}

function loadCache(): DocumentCache {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as DocumentCache) : {};
  } catch {
    return {};
  }
}

function saveCache(cache: DocumentCache): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(cache));
  } catch {
    // ignore quota errors
  }
}
