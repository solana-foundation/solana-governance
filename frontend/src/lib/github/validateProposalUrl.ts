import {
  isKnownProposalRepo,
  parseProposalUrl,
  type ParsedProposalUrl,
} from "./proposalUrl";

export type ProposalUrlErrorCode =
  | "empty"
  | "not-a-url"
  | "not-https"
  | "not-github"
  | "pull-request"
  | "tree-or-directory"
  | "not-markdown"
  | "query-or-fragment"
  | "too-long"
  | "rejected-on-chain"
  | "unsupported";

export type ProposalUrlWarningCode =
  | "mutable-ref"
  | "unknown-repo"
  | "unrecognized-filename";

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
   *
   * Validation trims, but the on-chain check requires a literal `https://github.com/` prefix
   * with no leading whitespace — so submitting the raw input instead would be rejected by the
   * program after the frontend had already accepted it.
   */
  normalized: string;
}

/**
 * Soft mirror of the program's `global_config.max_description_length`. That value is
 * configurable on chain, so this is a client-side courtesy check, not the authority.
 */
const MAX_DESCRIPTION_BYTES = 500;

const COMMIT_SHA = /^[0-9a-f]{40}$/i;

/**
 * The exact prefix the on-chain validator requires. A `www.` host or a
 * raw.githubusercontent.com link resolves fine in a browser but is rejected on chain, so it
 * has to be caught here rather than at transaction time.
 */
const ON_CHAIN_PREFIX = "https://github.com/";

/** Character class the on-chain validator allows in the path, plus `/` as the separator. */
const ON_CHAIN_DISALLOWED_CHAR = /[^\p{Alphabetic}\p{N}\-_./]/u;

const ON_CHAIN_MIN_SEGMENTS = 2;
const ON_CHAIN_MAX_SEGMENTS = 10;

const PULL_REQUEST_MESSAGE = [
  "Link to the proposal markdown file, not to a pull request.",
  "",
  'Open the PR\'s "Files changed" tab, click the proposal .md file, and copy its URL — it looks like',
  "https://github.com/<owner>/<repo>/blob/<commit-sha>/proposals/sgp-0001-....md",
  "",
  "Prefer the commit SHA over a branch name: the description is stored on chain and cannot be",
  "edited, so a branch link breaks once the branch moves or is deleted.",
].join("\n");

/**
 * Validates a proposal description URL before it is written on chain.
 *
 * The on-chain program only checks that the string looks broadly like a GitHub link — a pull
 * request URL is four clean path segments, so it passes there. This is where that is caught.
 *
 * Rule numbering is mirrored in svmgov/cli/src/utils/proposal_link.rs; keep the two in step.
 */
export function validateProposalUrl(url: string): ProposalUrlValidation {
  const errors: ProposalUrlIssue<ProposalUrlErrorCode>[] = [];
  const warnings: ProposalUrlIssue<ProposalUrlWarningCode>[] = [];
  const trimmed = url?.trim() ?? "";
  const parsed = parseProposalUrl(trimmed);

  const fail = (code: ProposalUrlErrorCode, message: string) => {
    errors.push({ code, message });
    return { ok: false, errors, warnings, parsed, normalized: trimmed };
  };

  // 1-5: shape. `parseProposalUrl` already distinguishes these cases.
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

  // 3 (continued): `parseProposalUrl` is deliberately lenient about the host so that existing
  // on-chain descriptions still render. Creation has to be stricter than that.
  if (!trimmed.startsWith(ON_CHAIN_PREFIX)) {
    return fail(
      "not-github",
      `The link must start with ${ON_CHAIN_PREFIX} — no "www.", and not raw.githubusercontent.com.`,
    );
  }

  // 6: the document has to be markdown.
  if (!/\.md$/i.test(parsed.fileName)) {
    errors.push({
      code: "not-markdown",
      message: "The link must point at a .md file.",
    });
  }

  // 7: the on-chain validator rejects `?` and `#` outright, so these fail at the program.
  if (/[?#]/.test(trimmed)) {
    errors.push({
      code: "query-or-fragment",
      message:
        "Remove the query string or #fragment — the on-chain program rejects them.",
    });
  }

  // 8
  if (byteLength(trimmed) > MAX_DESCRIPTION_BYTES) {
    errors.push({
      code: "too-long",
      message: `The link must be at most ${MAX_DESCRIPTION_BYTES} bytes.`,
    });
  }

  // Re-check the on-chain grammar directly rather than assuming the shape above implies it,
  // so anything accepted here is guaranteed to be accepted by the program.
  const onChainIssue = describeOnChainViolation(trimmed);
  if (onChainIssue) {
    errors.push({ code: "rejected-on-chain", message: onChainIssue });
  }

  // 9
  if (!COMMIT_SHA.test(parsed.gitRef)) {
    warnings.push({
      code: "mutable-ref",
      message: `"${parsed.gitRef}" is a branch or tag. The description cannot be changed once on chain, so a full commit SHA is safer.`,
    });
  }

  // 10: unknown repos are allowed — only their shape is checked.
  if (!isKnownProposalRepo(parsed.repo)) {
    warnings.push({
      code: "unknown-repo",
      message: `${parsed.repo.owner}/${parsed.repo.repo} is not a recognized proposal repository.`,
    });
  }

  // 11
  if (!parsed.ref) {
    warnings.push({
      code: "unrecognized-filename",
      message: `"${parsed.fileName}" does not look like a proposal filename (expected sgp-0001-title.md or 0001-title.md), so no proposal number will be shown.`,
    });
  }

  return { ok: errors.length === 0, errors, warnings, parsed, normalized: trimmed };
}

/**
 * Enforcement backstop for the SDK path; throws with the first error's user-facing message.
 *
 * Returns the normalized URL, which callers must use in place of their raw input so the string
 * that was checked is the string that reaches the program.
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
  const path = url.slice(ON_CHAIN_PREFIX.length).replace(/\/$/, "");
  const segments = path.split("/");

  if (segments.some((segment) => segment === "")) {
    return "The link contains an empty path segment, which the on-chain program rejects.";
  }

  if (
    segments.length < ON_CHAIN_MIN_SEGMENTS ||
    segments.length > ON_CHAIN_MAX_SEGMENTS
  ) {
    return `The link has ${segments.length} path segments; the on-chain program accepts ${ON_CHAIN_MIN_SEGMENTS}-${ON_CHAIN_MAX_SEGMENTS}.`;
  }

  const bad = path.match(ON_CHAIN_DISALLOWED_CHAR);
  if (bad) {
    return `The link contains "${bad[0]}", which the on-chain program rejects; only letters, digits, "-", "_" and "." are allowed in the path.`;
  }

  return undefined;
}
