import type { ReactNode } from "react";
import { useProposalRef } from "@/hooks";
import type { ProposalRef } from "@/lib/github";

interface Props {
  /** The proposal's on-chain `description` URL. */
  url: string;
  /** Number parsed synchronously from the URL; shown until (and unless) the document loads. */
  fallback?: ProposalRef;
  className?: string;
  /** Rendered when no number resolves. Pass `null` to render nothing. */
  placeholder?: ReactNode;
}

/**
 * Renders a proposal's number on its own (`SIMD-0022`, `SGP-0001`). To show it alongside the
 * title, use `ProposalHeading`, which avoids repeating a number the title already contains.
 *
 * This is a component rather than a bare hook call so it can be used inside table cell
 * renderers and `.map()` bodies, where calling a hook directly would be invalid. Every
 * instance for the same URL shares one react-query entry, so repeats cost no extra requests.
 */
export const ProposalRefLabel = ({
  url,
  fallback,
  className,
  placeholder = "-",
}: Props) => {
  const proposalRef = useProposalRef(url, fallback);

  if (!proposalRef) {
    return placeholder === null ? null : (
      <span className={className}>{placeholder}</span>
    );
  }

  return <span className={className}>{proposalRef.label}</span>;
};
