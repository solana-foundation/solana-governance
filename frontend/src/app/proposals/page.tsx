"use client";
import { Suspense } from "react";
import ProposalsHeader from "@/components/proposals/ProposalsHeader";
import ProposalsView from "@/components/proposals/ProposalsView";

export default function ProposalsPage() {
  return (
    <main className="py-8 space-y-10">
      <ProposalsHeader title="Proposal Overview" />
      <Suspense fallback={null}>
        <ProposalsView title="Recent Proposals" />
      </Suspense>
    </main>
  );
}
