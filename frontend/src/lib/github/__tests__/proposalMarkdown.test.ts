import { parseProposalMarkdown, splitFrontmatter } from "../proposalMarkdown";
import { makeProposalRef } from "../proposalUrl";

const SIMD_DOC = `---
simd: '0022'
title: Multi Delegation Stake Account
---

## Summary

This is the summary text.

## Motivation

Not the summary.
`;

// Verbatim shape of proposals/sgp-0001-solana-constitution.md.
const SGP_DOC = `---
sgp: 0001
title: The Solana Constitution
authors: Nick Almond <nick@jito.network> (Jito)
status: Draft
created: 2026-06-29
---

## Summary

A "yes" vote on this SGP ratifies **The Solana Constitution**.

## Technical Sponsor

No future technical work is required.
`;

describe("parseProposalMarkdown - proposal number", () => {
  it("reads a quoted simd value", () => {
    expect(parseProposalMarkdown(SIMD_DOC).ref).toEqual({
      number: "0022",
      kind: "simd",
      label: "SIMD-0022",
    });
  });

  it("reads an unquoted sgp value and preserves leading zeros", () => {
    const { ref } = parseProposalMarkdown(SGP_DOC);
    expect(ref).toEqual({ number: "0001", kind: "sgp", label: "SGP-0001" });
    // Guards against a `Number()` creeping in and turning "0001" into 1.
    expect(ref?.number).not.toBe(1);
  });

  it.each([
    [`---\nsimd: '0022'\n---\n`, "0022"],
    [`---\nsimd: "0022"\n---\n`, "0022"],
    [`---\nsimd: 0022\n---\n`, "0022"],
    [`---\nsgp: 0001\n---\n`, "0001"],
    [`---\n  simd:   0022  \n---\n`, "0022"],
  ])("parses %j", (doc, expected) => {
    expect(parseProposalMarkdown(doc).ref?.number).toBe(expected);
  });

  it("tolerates CRLF line endings", () => {
    const doc = "---\r\nsgp: 0003\r\ntitle: x\r\n---\r\n\r\n## Summary\r\n\r\nFees.\r\n";
    const parsed = parseProposalMarkdown(doc);
    expect(parsed.ref?.label).toBe("SGP-0003");
    expect(parsed.summary).toBe("Fees.");
  });

  it("ignores a lookalike key", () => {
    const doc = `---\nsimd_status: 0044\ntitle: x\n---\n\n## Summary\n\nBody.\n`;
    expect(parseProposalMarkdown(doc).ref).toBeUndefined();
  });

  it("ignores digits inside another field's value", () => {
    const doc = `---\ntitle: Something about simd: 0044 inline\n---\n\n## Summary\n\nBody.\n`;
    expect(parseProposalMarkdown(doc).ref).toBeUndefined();
  });

  it("falls back to the filename-derived ref when frontmatter has no key", () => {
    const doc = `---\ntitle: x\n---\n\n## Summary\n\nBody.\n`;
    expect(
      parseProposalMarkdown(doc, makeProposalRef("0077", "simd")).ref?.label,
    ).toBe("SIMD-0077");
  });

  it("lets frontmatter override the filename-derived ref", () => {
    expect(
      parseProposalMarkdown(SGP_DOC, makeProposalRef("0099", "simd")).ref,
    ).toEqual({ number: "0001", kind: "sgp", label: "SGP-0001" });
  });

  it("returns undefined when there is no frontmatter and no fallback", () => {
    expect(parseProposalMarkdown("# Title\n\nBody.\n").ref).toBeUndefined();
  });
});

describe("parseProposalMarkdown - summary", () => {
  it("extracts the Summary section and stops at the next heading", () => {
    expect(parseProposalMarkdown(SIMD_DOC).summary).toBe(
      "This is the summary text.",
    );
  });

  it("stops at a deeper heading too", () => {
    const doc = `## Summary\n\nTop level.\n\n### Detail\n\nNested.\n`;
    expect(parseProposalMarkdown(doc).summary).toBe("Top level.");
  });

  it("handles a Summary section that runs to end of file", () => {
    expect(parseProposalMarkdown(`## Summary\n\nLast thing.`).summary).toBe(
      "Last thing.",
    );
  });

  it("matches a Summary heading at any depth and ignores case", () => {
    expect(parseProposalMarkdown(`# summary\n\nText.\n## Next\n`).summary).toBe(
      "Text.",
    );
  });

  it("preserves multi-paragraph summaries", () => {
    const doc = `## Summary\n\nOne.\n\nTwo.\n\n## Motivation\n\nNope.\n`;
    expect(parseProposalMarkdown(doc).summary).toBe("One.\n\nTwo.");
  });

  it("falls back to the first paragraph when there is no Summary heading", () => {
    const doc = `---\nsgp: 0002\n---\n\n# Double Disinflation\n\nThis proposal changes the schedule.\n\nMore detail here.\n`;
    expect(parseProposalMarkdown(doc).summary).toBe(
      "This proposal changes the schedule.",
    );
  });

  it("truncates a very long fallback paragraph", () => {
    const long = "x".repeat(900);
    const summary = parseProposalMarkdown(`# Title\n\n${long}\n`).summary;
    expect(summary).toHaveLength(601);
    expect(summary.endsWith("…")).toBe(true);
  });

  it("returns an empty string for an empty document", () => {
    expect(parseProposalMarkdown("").summary).toBe("");
  });
});

describe("splitFrontmatter", () => {
  it("separates frontmatter from the body", () => {
    const { frontmatter, body } = splitFrontmatter(SIMD_DOC);
    expect(frontmatter).toContain("simd: '0022'");
    expect(body.startsWith("## Summary")).toBe(true);
  });

  it("returns the whole text as body when there is no frontmatter", () => {
    const { frontmatter, body } = splitFrontmatter("## Summary\n\nHi.\n");
    expect(frontmatter).toBeUndefined();
    expect(body).toBe("## Summary\n\nHi.\n");
  });

  it("does not treat a mid-document --- as frontmatter", () => {
    const { frontmatter } = splitFrontmatter("# Title\n\n---\n\nfoo: bar\n---\n");
    expect(frontmatter).toBeUndefined();
  });
});
