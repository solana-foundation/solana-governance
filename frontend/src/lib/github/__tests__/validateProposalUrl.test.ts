import { SGP_REPO, SIMD_REPO } from "../proposalUrl";
import {
  assertValidProposalUrl,
  validateProposalUrl,
} from "../validateProposalUrl";

const SHA = "27bca51e5c0fc34ddbea6904faf86f5098225316";
const VALID_SIMD = `https://github.com/${SIMD_REPO}/blob/main/proposals/0022-multi-stake.md`;
const VALID_SGP = `https://github.com/${SGP_REPO}/blob/${SHA}/proposals/sgp-0001-solana-constitution.md`;

const codes = (issues: { code: string }[]) => issues.map((i) => i.code);

describe("validateProposalUrl - accepted", () => {
  it("accepts a SIMD proposal link", () => {
    const result = validateProposalUrl(VALID_SIMD);
    expect(result.ok).toBe(true);
    expect(result.errors).toEqual([]);
  });

  it("accepts an SGP proposal link pinned to a commit with no warnings", () => {
    const result = validateProposalUrl(VALID_SGP);
    expect(result.ok).toBe(true);
    expect(result.warnings).toEqual([]);
  });

  it("trims surrounding whitespace", () => {
    expect(validateProposalUrl(`  ${VALID_SGP}  `).ok).toBe(true);
  });
});

// The on-chain check requires a literal https://github.com/ prefix, so submitting the raw
// input after validating its trimmed form would be rejected by the program despite the
// frontend having accepted it. Callers must send `normalized`.
describe("validateProposalUrl - normalization", () => {
  it("returns the trimmed URL as the value to submit", () => {
    expect(validateProposalUrl(`\n  ${VALID_SGP}\t `).normalized).toBe(
      VALID_SGP,
    );
  });

  it("returns the normalized URL from the assert helper", () => {
    expect(assertValidProposalUrl(`  ${VALID_SGP}  `)).toBe(VALID_SGP);
  });

  it("normalizes even when validation fails, so errors can quote the real value", () => {
    expect(validateProposalUrl("  not a url  ").normalized).toBe("not a url");
  });
});

describe("validateProposalUrl - rejected", () => {
  it.each([
    ["", "empty"],
    ["   ", "empty"],
    ["github.com/org/repo", "not-a-url"],
    [`http://github.com/${SIMD_REPO}/blob/main/proposals/0022-x.md`, "not-https"],
    ["https://gitlab.com/org/repo/blob/main/proposals/0022-x.md", "not-github"],
    [`https://github.com/${SGP_REPO}/pull/3`, "pull-request"],
    [`https://github.com/${SGP_REPO}/pull/3/files`, "pull-request"],
    ["https://github.com/org/repo/tree/main/proposals", "tree-or-directory"],
    ["https://github.com/org/repo", "unsupported"],
    ["https://github.com/org/repo/issues/12", "unsupported"],
    [
      `https://github.com/${SIMD_REPO}/blob/main/proposals/0022-multi-stake.txt`,
      "not-markdown",
    ],
    [`${VALID_SIMD}?plain=1`, "query-or-fragment"],
    [`${VALID_SIMD}#L10`, "query-or-fragment"],
  ])("rejects %j with code %s", (url, code) => {
    const result = validateProposalUrl(url);
    expect(result.ok).toBe(false);
    expect(codes(result.errors)).toContain(code);
  });

  // These resolve fine in a browser but are rejected by the on-chain validator, so accepting
  // them would only surface as an opaque custom program error at transaction time.
  it.each([
    [`https://www.github.com/${SIMD_REPO}/blob/main/proposals/0022-x.md`],
    [`https://raw.githubusercontent.com/${SIMD_REPO}/main/proposals/0022-x.md`],
  ])("rejects %j because the program requires an exact github.com prefix", (url) => {
    const result = validateProposalUrl(url);
    expect(result.ok).toBe(false);
    expect(codes(result.errors)).toContain("not-github");
  });

  it.each([
    [
      `https://github.com/${SIMD_REPO}/blob/main/proposals/0022%20multi.md`,
      "character",
    ],
    [
      `https://github.com/${SIMD_REPO}/blob/main/a/b/c/d/e/f/g/h/0022-x.md`,
      "segments",
    ],
  ])("rejects %j as incompatible with the on-chain grammar", (url) => {
    const result = validateProposalUrl(url);
    expect(result.ok).toBe(false);
    expect(codes(result.errors)).toContain("rejected-on-chain");
  });

  it("rejects a link longer than the on-chain description limit", () => {
    const url = `https://github.com/${SIMD_REPO}/blob/main/proposals/${"a".repeat(500)}/0022-x.md`;
    expect(codes(validateProposalUrl(url).errors)).toContain("too-long");
  });

  it("gives pull request links actionable guidance", () => {
    const { errors } = validateProposalUrl(`https://github.com/${SGP_REPO}/pull/3`);
    expect(errors[0].message).toContain("pull request");
    expect(errors[0].message).toContain("blob/<commit-sha>");
  });
});

describe("validateProposalUrl - warnings", () => {
  it("warns about a branch ref but still passes", () => {
    const result = validateProposalUrl(VALID_SIMD);
    expect(result.ok).toBe(true);
    expect(codes(result.warnings)).toContain("mutable-ref");
  });

  it("warns about an unknown repo but still passes", () => {
    const result = validateProposalUrl(
      `https://github.com/someone/fork/blob/${SHA}/proposals/sgp-0002-x.md`,
    );
    expect(result.ok).toBe(true);
    expect(codes(result.warnings)).toContain("unknown-repo");
  });

  it("warns when the filename carries no proposal number", () => {
    const result = validateProposalUrl(
      `https://github.com/${SGP_REPO}/blob/${SHA}/proposals/notes.md`,
    );
    expect(result.ok).toBe(true);
    expect(codes(result.warnings)).toContain("unrecognized-filename");
  });
});

describe("assertValidProposalUrl", () => {
  it("does not throw for a valid link", () => {
    expect(() => assertValidProposalUrl(VALID_SGP)).not.toThrow();
  });

  it("throws the first error message for a pull request link", () => {
    expect(() =>
      assertValidProposalUrl(`https://github.com/${SGP_REPO}/pull/3`),
    ).toThrow(/pull request/);
  });
});
