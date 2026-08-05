/**
 * Parsing for the GitHub URLs stored in a proposal's on-chain `description` field.
 *
 * Two repos are in play, with different file-naming conventions:
 *   - solana-improvement-documents  ->  proposals/0022-multi-stake.md          (frontmatter `simd:`)
 *   - solana-governance-proposals   ->  proposals/sgp-0001-constitution.md     (frontmatter `sgp:`)
 *
 * Descriptions may point at a blob URL (the intended form) or at a pull request. PR links
 * happen because `solana-governance-proposals` has no merged proposals yet — every SGP lives
 * only on an open PR — so a PR link was the only link available. Resolving one requires a
 * network round-trip, so this module classifies it and `fetchProposalDocument` does the rest.
 *
 * Everything here is pure and never throws.
 */

export type ProposalNumberKind = "simd" | "sgp";

export interface GithubRepoRef {
  owner: string;
  repo: string;
}

export interface ProposalRef {
  /** Digits exactly as written, leading zeros preserved: "0022", "0001". */
  number: string;
  kind: ProposalNumberKind;
  /** Display form: "SIMD-0022" | "SGP-0001". */
  label: string;
}

/** Machine-readable reason a URL could not be classified. Consumed by `validateProposalUrl`. */
export type UnsupportedUrlCode =
  | "empty"
  | "not-a-url"
  | "not-https"
  | "not-github"
  | "tree-or-directory"
  | "unrecognized";

export type ParsedProposalUrl =
  | {
      kind: "blob";
      repo: GithubRepoRef;
      /** Branch, tag, or 40-hex commit SHA. */
      gitRef: string;
      /** Repo-relative, percent-decoded. */
      path: string;
      /** Basename of `path`. */
      fileName: string;
      /** Parsed from `fileName` per the repo's rules; `undefined` if it matches none. */
      ref: ProposalRef | undefined;
    }
  | {
      kind: "pull";
      /** The BASE repo. A fork's head is discovered later via the PR files API. */
      repo: GithubRepoRef;
      pullNumber: number;
    }
  | { kind: "unsupported"; code: UnsupportedUrlCode; reason: string };

export interface ProposalFileRule {
  kind: ProposalNumberKind;
  /** Tested against the BASENAME; must capture the number digits in group 1. */
  pattern: RegExp;
}

export interface ProposalRepoConfig {
  /**
   * Repo-relative directory proposal docs live in; `undefined` means any directory.
   *
   * Deliberately enforced ONLY when choosing among the files in a pull request, never for a
   * blob URL. A blob URL names the one exact file the proposer chose, so a directory check
   * there would only add a failure mode; PR file selection, by contrast, IS a filtering
   * problem — it is what excludes root-level `XXXX-sgp-template.md` and `README.md`.
   */
  proposalDir: string | undefined;
  /** Ordered; first match wins. */
  fileRules: ProposalFileRule[];
  /** Used when nothing else identifies the flavour. */
  defaultKind: ProposalNumberKind;
}

const SIMD_RULE: ProposalFileRule = {
  kind: "simd",
  pattern: /^(\d{1,5})(?:[-_.].*)?\.md$/i,
};

const SGP_RULE: ProposalFileRule = {
  kind: "sgp",
  pattern: /^sgp-(\d{1,5})(?:[-_.].*)?\.md$/i,
};

export const SIMD_REPO = "solana-foundation/solana-improvement-documents";
export const SGP_REPO = "solana-foundation/solana-governance-proposals";

const REPO_CONFIGS: Record<string, ProposalRepoConfig> = {
  [SIMD_REPO]: {
    proposalDir: "proposals",
    fileRules: [SIMD_RULE],
    defaultKind: "simd",
  },
  [SGP_REPO]: {
    proposalDir: "proposals",
    fileRules: [SGP_RULE],
    defaultKind: "sgp",
  },
};

/** Unknown repos accept either naming scheme in any directory, so test/fork repos still work. */
const DEFAULT_REPO_CONFIG: ProposalRepoConfig = {
  proposalDir: undefined,
  fileRules: [SGP_RULE, SIMD_RULE],
  defaultKind: "simd",
};

export function isKnownProposalRepo(repo: GithubRepoRef): boolean {
  return repoKey(repo) in REPO_CONFIGS;
}

export function resolveRepoConfig(repo: GithubRepoRef): ProposalRepoConfig {
  return REPO_CONFIGS[repoKey(repo)] ?? DEFAULT_REPO_CONFIG;
}

function repoKey(repo: GithubRepoRef): string {
  return `${repo.owner.toLowerCase()}/${repo.repo.toLowerCase()}`;
}

