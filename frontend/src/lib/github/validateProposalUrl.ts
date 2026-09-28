import {
  parseProposalUrl,
  SGP_REPO,
  type ParsedProposalUrl,
} from "./proposalUrl";

export type ProposalUrlErrorCode =
  | "empty"
  | "not-a-url"
  | "not-https"
  | "not-github"
  | "not-allowed-repo"
  | "pull-request"
  | "tree-or-directory"
  | "not-markdown"
  | "not-commit-sha"
  | "query-or-fragment"
  | "too-long"
  | "rejected-on-chain"
  | "unsupported";

export type ProposalUrlWarningCode = "unrecognized-filename";

export interface ProposalUrlIssue<Code extends string> {
  code: Code;
  message: string;
}

export interface ProposalUrlValidation {
  ok: boolean;
  errors: ProposalUrlIssue<ProposalUrlErrorCode>[];
  warnings: ProposalUrlIssue<ProposalUrlWarningCode>[];
  parsed: ParsedProposalUrl;
  /**
   * The exact string that was validated, and the one that must be sent on chain.
   */
  normalized: string;
}

/**
 * Soft mirror of the program's `global_config.max_description_length`. That value is
 * configurable on chain, so this is a client-side courtesy check, not the authority.
 */
const MAX_DESCRIPTION_BYTES = 200;

const COMMIT_SHA = /^[0-9a-f]{40}$/i;

/**
 * The exact prefix the on-chain validator requires. A `www.` host or a
 * raw.githubusercontent.com link resolves fine in a browser but is rejected on chain, so it
 * has to be caught here rather than at transaction time.
 */
const ON_CHAIN_PREFIX = "https://github.com/";

/**
 * Character class the on-chain validator allows in the path, plus `/` as the separator.
 *
 * This must remain ASCII-only: Rust's `is_ascii_alphanumeric` rejects Unicode letters and
 * digits that JavaScript's Unicode property classes would otherwise accept.
 */
const ON_CHAIN_DISALLOWED_CHAR = /[^A-Za-z0-9\-_./]/;

const ON_CHAIN_MIN_SEGMENTS = 5;
const ON_CHAIN_MAX_SEGMENTS = 10;

const PULL_REQUEST_MESSAGE = [
  "Link to the proposal markdown file, not to a pull request.",
  "",
  'Open the PR\'s "Files changed" tab, click the proposal .md file, and copy its URL — it looks like',
  "https://github.com/<owner>/<repo>/blob/<commit-sha>/proposals/sgp-0001-....md",
  "",
  "Prefer the commit SHA over a branch name: the description is stored on chain and cannot be",
  "tied to a particular revision, so a branch link breaks once the branch moves or is deleted.",
].join("\n");

/**
 * Validates a proposal description URL before it is written on chain.
 *
 * Requires an HTTPS GitHub `blob` URL to one Markdown file in the approved repository at a
 * full commit SHA. Rejects empty, oversized, mutable, malformed, pull-request, directory,
 * query/fragment, and on-chain-incompatible URLs. It warns when the filename cannot provide a
 * proposal reference for display.
 */
