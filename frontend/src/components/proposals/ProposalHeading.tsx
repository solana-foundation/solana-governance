import { useProposalRef } from "@/hooks";
import { formatProposalHeading, type ProposalRef } from "@/lib/github";

interface Props {
  /** The proposal's on-chain `description` URL. */
  url: string;
  /** Number parsed synchronously from the URL; used until (and unless) the document loads. */
  fallback?: ProposalRef;
  /** The on-chain title, verbatim. */
  title: string;
}

/**
 * A proposal's heading text: its number followed by its title, with the number shown once even
 * when the proposer already put it in the title.
 *
 * Renders bare text so the caller keeps full control of the surrounding element, and is a
 * component rather than a hook call so it can be used inside `.map()` bodies.
 */
export const ProposalHeading = ({ url, fallback, title }: Props) => {
  const proposalRef = useProposalRef(url, fallback);
  return <>{formatProposalHeading(proposalRef, title)}</>;
};
