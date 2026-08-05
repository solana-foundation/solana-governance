import type { ProposalRef } from "./proposalUrl";

/**
 * On-chain proposal titles are free-form, and proposers are inconsistent about whether they
 * include the proposal number. Both of these are live on mainnet:
 *
 *   SGP-0002  title: "Double Disinflation"
 *   SGP-0003  title: "SGP-0003: Resource and Inclusion Fee"
 *
 * Since the title cannot be edited once on chain, the UI has to cope with both rather than
 * expect one convention.
 */
const TITLE_PREFIX = /^#?\s*(simd|sgp)[\s-]*0*(\d{1,5})\b/i;

/** True when `title` already opens by naming this proposal, in any plausible spelling. */
export function titleNamesProposal(title: string, ref: ProposalRef): boolean {
  const match = title.trimStart().match(TITLE_PREFIX);
  if (!match) return false;
  // Compare numerically so "SGP-3" matches a ref of "0003".
  return (
    match[1].toLowerCase() === ref.kind &&
    Number(match[2]) === Number(ref.number)
  );
}

/**
 * A proposal heading with its number shown exactly once.
 *
 * The proposer's own wording is left untouched when they already included the number — this
 * only decides whether to add the prefix, never rewrites what they chose to write.
 */
export function formatProposalHeading(
  ref: ProposalRef | undefined,
  title: string,
): string {
  if (!ref) return title;
  return titleNamesProposal(title, ref) ? title : `${ref.label}: ${title}`;
}