export function makeProposalRef(
  number: string,
  kind: ProposalNumberKind,
): ProposalRef {
  return { number, kind, label: `${kind.toUpperCase()}-${number}` };
}

export function proposalRefFromFileName(
  fileName: string,
  config: ProposalRepoConfig,
): ProposalRef | undefined {
  for (const rule of config.fileRules) {
    const match = fileName.match(rule.pattern);
    if (match) return makeProposalRef(match[1], rule.kind);
  }
  return undefined;
}

/** Percent-encodes each path segment. Safe for both blob and PR-resolved fetches. */
export function rawContentUrl(
  repo: GithubRepoRef,
  gitRef: string,
  path: string,
): string {
  const encodedPath = path.split("/").map(encodeURIComponent).join("/");
  return `https://raw.githubusercontent.com/${encodeURIComponent(repo.owner)}/${encodeURIComponent(repo.repo)}/${encodeURIComponent(gitRef)}/${encodedPath}`;
}

function unsupported(
  code: UnsupportedUrlCode,
  reason: string,
): ParsedProposalUrl {
  return { kind: "unsupported", code, reason };
}

/**
 * Classifies a GitHub URL. Never throws.
 *
 * Known limitation: a branch name containing `/` (e.g. `blob/feat/x/proposals/y.md`) cannot be
 * split from the file path without an API call, so a single-segment ref is assumed. Every real
 * proposal link satisfies this.
 */
export function parseProposalUrl(rawUrl: string): ParsedProposalUrl {
  const trimmed = rawUrl?.trim() ?? "";
  if (!trimmed) return unsupported("empty", "No URL provided");

  let url: URL;
  try {
    url = new URL(trimmed);
  } catch {
    return unsupported("not-a-url", `Not a valid URL: ${trimmed}`);
  }

  if (url.protocol !== "https:") {
    return unsupported("not-https", "URL must use https");
  }

  const host = url.hostname.toLowerCase().replace(/^www\./, "");
  const segments = url.pathname.split("/").filter(Boolean).map(decodeSegment);

  if (host === "raw.githubusercontent.com") {
    // /{owner}/{repo}/{ref}/{...path}
    if (segments.length < 4) {
      return unsupported("unrecognized", "Not a raw file URL");
    }
    const [owner, repo, gitRef, ...pathParts] = segments;
    return blobResult({ owner, repo }, gitRef, pathParts);
  }

  if (host !== "github.com") {
    return unsupported("not-github", `Not a github.com URL: ${url.hostname}`);
  }

  if (segments.length < 2) {
    return unsupported("unrecognized", "URL does not name a repository");
  }

  const [owner, repo, kind, ...rest] = segments;

  if (kind === "blob" || kind === "raw") {
    if (rest.length < 2) {
      return unsupported("unrecognized", "URL does not name a file");
    }
    const [gitRef, ...pathParts] = rest;
    return blobResult({ owner, repo }, gitRef, pathParts);
  }

  if (kind === "tree") {
    return unsupported(
      "tree-or-directory",
      "URL points at a directory listing, not a file",
    );
  }

  if (kind === "pull" || kind === "pulls") {
    const pullNumber = Number(rest[0]);
    if (!Number.isInteger(pullNumber) || pullNumber <= 0) {
      return unsupported("unrecognized", "Pull request URL has no number");
    }
    return { kind: "pull", repo: { owner, repo }, pullNumber };
  }

  return unsupported(
    "unrecognized",
    kind
      ? `Unrecognized GitHub URL type: /${kind}/`
      : "URL points at a repository root, not a file",
  );
}

function blobResult(
  repo: GithubRepoRef,
  gitRef: string,
  pathParts: string[],
): ParsedProposalUrl {
  const path = pathParts.join("/");
  const fileName = pathParts[pathParts.length - 1] ?? "";
  return {
    kind: "blob",
    repo,
    gitRef,
    path,
    fileName,
    ref: proposalRefFromFileName(fileName, resolveRepoConfig(repo)),
  };
}

function decodeSegment(segment: string): string {
  try {
    return decodeURIComponent(segment);
  } catch {
    return segment;
  }
}

/**
 * Best-effort proposal number, resolved synchronously from the URL alone.
 *
 * Returns `undefined` for pull-request links — those need a network round-trip, which
 * `useProposalDocument` performs.
 */
export function getProposalRefFromUrl(url: string): ProposalRef | undefined {
  const parsed = parseProposalUrl(url);
  return parsed.kind === "blob" ? parsed.ref : undefined;
}
