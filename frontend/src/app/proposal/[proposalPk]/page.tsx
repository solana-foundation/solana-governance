import { Suspense } from "react";
import { notFound } from "next/navigation";
import { captureException } from "@sentry/nextjs";
import ProposalDetailView from "@/components/proposals/detail/ProposalDetailView";
import { ProposalDetailClientPage } from "../ProposalDetailClientPage";

type ProposalPageProps = {
  params: Promise<{
    proposalPk?: string;
  }>;
};

async function ProposalPageContent({ params }: ProposalPageProps) {
  const resolvedParams = await params;
  const proposalPublicKey = resolvedParams.proposalPk;

  if (!proposalPublicKey || typeof proposalPublicKey !== "string") {
    captureException(new Error("Invalid proposalPublicKey route param"));
    notFound();
  }

  return <ProposalDetailClientPage proposalPublicKey={proposalPublicKey} />;
}

export default function ProposalPage({ params }: ProposalPageProps) {
  return (
    // The fallback must render DOM: Next.js scrolls a navigation to the top by
    // finding the page's first element, and skips the scroll if there is none,
    // leaving the reader at the previous page's offset.
    <Suspense
      fallback={
        <main className="space-y-8 py-8">
          <ProposalDetailView proposal={undefined} isLoading />
        </main>
      }
    >
      <ProposalPageContent params={params} />
    </Suspense>
  );
}
