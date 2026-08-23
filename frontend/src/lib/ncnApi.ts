/**
 * Shared client for the NCN verifier API (snapshot meta, voter summaries, merkle proofs).
 *
 * `ncn-governance.solana.com` is a router that 302-redirects each request to a randomly
 * chosen verifier operator's own domain, so every call spans two origins and two chances
 * to fail. When either hop fails, the failure often arrives without CORS headers (a
 * Cloudflare 5xx error page carries none), which the browser reports as an opaque
 * `TypeError` rather than a readable status. This module gives those failures a name, a
 * host, and a bounded deadline.
 */

/** Default NCN API base URL. Users can override it via the settings modal (see NcnApiContext). */
export const DEFAULT_NCN_API_URL = "https://ncn-governance.solana.com";

/** The router stalls for ~20s when unhealthy; fail sooner and try another routed operator. */
const DEFAULT_TIMEOUT_MS = 10_000;

/** Three retries matches React Query's former policy: four total attempts per NCN request. */
const DEFAULT_MAX_RETRIES = 3;

const MAX_RETRY_DELAY_MS = 8_000;
const RETRY_JITTER_MS = 250;

/** Enough of an error body to identify who refused us, without shipping a whole HTML page. */
const MAX_BODY_SNIPPET_CHARS = 200;

const hostOf = (url: string): string => {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
};

/** The API answered, but with a non-2xx status. */
export class NcnApiHttpError extends Error {
  readonly status: number;
  /**
   * Host that actually answered. Because the router redirects, this names the verifier operator
   * that refused the request rather than the router we asked.
   */
  readonly host: string;
  /** Start of the response body, or "" when it could not be read. Names which hop refused us. */
  readonly bodySnippet: string;

  constructor(
    label: string,
    status: number,
    {
      url,
      statusText,
      bodySnippet = "",
    }: { url: string; statusText?: string; bodySnippet?: string },
  ) {
    const host = hostOf(url);

    // These endpoints are served over HTTP/2, which has no reason phrase, so statusText is
    // almost always empty. Always include the numeric status.
    super(
      `Failed to get ${label} from ${host}: ${statusText ? `${status} ${statusText}` : status}`,
    );
    this.name = "NcnApiHttpError";
    this.status = status;
    this.host = host;
    this.bodySnippet = bodySnippet;
  }
}

/**
 * No readable response ever arrived: DNS/TLS failure, timeout, refused connection, or a
 * cross-origin response that failed the CORS check.
 */
export class NcnApiNetworkError extends Error {
  readonly url: string;
  /** Host we could not reach, or whose response failed the CORS check. */
  readonly host: string;

  constructor(message: string, url: string, options?: { cause?: unknown }) {
    super(message, options);
    this.name = "NcnApiNetworkError";
    this.url = url;
    this.host = hostOf(url);
  }
}

/**
 * Whether an error is a network-level fetch failure rather than an application error.
 *
 * Browsers reject `fetch` with a bare `TypeError` when the request never completed, and the
 * message is engine specific: "Load failed" (Safari/WebKit), "Failed to fetch" (Chrome),
 * "NetworkError when attempting to fetch resource" (Firefox), "fetch failed" (Node/undici).
 */
export const isNetworkFailure = (error: unknown): boolean => {
  if (error instanceof NcnApiNetworkError) return true;

  return (
    error instanceof TypeError &&
    /load failed|failed to fetch|fetch failed|networkerror|network request failed/i.test(
      error.message,
    )
  );
};

/** Sub-500 statuses that still mean a hop refused or shed the request, not that we asked wrong. */
const UPSTREAM_STATUSES = new Set([403, 408, 429]);

export type NcnFailureKind = "network" | "upstream" | "request";

/**
 * How to treat an NCN API failure, or `undefined` for an error that did not come out of this
 * client (which the caller should report unchanged).
 *
 * `request` is the fallback for unrecognized statuses on purpose: a status meaning our request was
 * wrong has to stay visible, while a new upstream status costs one noisy report before we add it
 * to `UPSTREAM_STATUSES`.
 */
export const classifyNcnFailure = (
  error: unknown,
): NcnFailureKind | undefined => {
  if (isNetworkFailure(error)) return "network";
  if (!(error instanceof NcnApiHttpError)) return undefined;

  return error.status >= 500 || UPSTREAM_STATUSES.has(error.status)
    ? "upstream"
    : "request";
};

/**
 * Whether retrying is pointless because the upstream already answered definitively.
 *
 * The router redirects to a randomly chosen operator per request, so a retry doubles as this app's
 * operator failover: worth spending on a 403 or a 5xx, wasted on a 404 that every operator would
 * repeat.
 */
export const isPermanentNcnFailure = (error: unknown): boolean =>
  classifyNcnFailure(error) === "request";

/**
 * Never throws. The internal timeout is still armed while the body is read, so letting an abort
 * escape here would trade a known status for an opaque AbortError — the exact failure this client
 * exists to prevent.
 */
const readBodySnippet = async (response: Response): Promise<string> => {
  try {
    return (await response.text()).trim().slice(0, MAX_BODY_SNIPPET_CHARS);
  } catch {
    return "";
  }
};

interface FetchNcnJsonOptions {
  /**
   * Pass React Query's `signal` through so unmounting or invalidating the query aborts the
   * request instead of leaving it in flight to fail later.
   */
  signal?: AbortSignal;
  timeoutMs?: number;
  /** Number of retries after the initial request. */
  maxRetries?: number;
  /** Integer-valued response fields that must be returned as exact decimal strings. */
  losslessIntegerFields?: readonly string[];
  /** Override the backoff calculation, primarily for callers with tighter latency budgets. */
  retryDelayMs?: (retryNumber: number) => number;
  /** Human-readable name of the resource, used in error messages. */
  label: string;
}

