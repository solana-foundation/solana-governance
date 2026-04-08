"use client";

import { notFound, useParams } from "next/navigation";
import ProposalDetailView from "@/components/proposals/detail/ProposalDetailView";
import { useProposalDetails } from "@/hooks";

export const ProposalDetailClientPage = () => {
  const params = useParams();
  const proposalPublicKey = params.proposalPk;

  if (!proposalPublicKey || typeof proposalPublicKey !== "string") notFound();

  const {
    data: proposalData,
    isFetched,
    isLoading,
  } = useProposalDetails(proposalPublicKey);

  if (!proposalData && isFetched) {
    notFound();
  }

  return (
    <main className="space-y-8 py-8">
      <ProposalDetailView proposal={proposalData} isLoading={isLoading} />
    </main>
  );
};
