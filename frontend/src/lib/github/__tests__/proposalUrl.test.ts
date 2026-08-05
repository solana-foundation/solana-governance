import {
  getProposalRefFromUrl,
  parseProposalUrl,
  proposalRefFromFileName,
  rawContentUrl,
  resolveRepoConfig,
  SGP_REPO,
  SIMD_REPO,
} from "../proposalUrl";

const SIMD_BLOB = `https://github.com/${SIMD_REPO}/blob/main/proposals/0022-multi-stake.md`;
const SGP_BLOB = `https://github.com/${SGP_REPO}/blob/main/proposals/sgp-0001-solana-constitution.md`;
const SGP_SHA = "27bca51e5c0fc34ddbea6904faf86f5098225316";

describe("parseProposalUrl - blob URLs", () => {
  it("parses a SIMD proposal URL", () => {
    expect(parseProposalUrl(SIMD_BLOB)).toEqual({
      kind: "blob",
      repo: {
        owner: "solana-foundation",
        repo: "solana-improvement-documents",
      },
      gitRef: "main",
      path: "proposals/0022-multi-stake.md",
      fileName: "0022-multi-stake.md",
      ref: { number: "0022", kind: "simd", label: "SIMD-0022" },
    });
  });

  it("parses an SGP proposal URL on main", () => {
    const parsed = parseProposalUrl(SGP_BLOB);
    expect(parsed).toMatchObject({
      kind: "blob",
      repo: {
        owner: "solana-foundation",
        repo: "solana-governance-proposals",
      },
      gitRef: "main",
      path: "proposals/sgp-0001-solana-constitution.md",
      ref: { number: "0001", kind: "sgp", label: "SGP-0001" },
    });
  });

  it("parses an SGP proposal URL pinned to a commit SHA", () => {
    const parsed = parseProposalUrl(
      `https://github.com/${SGP_REPO}/blob/${SGP_SHA}/proposals/sgp-0001-solana-constitution.md`,
    );
    expect(parsed).toMatchObject({ kind: "blob", gitRef: SGP_SHA });
  });

  it("parses a raw.githubusercontent.com URL as a blob", () => {
    const parsed = parseProposalUrl(
      `https://raw.githubusercontent.com/${SIMD_REPO}/main/proposals/0022-multi-stake.md`,
    );
    expect(parsed).toMatchObject({
      kind: "blob",
      gitRef: "main",
      path: "proposals/0022-multi-stake.md",
      ref: { label: "SIMD-0022" },
    });
  });

  it("accepts the /raw/ path form on github.com", () => {
    const parsed = parseProposalUrl(
      `https://github.com/${SIMD_REPO}/raw/main/proposals/0022-multi-stake.md`,
    );
    expect(parsed).toMatchObject({ kind: "blob", ref: { label: "SIMD-0022" } });
  });

  it("ignores a ?plain=1 query and a line-range fragment", () => {
    const parsed = parseProposalUrl(`${SIMD_BLOB}?plain=1#L10-L20`);
    expect(parsed).toMatchObject({
      kind: "blob",
      path: "proposals/0022-multi-stake.md",
      ref: { label: "SIMD-0022" },
    });
  });

  it("decodes percent-encoded path segments", () => {
    const parsed = parseProposalUrl(
      `https://github.com/${SGP_REPO}/blob/${SGP_SHA}/proposals%2Fsgp-0001-solana-constitution.md`,
    );
    expect(parsed).toMatchObject({
      kind: "blob",
      // A single encoded segment decodes to a path containing a slash.
      path: "proposals/sgp-0001-solana-constitution.md",
    });
  });

  it("normalizes a www. prefix and uppercase host", () => {
    const parsed = parseProposalUrl(
      `https://WWW.GitHub.com/${SIMD_REPO}/blob/main/proposals/0022-multi-stake.md`,
    );
    expect(parsed).toMatchObject({ kind: "blob", ref: { label: "SIMD-0022" } });
  });
});

describe("parseProposalUrl - pull request URLs", () => {
  it.each([
    [`https://github.com/${SGP_REPO}/pull/3`],
    [`https://github.com/${SGP_REPO}/pull/3/files`],
    [`https://github.com/${SGP_REPO}/pull/3/files#diff-abc123`],
    [`https://github.com/${SGP_REPO}/pull/3/commits/${SGP_SHA}`],
    [`https://github.com/${SGP_REPO}/pull/3/`],
  ])("classifies %s as a pull request", (url) => {
    expect(parseProposalUrl(url)).toEqual({
      kind: "pull",
      repo: {
        owner: "solana-foundation",
        repo: "solana-governance-proposals",
      },
      pullNumber: 3,
    });
  });

  it("rejects a pull URL with no number", () => {
    expect(parseProposalUrl(`https://github.com/${SGP_REPO}/pull`)).toMatchObject(
      { kind: "unsupported", code: "unrecognized" },
    );
  });
});