export function validateProposalUrl(url: string): ProposalUrlValidation {
  const errors: ProposalUrlIssue<ProposalUrlErrorCode>[] = [];
  const warnings: ProposalUrlIssue<ProposalUrlWarningCode>[] = [];
  const value = url ?? "";
  const parsed = parseProposalUrl(value);

  const fail = (code: ProposalUrlErrorCode, message: string) => {
    errors.push({ code, message });
    return { ok: false, errors, warnings, parsed, normalized: value };
  };

  // `parseProposalUrl` distinguishes pull requests and unsupported URL shapes.
  if (parsed.kind === "pull") {
    return fail("pull-request", PULL_REQUEST_MESSAGE);
  }

  if (parsed.kind === "unsupported") {
    switch (parsed.code) {
      case "empty":
        return fail("empty", "A GitHub link is required.");
      case "not-a-url":
        return fail("not-a-url", "This is not a valid URL.");
      case "not-https":
        return fail("not-https", "The link must start with https://.");
      case "not-github":
        return fail("not-github", "The link must point at github.com.");
      case "tree-or-directory":
        return fail(
          "tree-or-directory",
          "This links to a directory. Link to the proposal markdown file itself.",
        );
      default:
        return fail(
          "unsupported",
          "Link to a file on GitHub, e.g. https://github.com/<owner>/<repo>/blob/<ref>/proposals/sgp-0001-....md",
        );
    }
  }

  // `parseProposalUrl` is deliberately lenient about the host so that existing
  // on-chain descriptions still render. Creation has to be stricter than that.
  if (!value.startsWith(ON_CHAIN_PREFIX)) {
    return fail(
      "not-github",
      `The link must start with ${ON_CHAIN_PREFIX} — no "www.", and not raw.githubusercontent.com.`,
    );
  }

  if (parsed.fileName.length <= 3 || !/\.md$/i.test(parsed.fileName)) {
    errors.push({
      code: "not-markdown",
      message: "The link must point at a .md file.",
    });
  }

  if (/[?#]/.test(value)) {
    errors.push({
      code: "query-or-fragment",
      message:
        "Remove the query string or #fragment — the on-chain program rejects them.",
    });
  }

  if (byteLength(value) > MAX_DESCRIPTION_BYTES) {
    errors.push({
      code: "too-long",
      message: `The link must be at most ${MAX_DESCRIPTION_BYTES} bytes.`,
    });
  }

  // Re-check the on-chain grammar directly rather than assuming the shape above implies it,
  // so anything accepted here is guaranteed to be accepted by the program.
  const onChainIssue = describeOnChainViolation(value);
  if (onChainIssue) {
    errors.push({ code: "rejected-on-chain", message: onChainIssue });
  }

  // A document is immutable only when its URL pins a full Git commit.
  if (!COMMIT_SHA.test(parsed.gitRef)) {
    errors.push({
      code: "not-commit-sha",
      message: `"${parsed.gitRef}" is not a full 40-character commit SHA.`,
    });
  }

  // Descriptions are an on-chain trust boundary, so creation is limited to the
  // repository the program accepts. Compare the raw case-sensitive components to mirror the
  // program rather than treating GitHub's case-insensitive names as interchangeable.
  if (`${parsed.repo.owner}/${parsed.repo.repo}` !== SGP_REPO) {
    errors.push({
      code: "not-allowed-repo",
      message: `The link must point to https://github.com/${SGP_REPO}.`,
    });
  }

  if (!parsed.ref) {
    warnings.push({
      code: "unrecognized-filename",
      message: `"${parsed.fileName}" does not look like a proposal filename (expected sgp-0001-title.md or 0001-title.md), so no proposal number will be shown.`,
    });
  }

  return { ok: errors.length === 0, errors, warnings, parsed, normalized: value };
}

/**
 * Enforcement backstop for the SDK path; throws with the first error's user-facing message.
 *
 * Returns the exact URL that was checked so callers submit the same string to the program.
 */
export function assertValidProposalUrl(url: string): string {
  const { ok, errors, normalized } = validateProposalUrl(url);
  if (!ok) throw new Error(errors[0].message);
  return normalized;
}

function byteLength(value: string): number {
  return new TextEncoder().encode(value).length;
}

/** Mirrors `svmgov_program::utils::is_valid_github_link`. */
function describeOnChainViolation(url: string): string | undefined {
  const path = url.slice(ON_CHAIN_PREFIX.length);
  const segments = path.split("/");

  if (segments.some((segment) => segment === "")) {
    return "The link contains an empty path segment, which the on-chain program rejects.";
  }

  if (segments[2] !== "blob") {
    return "The link must use the canonical /blob/<commit-sha>/ path; /raw/ is only used internally to fetch document content.";
  }

  if (segments.some((segment) => segment === "." || segment === "..")) {
    return 'The link contains a "." or ".." path segment, which the on-chain program rejects.';
  }

  if (
    segments.length < ON_CHAIN_MIN_SEGMENTS ||
    segments.length > ON_CHAIN_MAX_SEGMENTS
  ) {
    return `The link has ${segments.length} path segments; the on-chain program accepts ${ON_CHAIN_MIN_SEGMENTS}-${ON_CHAIN_MAX_SEGMENTS}.`;
  }

  const bad = path.match(ON_CHAIN_DISALLOWED_CHAR);
  if (bad) {
    return `The link contains "${bad[0]}", which the on-chain program rejects; only ASCII letters, digits, "-", "_" and "." are allowed in the path.`;
  }

  return undefined;
}
