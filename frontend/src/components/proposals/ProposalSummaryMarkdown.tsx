import type { MouseEvent } from "react";
import type { Components } from "react-markdown";
import ReactMarkdown from "react-markdown";

const ALLOWED_ELEMENTS = [
  "p",
  "strong",
  "em",
  "a",
  "code",
  "ul",
  "ol",
  "li",
  "br",
];

function stopLinkNavigationBubble(event: MouseEvent<HTMLAnchorElement>) {
  event.stopPropagation();
}

const components: Components = {
  p: ({ children }) => <p className="mb-2 last:mb-0">{children}</p>,
  strong: ({ children }) => (
    <strong className="font-semibold text-white/80">{children}</strong>
  ),
  em: ({ children }) => <em>{children}</em>,
  code: ({ children }) => (
    <code className="rounded bg-white/10 px-1 py-0.5 font-mono text-[0.85em] text-white/80">
      {children}
    </code>
  ),
  a: ({ href, children }) => (
    <a
      href={href}
      target="_blank"
      rel="noreferrer noopener"
      onClick={stopLinkNavigationBubble}
      className="text-primary underline-offset-2 hover:underline"
    >
      {children}
    </a>
  ),
  ul: ({ children }) => (
    <ul className="mb-2 list-disc space-y-1 pl-4 last:mb-0">{children}</ul>
  ),
  ol: ({ children }) => (
    <ol className="mb-2 list-decimal space-y-1 pl-4 last:mb-0">{children}</ol>
  ),
};

interface Props {
  summary: string;
}

export function ProposalSummaryMarkdown({ summary }: Props) {
  if (!summary) return null;

  return (
    <ReactMarkdown
      allowedElements={ALLOWED_ELEMENTS}
      unwrapDisallowed
      components={components}
    >
      {summary}
    </ReactMarkdown>
  );
}
