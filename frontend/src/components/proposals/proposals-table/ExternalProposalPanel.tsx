"use client";

import Link from "next/link";
import { AppButton } from "@/components/ui/AppButton";
import { GitHubIcon } from "@/components/icons/SvgIcons";
import { Circle, Loader, X } from "lucide-react";
import { useModal } from "@/contexts/ModalContext";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useWallet } from "@solana/wallet-adapter-react";
import { ProposalDescription } from "../ProposalDescription";
import { ProposalRecord, ProposalStatus } from "@/types";
import { useChainVoteAccount, useWalletRole } from "@/hooks";
import { SupportButton } from "../SupportButton";
import { PublicKey } from "@solana/web3.js";
import { toast } from "sonner";
import { getProposalDetailPagePath } from "@/helpers/proposalPage";
import { getVoteModalNames } from "@/lib/governance/role-detection";
import { useGovernanceConfigContext } from "@/contexts/GovernanceConfigContext";
import {
  supportPhaseRequirementCopy,
  supportThresholdPercentFromConfig,
} from "@/lib/proposals";
import { FAILED_PHASE_DETAIL } from "../detail/phase-timeline/constants";

const VOTE_STATE_LABEL: Record<ProposalRecord["status"], string> = {
  supporting: "Supporting",
  discussion: "Discussion",
  voting: "In Progress",
  finalized: "Finished",
  failed: "Failed",
};

const STAGE_ORDER: ProposalStatus[] = [
  "supporting",
  "discussion",
  "voting",
  "finalized",
];

const STAGE_LABEL: Record<(typeof STAGE_ORDER)[number], string> = {
  supporting: "Supporting",
  discussion: "Discussion",
  voting: "Voting",
  finalized: "Finished",
};

const STAGE_DESCRIPTION: Record<(typeof STAGE_ORDER)[number], string> = {
  supporting: "",
  discussion:
    "The discussion phase covers the 4-5 epoch period while the NCN is created. Voting begins only after this process completes.",
  voting:
    "Validators vote on active governance proposals. Delegators can override their validator's vote using stake account verification.",
  finalized:
    "Voting period has ended and all votes have been counted. The proposal is finalized and ready for on-chain execution.",
};

const getVoteStateLabel = (proposal: ProposalRecord): string => {
  if (proposal.status === "failed") {
    return "Ended";
  }
  return VOTE_STATE_LABEL[proposal.vote.state];
};

const getHeaderLabel = (proposal: ProposalRecord): string => {
  if (proposal.status === "failed") {
    return "Stage";
  }
  return proposal.status === "finalized" ? "Vote" : "Stage";
};

function ProposalInfo({ proposal }: { proposal: ProposalRecord }) {
  return (
    <div className="flex flex-1 flex-col justify-between gap-6">
      <Link
        href={getProposalDetailPagePath(proposal.publicKey)}
        className="space-y-3 block"
      >
        <h3 className="h3 whitespace-pre-wrap text-lg font-semibold tracking-tight text-white sm:text-xl hover-gradient-text transition-all duration-200">
          {proposal.simd && `${proposal.simd}: `}
          {proposal.title}
        </h3>
        <ProposalDescription githubUrl={proposal.description} />
      </Link>

      <AppButton
        asChild
        variant="outline"
        size="sm"
        className="w-fit border-white/20  text-[11px] font-medium uppercase tracking-[0.1em] text-white/70 hover:bg-white/20"
      >
        <Link
          href={proposal.description}
          target="_blank"
          rel="noreferrer"
          className="inline-flex items-center gap-2"
        >
          <GitHubIcon />
          Link to proposal
        </Link>
      </AppButton>
    </div>
  );
}

function LifecycleStageBar({ stage }: { stage: ProposalStatus }) {
  const isFailed = stage === "failed";
  const activeIndex = isFailed ? 0 : Math.max(STAGE_ORDER.indexOf(stage), 0);
  const governanceConfigQuery = useGovernanceConfigContext();
  const thresholdPercent = supportThresholdPercentFromConfig(
    governanceConfigQuery.data,
  );

  const getDescription = (value: (typeof STAGE_ORDER)[number]) => {
    if (isFailed && value === "supporting") {
      return FAILED_PHASE_DETAIL.body;
    }
    if (value === "supporting") {
      return `${supportPhaseRequirementCopy(thresholdPercent)}.`;
    }
    return STAGE_DESCRIPTION[value];
  };

  // Match LifecycleIndicator: Loader for in-progress stages, Circle for
  // finalized, X for failed support.
  const getStageIcon = (value: (typeof STAGE_ORDER)[number]) => {
    if (isFailed && value === "supporting") {
      return <X className="size-4 text-white" strokeWidth={2.5} />;
    }
    if (value === "finalized") {
      return <Circle className="size-3 fill-white text-white" />;
    }
    return <Loader className="size-4 animate-spin text-white" />;
  };

  return (
    <div className="flex w-full items-center gap-2">
      {STAGE_ORDER.map((value, index) => (
        <Tooltip key={value}>
          <TooltipTrigger asChild>
            <button
              type="button"
              aria-label={`${STAGE_LABEL[value]} stage`}
              className={`h-2 flex-1 rounded-full transition-transform duration-200 ease-out will-change-transform hover:scale-120 focus-visible:scale-110 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40 ${
                index <= activeIndex
                  ? "bg-green-600 hover:bg-green-500"
                  : "bg-white/10 hover:bg-white/30"
              }`}
            />
          </TooltipTrigger>
          <TooltipContent
            side="top"
            sideOffset={8}
            className="w-[240px] rounded-xl border border-white/10 bg-white/10 px-4 py-3 text-left shadow-xl backdrop-blur-md"
          >
            <div className="flex flex-col gap-2">
              <div className="flex items-center gap-2">
                {getStageIcon(value)}
                <p className="text-sm font-semibold text-white">
                  {STAGE_LABEL[value]}
                </p>
              </div>
              <p className="text-xs leading-[1.5] whitespace-pre-wrap text-white">
                {getDescription(value)}
              </p>
            </div>
          </TooltipContent>
        </Tooltip>
      ))}
    </div>
  );
}