describe("parseProposalUrl - unsupported URLs", () => {
  it.each([
    ["", "empty"],
    ["   ", "empty"],
    ["not a url", "not-a-url"],
    [`http://github.com/${SIMD_REPO}/blob/main/proposals/0022-x.md`, "not-https"],
    ["https://gitlab.com/org/repo/blob/main/proposals/0022-x.md", "not-github"],
    ["https://github.com/org/repo/tree/main/proposals", "tree-or-directory"],
    ["https://github.com/org/repo/issues/12", "unrecognized"],
    ["https://github.com/org/repo", "unrecognized"],
    ["https://github.com/org", "unrecognized"],
  ])("classifies %s as unsupported (%s)", (url, code) => {
    expect(parseProposalUrl(url)).toMatchObject({
      kind: "unsupported",
      code,
    });
  });
});

// Ported verbatim from the deleted hooks/__tests__/useProposalSimd.test.ts — these cases are
// the only written record of the pre-existing sync-parsing contract.
describe("getProposalRefFromUrl (ported useProposalSimd contract)", () => {
  it("extracts the SIMD code from a standard proposal URL", () => {
    expect(
      getProposalRefFromUrl(
        `https://github.com/${SIMD_REPO}/blob/main/proposals/0075-token22.md`,
      )?.number,
    ).toBe("0075");
  });

  it("extracts a 5 digit SIMD code", () => {
    expect(
      getProposalRefFromUrl(
        `https://github.com/${SIMD_REPO}/blob/main/proposals/00754-token22.md`,
      )?.number,
    ).toBe("00754");
  });

  it("works regardless of directory structure", () => {
    expect(
      getProposalRefFromUrl(
        "https://github.com/org/repo/blob/dev/specs/subdir/0123-some-feature.md",
      )?.number,
    ).toBe("0123");
  });

  it("returns undefined for a tree URL", () => {
    expect(
      getProposalRefFromUrl(
        "https://github.com/org/repo/tree/main/0123-something.md",
      ),
    ).toBeUndefined();
  });

  it("returns undefined if the filename does not start with a number", () => {
    expect(
      getProposalRefFromUrl(
        "https://github.com/org/repo/blob/main/specs/something.md",
      ),
    ).toBeUndefined();
  });

  it("returns undefined for a pull request URL", () => {
    expect(
      getProposalRefFromUrl(`https://github.com/${SGP_REPO}/pull/3`),
    ).toBeUndefined();
  });

  it("labels SGP files from an unknown repo using the sgp- prefix", () => {
    expect(
      getProposalRefFromUrl(
        "https://github.com/someone/fork/blob/main/proposals/sgp-0002-double-disinflation.md",
      ),
    ).toEqual({ number: "0002", kind: "sgp", label: "SGP-0002" });
  });
});

describe("proposalRefFromFileName", () => {
  const simdConfig = resolveRepoConfig({
    owner: "solana-foundation",
    repo: "solana-improvement-documents",
  });
  const sgpConfig = resolveRepoConfig({
    owner: "solana-foundation",
    repo: "solana-governance-proposals",
  });

  it("rejects the SGP template file", () => {
    expect(
      proposalRefFromFileName("XXXX-sgp-template.md", sgpConfig),
    ).toBeUndefined();
  });

  it("rejects README.md", () => {
    expect(proposalRefFromFileName("README.md", sgpConfig)).toBeUndefined();
  });

  it("does not accept SIMD-style names in the SGP repo", () => {
    expect(proposalRefFromFileName("0001-thing.md", sgpConfig)).toBeUndefined();
  });

  it("does not accept SGP-style names in the SIMD repo", () => {
    expect(
      proposalRefFromFileName("sgp-0001-thing.md", simdConfig),
    ).toBeUndefined();
  });

  it("accepts a bare numbered filename", () => {
    expect(proposalRefFromFileName("0022.md", simdConfig)).toEqual({
      number: "0022",
      kind: "simd",
      label: "SIMD-0022",
    });
  });

  it("rejects a non-markdown file", () => {
    expect(proposalRefFromFileName("0022-x.txt", simdConfig)).toBeUndefined();
  });
});

describe("rawContentUrl", () => {
  it("builds a raw.githubusercontent.com URL", () => {
    expect(
      rawContentUrl(
        { owner: "solana-foundation", repo: "solana-governance-proposals" },
        SGP_SHA,
        "proposals/sgp-0001-solana-constitution.md",
      ),
    ).toBe(
      `https://raw.githubusercontent.com/solana-foundation/solana-governance-proposals/${SGP_SHA}/proposals/sgp-0001-solana-constitution.md`,
    );
  });

  it("encodes path segments without encoding the separators", () => {
    expect(
      rawContentUrl({ owner: "o", repo: "r" }, "main", "a dir/b file.md"),
    ).toBe("https://raw.githubusercontent.com/o/r/main/a%20dir/b%20file.md");
  });
});
