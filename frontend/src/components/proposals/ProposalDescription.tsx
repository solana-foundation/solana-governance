import { useEffect, useRef, useState } from "react";
import { useProposalDocument } from "@/hooks";

interface Props {
  githubUrl: string;
}

export const ProposalDescription = ({ githubUrl }: Props) => {
  const { data, isLoading } = useProposalDocument(githubUrl);
  const scrollRef = useRef<HTMLParagraphElement>(null);
  const [showTopShadow, setShowTopShadow] = useState(false);
  const [showBottomShadow, setShowBottomShadow] = useState(false);

  useEffect(() => {
    const element = scrollRef.current;
    if (!element) return;

    const checkScroll = () => {
      const { scrollTop, scrollHeight, clientHeight } = element;
      setShowTopShadow(scrollTop > 0);
      setShowBottomShadow(scrollTop < scrollHeight - clientHeight - 1);
    };

    // Check initially
    checkScroll();

    // Check on scroll
    element.addEventListener("scroll", checkScroll);
    // Check on resize (in case content changes)
    window.addEventListener("resize", checkScroll);

    return () => {
      element.removeEventListener("scroll", checkScroll);
      window.removeEventListener("resize", checkScroll);
    };
  }, [data?.summary]);

  if (isLoading) return <p>Loading summary...</p>;

  // if link is invalid or some other error, show nothing. user will just see link to github
  // if (isError) return <p>Error: {(error as Error).message}</p>;

  return (
    <div className="relative">
      {/* Top shadow gradient */}
      {showTopShadow && (
        <div
          className="pointer-events-none absolute top-0 left-0 right-0 h-10 z-10"
          style={{
            background:
              "radial-gradient(ellipse 60% 100% at center top, rgba(13, 15, 23, 1) 0%, rgba(13, 15, 23, 0.6) 40%, rgba(13, 15, 23, 0) 100%)",
          }}
        />
      )}
      {/* Bottom shadow gradient */}
      {showBottomShadow && (
        <div
          className="pointer-events-none absolute bottom-0 left-0 right-0 h-10 z-10"
          style={{
            background:
              "radial-gradient(ellipse 60% 100% at center bottom, rgba(13, 15, 23, 1) 0%, rgba(13, 15, 23, 0.6) 40%, rgba(13, 15, 23, 0) 100%)",
          }}
        />
      )}
      <p
        ref={scrollRef}
        className="break-all whitespace-pre-wrap text-sm leading-relaxed text-(--basic-color-gray) [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none] text-wrap"
        style={{
          maxHeight: "300px",
          overflowY: "auto",
        }}
      >
        {data?.summary}
      </p>
    </div>
  );
};