function VoteActions({
  state,
  proposalId,
  consensusResult,
  disabled,
}: {
  state: ProposalStatus;
  proposalId?: string;
  consensusResult?: PublicKey;
  disabled?: boolean;
}) {
  const { openModal } = useModal();
  const { publicKey } = useWallet();
  const { isLoading: isLoadingWalletRole } = useWalletRole(
    publicKey?.toBase58(),
  );
  const { data: chainVoteAccount, isLoading: isLoadingChainVoteAccount } =
    useChainVoteAccount(publicKey?.toBase58());

  const isLoadingVoteIdentity =
    isLoadingWalletRole || isLoadingChainVoteAccount;
  const { castModalName, modifyModalName } =
    getVoteModalNames(chainVoteAccount);

  const isVoting = state === "voting";

  return (
    <div className="flex flex-col gap-3">
      {isVoting && (
        <>
          <AppButton
            variant="outline"
            text="Modify Vote"
            className="w-full justify-center border-white/15 bg-white/10 text-sm font-medium text-white/75 hover:text-white"
            disabled={
              disabled || consensusResult === undefined || isLoadingVoteIdentity
            }
            onClick={() => {
              if (consensusResult && proposalId) {
                openModal(modifyModalName, { proposalId, consensusResult });
              } else if (isLoadingVoteIdentity) {
                toast.error("Loading wallet voting identity");
              } else {
                toast.error("Couldn't obtain consensus result");
              }
            }}
          />
          <AppButton
            variant="gradient"
            text="Cast Vote"
            className="w-full justify-center text-sm font-semibold text-foreground"
            disabled={
              disabled || consensusResult === undefined || isLoadingVoteIdentity
            }
            onClick={() => {
              if (consensusResult && proposalId) {
                openModal(castModalName, { proposalId, consensusResult });
              } else if (isLoadingVoteIdentity) {
                toast.error("Loading wallet voting identity");
              } else {
                toast.error("Couldn't obtain consensus result");
              }
            }}
          />
        </>
      )}
      <SupportButton
        proposalStatus={state}
        proposalId={proposalId}
        disabled={disabled}
      />
    </div>
  );
}

function DiscussionMessage({ proposalId }: { proposalId: string }) {
  return (
    <div className="space-y-3">
      <p className="text-sm leading-relaxed text-white/70">
        This proposal is in the discussion phase.
        <br />
        No actions are available at this time.
      </p>
      <AppButton
        asChild
        variant="outline"
        className="w-full justify-center border-white/15 bg-white/10 text-sm font-medium text-white/75 hover:text-white"
      >
        <Link href={getProposalDetailPagePath(proposalId)}>View Details</Link>
      </AppButton>
    </div>
  );
}

function VotingPanel({ proposal }: { proposal: ProposalRecord }) {
  const { connected } = useWallet();

  const isVoting = proposal.status === "voting";
  const isSupporting = proposal.status === "supporting";
  const isDiscussion = proposal.status === "discussion";

  return (
    <aside className="w-full glass-card p-6 lg:w-80 xl:w-80">
      <header className="mb-6">
        <span className="block text-[11px] uppercase tracking-[0.24em] text-white/45 mb-3">
          {getHeaderLabel(proposal)}
        </span>
        <div className="flex items-center justify-between gap-4">
          <span className="text-lg font-semibold text-white">
            {getVoteStateLabel(proposal)}
          </span>
          <div className="min-w-28 max-w-36 flex-1">
            <LifecycleStageBar stage={proposal.status} />
          </div>
        </div>
      </header>

      {connected ? (
        <>
          {isDiscussion && (
            <DiscussionMessage proposalId={proposal.publicKey.toBase58()} />
          )}
          {(isSupporting || isVoting) && (
            <VoteActions
              state={proposal.status}
              proposalId={proposal.publicKey.toBase58()}
              consensusResult={proposal.consensusResult}
            />
          )}
        </>
      ) : (
        <Tooltip>
          <TooltipTrigger asChild>
            <span>
              {isSupporting ||
                (isVoting && <VoteActions state={proposal.status} disabled />)}
            </span>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            <p className="text-sm text-red-500/80">
              Wallet not connected, please connect your wallet to be able to
              perform these actions
            </p>
          </TooltipContent>
        </Tooltip>
      )}
    </aside>
  );
}

type ExternalProposalPanelProps = {
  proposal: ProposalRecord;
};

export default function ExternalProposalPanel({
  proposal,
}: ExternalProposalPanelProps) {
  return (
    <div className="flex flex-col gap-6 p-6 lg:flex-row lg:items-stretch xl:gap-8">
      <ProposalInfo proposal={proposal} />
      <div className="lg:ml-auto">
        <VotingPanel proposal={proposal} />
      </div>
    </div>
  );
}
