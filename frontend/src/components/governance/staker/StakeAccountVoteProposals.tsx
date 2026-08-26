import React from "react";
import { useStakerVotedProposals } from "@/hooks";
import { StakeAccountData } from "@/types/stakeAccounts";
import { Card } from "@/components/ui/Card";
import { CopyableAddress } from "@/components/governance/shared/CopyableAddress";
import { ProposalHeading } from "@/components/proposals/ProposalHeading";
import { formatLamportsDisplay } from "@/lib/governance/formatters";
import Link from "next/link";
import { getProposalDetailPagePath } from "@/helpers/proposalPage";

interface StakeAccountVoteProposalsProps {
  stakeAccount: StakeAccountData;
}

export function StakeAccountVoteProposals({
  stakeAccount,
}: StakeAccountVoteProposalsProps) {
  const {
    data: voteProposals,
    isLoading,
    error,
  } = useStakerVotedProposals(stakeAccount);

  if (isLoading) {
    return (
      <div className="p-4">
        <div className="animate-pulse space-y-3">
          <div className="h-4 bg-white/10 rounded w-1/4"></div>
          <div className="space-y-2">
            <div className="h-3 bg-white/10 rounded"></div>
            <div className="h-3 bg-white/10 rounded w-3/4"></div>
          </div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-4 text-red-400 text-sm">
        Error loading vote proposals: {error.message}
      </div>
    );
  }

  if (!voteProposals || voteProposals.length === 0) {
    return (
      <div className="p-4 text-white/60 text-sm">
        {!stakeAccount.voteAccount
          ? "This stake account is not delegated to a validator."
          : "No vote proposals found for this stake account's delegated validator."}
      </div>
    );
  }

  return (
    <div className="p-4 space-y-4">
      <div className="space-y-2">
        <h4 className="text-sm font-semibold text-white/80">
          Proposals Participated In ({voteProposals.length})
        </h4>
        <div className="text-xs text-white/60">
          Votes cast by validator{" "}
          {stakeAccount.voteAccount ? (
            <CopyableAddress
              address={stakeAccount.voteAccount}
              shortenedLength={8}
              copyLabel="Copy validator address"
            />
          ) : (
            "Unknown"
          )}{" "}
          using delegated stake power
        </div>
      </div>

      <div className="space-y-3">
        {voteProposals.map((voteProposal) => (
          <Card
            key={voteProposal.votePublicKey}
            className="p-3 bg-white/5 border-white/10"
          >
            <div className="space-y-2">
              {/* Proposal Title */}
              <div className="flex items-start justify-between">
                <Link
                  href={getProposalDetailPagePath(
                    voteProposal.proposal.publicKey
                  )}
                  className="space-y-3 block"
                >
                  <h3 className="h3 whitespace-pre-wrap text-lg font-semibold tracking-tight text-white sm:text-xl hover-gradient-text transition-all duration-200">
                    <ProposalHeading
                      url={voteProposal.proposal.description}
                      fallback={voteProposal.proposal.proposalRef}
                      title={voteProposal.proposal.title}
                    />
                  </h3>
                </Link>
                <span
                  className={`text-xs px-2 py-1 rounded-full ${
                    voteProposal.proposal.status === "voting"
                      ? "bg-blue-500/20 text-blue-400"
                      : voteProposal.proposal.status === "finalized"
                      ? "bg-green-500/20 text-green-400"
                      : "bg-yellow-500/20 text-yellow-400"
                  }`}
                >
                  {voteProposal.proposal.status}
                </span>
              </div>

              {/* Proposal Description */}
              {/* <ProposalDescription
                githubUrl={voteProposal.proposal.description}
              /> */}

              {/* Vote Details */}
              <div className="flex items-center justify-between pt-2 border-t border-white/10">
                <div className="flex items-center space-x-4 text-xs">
                  <div className="flex items-center space-x-1">
                    <span className="text-green-400">●</span>
                    <span className="text-white/70">
                      {(voteProposal.voteAccount.forVotesBp / 100n).toString()}%
                      For
                    </span>
                  </div>
                  <div className="flex items-center space-x-1">
                    <span className="text-red-400">●</span>
                    <span className="text-white/70">
                      {(voteProposal.voteAccount.againstVotesBp / 100n).toString()}
                      % Against
                    </span>
                  </div>
                  <div className="flex items-center space-x-1">
                    <span className="text-gray-400">●</span>
                    <span className="text-white/70">
                      {(voteProposal.voteAccount.abstainVotesBp / 100n).toString()}
                      % Abstain
                    </span>
                  </div>
                </div>

                {/* Vote Amount */}
                <div className="text-xs text-white/60">
                  Total:{" "}
                  {
                    formatLamportsDisplay(
                      voteProposal.voteAccount.stakeAmount ?? 0n
                    ).value
                  }
                </div>
              </div>

              {/* Delegation Info */}
              <div className="text-xs text-white/50 pt-1">
                Your stake contributed to this validator&apos;s vote decision
              </div>

              {/* Proposal Link */}
              <div className="flex items-center justify-between pt-1">
                <CopyableAddress
                  address={voteProposal.proposal.publicKey}
                  shortenedLength={8}
                  copyLabel="Copy proposal address"
                />
                <div className="text-xs text-white/50">
                  Vote ID: {voteProposal.votePublicKey.slice(0, 8)}...
                </div>
              </div>
            </div>
          </Card>
        ))}
      </div>
    </div>
  );
}
