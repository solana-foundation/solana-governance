import { makeProposalRef, type ProposalRef } from "./proposalUrl";

/**
 * Frontmatter must open on the very first line. Tolerates CRLF, since GitHub serves whatever
 * line endings the author committed.
 */
const FRONTMATTER = /^---\r?\n([\s\S]*?)\r?\n---/;

/**
 * One rule covers both repos: SIMDs write `simd: '0022'` and SGPs write `sgp: 0001`.
 * Full-line anchors keep it from matching a lookalike key (`simd_status:`) or a digit run
 * inside some other field's value.
 */
const NUMBER_LINE = /^[ \t]*(simd|sgp)[ \t]*:[ \t]*['"]?(\d+)['"]?[ \t]*$/im;

const SUMMARY_HEADING = /^#{1,6}[ \t]*summary[ \t]*:?[ \t]*$/i;
const ANY_HEADING = /^#{1,6}[ \t]/;

const FALLBACK_SUMMARY_MAX_LENGTH = 600;

export interface ParsedProposalMarkdown {
  ref: ProposalRef | undefined;
  summary: string;
}

/**
 * Reads the proposal number and summary out of a proposal markdown document.
 *
 * The frontmatter key is authoritative for which flavour a document is — that is what makes
 * an unrecognized repo resolve correctly without needing an entry in the repo config table.
 * `fallbackRef` (derived from the filename) is used only when there is no frontmatter key.
 */
export function parseProposalMarkdown(
  text: string,
  fallbackRef?: ProposalRef,
): ParsedProposalMarkdown {
  const { frontmatter, body } = splitFrontmatter(text);

  let ref = fallbackRef;
  if (frontmatter) {
    const match = frontmatter.match(NUMBER_LINE);
    if (match) {
      // Never coerce to a number: `sgp: 0001` must stay "0001", not become 1.
      ref = makeProposalRef(match[2], match[1].toLowerCase() as "simd" | "sgp");
    }
  }

  return { ref, summary: extractSummary(body) };
}

export function splitFrontmatter(text: string): {
  frontmatter: string | undefined;
  body: string;
} {
  const match = text.match(FRONTMATTER);
  if (!match) return { frontmatter: undefined, body: text };
  return {
    frontmatter: match[1],
    body: text.slice(match[0].length).replace(/^(?:\r?\n)+/, ""),
  };
}

/**
 * Prefers the document's `## Summary` section. Falls back to the first prose paragraph, since
 * the SGP template is new and later proposals may not follow it exactly.
 */
function extractSummary(body: string): string {
  const lines = body.split(/\r?\n/);
  const headingIndex = lines.findIndex((line) => SUMMARY_HEADING.test(line));

  if (headingIndex !== -1) {
    const rest = lines.slice(headingIndex + 1);
    const endIndex = rest.findIndex((line) => ANY_HEADING.test(line));
    const section = (endIndex === -1 ? rest : rest.slice(0, endIndex))
      .join("\n")
      .trim();
    if (section) return section;
  }

  return firstParagraph(lines);
}

function firstParagraph(lines: string[]): string {
  const paragraph: string[] = [];
  for (const line of lines) {
    if (ANY_HEADING.test(line)) {
      if (paragraph.length > 0) break;
      continue;
    }
    if (line.trim() === "") {
      if (paragraph.length > 0) break;
      continue;
    }
    paragraph.push(line);
  }

  const text = paragraph.join("\n").trim();
  return text.length > FALLBACK_SUMMARY_MAX_LENGTH
    ? `${text.slice(0, FALLBACK_SUMMARY_MAX_LENGTH).trimEnd()}…`
    : text;
}