interface FetchNcnJsonOnceOptions {
  signal?: AbortSignal;
  timeoutMs: number;
  label: string;
  losslessIntegerFields?: readonly string[];
}

/**
 * Quote selected integer tokens before JSON.parse can round them to IEEE-754 doubles.
 *
 * The verifier API currently serializes u64 lamport values as JSON numbers. Validator stake can
 * exceed Number.MAX_SAFE_INTEGER, and rounding even one lamport changes a Merkle leaf hash. Keep
 * this scoped to explicitly named, trusted API fields so ordinary response numbers stay numbers.
 */
const parseNcnJson = <T>(
  body: string,
  losslessIntegerFields: readonly string[],
): T => {
  let normalized = body;

  for (const field of losslessIntegerFields) {
    const escapedField = field.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const integerValue = new RegExp(
      `("${escapedField}"\\s*:\\s*)(-?(?:0|[1-9]\\d*))(?=\\s*[,}])`,
      "g",
    );
    normalized = normalized.replace(integerValue, '$1"$2"');
  }

  return JSON.parse(normalized) as T;
};

const defaultRetryDelayMs = (retryNumber: number): number =>
  Math.min(1000 * 2 ** retryNumber, MAX_RETRY_DELAY_MS) +
  Math.random() * RETRY_JITTER_MS;

const abortError = (): DOMException =>
  new DOMException("The operation was aborted", "AbortError");

/** Wait for the next attempt while still honoring React Query cancellation. */
const waitForRetry = async (
  delayMs: number,
  signal?: AbortSignal,
): Promise<void> => {
  if (signal?.aborted) throw abortError();
  if (delayMs <= 0) return;

  await new Promise<void>((resolve, reject) => {
    const onAbort = () => {
      clearTimeout(timer);
      signal?.removeEventListener("abort", onAbort);
      reject(abortError());
    };
    const timer = setTimeout(() => {
      signal?.removeEventListener("abort", onAbort);
      resolve();
    }, delayMs);

    signal?.addEventListener("abort", onAbort);
  });
};

/** Make one bounded request. Retry orchestration lives in `fetchNcnJson`. */
const fetchNcnJsonOnce = async <T>(
  url: string,
  {
    signal,
    timeoutMs,
    label,
    losslessIntegerFields = [],
  }: FetchNcnJsonOnceOptions,
): Promise<T> => {
  // Composed by hand rather than with AbortSignal.any/AbortSignal.timeout, which need
  // Safari 17.4+ — and Safari users are the ones hitting these failures.
  const controller = new AbortController();
  let timedOut = false;

  const timer = setTimeout(() => {
    timedOut = true;
    controller.abort();
  }, timeoutMs);

  const abortFromCaller = () => controller.abort();
  signal?.addEventListener("abort", abortFromCaller);
  // "abort" does not fire for a signal that was already aborted before we subscribed.
  if (signal?.aborted) controller.abort();

  try {
    const response = await fetch(url, { signal: controller.signal });

    if (!response.ok) {
      throw new NcnApiHttpError(label, response.status, {
        // After the router's redirect this is the operator that actually answered, which is the
        // one fact the status alone never tells us.
        url: response.url || url,
        statusText: response.statusText,
        bodySnippet: await readBodySnippet(response),
      });
    }

    if (losslessIntegerFields.length === 0) {
      return (await response.json()) as T;
    }

    return parseNcnJson<T>(await response.text(), losslessIntegerFields);
  } catch (error) {
    // The caller cancelled us (unmount, invalidation). Rethrow untouched so React Query
    // records a cancellation rather than a failure — cancellations never reach
    // QueryCache.onError, so they are never reported to Sentry.
    if (signal?.aborted) throw error;

    if (timedOut) {
      // No `cause`: the only error reachable here is the DOMException from our own
      // controller.abort(), which Sentry's linkedErrors integration would upload as a second
      // exception ("signal is aborted without reason") saying nothing beyond "we aborted".
      throw new NcnApiNetworkError(
        `Timed out after ${timeoutMs}ms getting ${label} from ${hostOf(url)}`,
        url,
      );
    }

    if (isNetworkFailure(error)) {
      throw new NcnApiNetworkError(
        `NCN API unreachable at ${hostOf(url)} while getting ${label} (network or CORS failure)`,
        url,
        { cause: error },
      );
    }

    throw error;
  } finally {
    clearTimeout(timer);
    signal?.removeEventListener("abort", abortFromCaller);
  }
};

/**
 * Fetch JSON from the NCN API with a bounded timeout and transient-failure retries.
 *
 * Each retry goes back through the configured base URL. With the default router, that lets the
 * next attempt choose another operator. Definite request failures such as a 400 or 404 are returned
 * immediately. Caller cancellation stops both an in-flight request and a pending backoff without
 * starting another attempt.
 */
export async function fetchNcnJson<T>(
  url: string,
  {
    signal,
    timeoutMs = DEFAULT_TIMEOUT_MS,
    maxRetries = DEFAULT_MAX_RETRIES,
    losslessIntegerFields,
    retryDelayMs = defaultRetryDelayMs,
    label,
  }: FetchNcnJsonOptions,
): Promise<T> {
  for (let retryNumber = 0; ; retryNumber += 1) {
    try {
      return await fetchNcnJsonOnce<T>(url, {
        signal,
        timeoutMs,
        label,
        losslessIntegerFields,
      });
    } catch (error) {
      const failureKind = classifyNcnFailure(error);
      const shouldRetry =
        retryNumber < maxRetries &&
        (failureKind === "network" || failureKind === "upstream");

      if (!shouldRetry || signal?.aborted) throw error;
      await waitForRetry(retryDelayMs(retryNumber), signal);
    }
  }
}
