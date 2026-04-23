"use client";

import type { ProposalRecord } from "@/types";
import ProposalBreadcrumb from "./ProposalBreadcrumb";
import ProposalDetailHeader from "./ProposalDetailHeader";
import VoteBreakdown from "./VoteBreakdown";
import PhaseTimeline from "./phase-timeline";
import TopVotersTable from "./TopVotersTable";
import CastVoteWrapper from "./CastVote";
import SupportProposalSection from "./SupportProposalSection";
import { useWallet } from "@solana/wallet-adapter-react";
import { SupportPhaseProgress } from "./support-phase-progress";

interface ProposalDetailViewProps {
  proposal: ProposalRecord | undefined;
  isLoading: boolean;
}

export default function ProposalDetailView({
  proposal,
  isLoading,
}: ProposalDetailViewProps) {
  const { connected: isConnected } = useWallet();

  const isSupporting = proposal?.status === "supporting";
  const isDiscussion = proposal?.status === "discussion";
  const isFailed = proposal?.status === "failed";
  const isFinalized = proposal?.status === "finalized";

  const isSupportPhaseView =
    proposal?.status === "supporting" ||
    proposal?.status === "discussion" ||
    proposal?.status === "failed";

  return (
    <div className="space-y-6 sm:space-y-8">
      <ProposalBreadcrumb />
      <ProposalDetailHeader proposal={proposal} isLoading={isLoading} />

      {/* Support phase layout: stacked on tablet/iPad Pro, side-by-side on desktop */}
      {isSupportPhaseView ? (
        <>
          {/* Mobile only: SupportProposal above SupportPhaseProgress (supporting phase only) */}
          {isSupporting && (
            <div className="md:hidden">
              <SupportProposalSection
                proposalId={proposal?.publicKey?.toBase58()}
                proposalStatus={proposal?.status}
                disabled={!isConnected}
              />
            </div>
          )}
          <div className="flex flex-col gap-6 xl:grid xl:grid-cols-[2fr_1fr]">
            <SupportPhaseProgress proposal={proposal} />
            {/* Supporting phase: show on tablet and desktop */}
            {isSupporting && (
              <SupportProposalSection
                proposalId={proposal?.publicKey?.toBase58()}
                proposalStatus={proposal?.status}
                disabled={!isConnected}
                className="hidden md:flex"
              />
            )}
            {/* Discussion phase: show only on desktop with discussion variant */}
            {isDiscussion && (
              <SupportProposalSection
                proposalId={proposal?.publicKey?.toBase58()}
                proposalStatus={proposal?.status}
                variant="discussion"
                className="hidden xl:flex"
              />
            )}
            {/* Failed status: show only on desktop with failed variant */}
            {isFailed && (
              <SupportProposalSection
                proposalId={proposal?.publicKey?.toBase58()}
                proposalStatus={proposal?.status}
                variant="failed"
                className="hidden xl:flex"
              />
            )}
          </div>
        </>
      ) : (
        <>
          {/* Mobile only: CastVote above VoteBreakdown (voting phase only) */}
          {isFinalized && (
            <div className="md:hidden">
              <CastVoteWrapper proposal={proposal} isLoading={isLoading} />
            </div>
          )}
          <div className="grid gap-6 md:grid-cols-[2fr_1fr] xl:grid-cols-[2fr_1fr]">
            <VoteBreakdown proposal={proposal} isLoading={isLoading} />
            {isFinalized ? (
              <SupportProposalSection
                proposalId={proposal?.publicKey?.toBase58()}
                proposalStatus={proposal?.status}
                variant="finalized"
                className="hidden md:flex"
              />
            ) : (
              <div className="hidden md:block">
                <CastVoteWrapper proposal={proposal} isLoading={isLoading} />
              </div>
            )}
          </div>
        </>
      )}
      <PhaseTimeline proposal={proposal} isLoading={isLoading} />
      <TopVotersTable proposal={proposal} />
    </div>
  );
}
