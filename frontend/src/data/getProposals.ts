import { getProposalRefFromUrl } from "@/lib/github";
import { fetchProposals, type Proposal } from "@/lib/governance/programAccounts";
import { toLegacyPublicKey } from "@/lib/governance/legacyAdapters";
import type { GovernanceConfigDto } from "@/lib/getGovernanceConfig";
import {
  epochConstantsFromGovernanceConfig,
  getProposalStatus,
  type EpochConstants,
} from "@/lib/proposals";
import type { ProposalRecord } from "@/types";
import { createSolanaRpc, unwrapOption } from "@solana/kit";
import { EpochInfo, VoteAccountInfo } from "@solana/web3.js";

export interface RawVoteAccountsData {
  current: VoteAccountInfo[];
  delinquent: VoteAccountInfo[];
}

export const getProposals = async (
  endpoint: string,
  filters:
    | {
      voting?: boolean;
      finalized?: boolean;
    }
    | undefined,
  epochInfo: EpochInfo,
  voteAccountsData: RawVoteAccountsData,
  governanceConfig: GovernanceConfigDto,
): Promise<ProposalRecord[]> => {
  const epochConstants = epochConstantsFromGovernanceConfig(governanceConfig);
  const proposalAccs = await fetchProposals(createSolanaRpc(endpoint));

  // Calculate total staked lamports from all vote accounts
  const allVotes = [
    ...voteAccountsData.current,
    // ...voteAccountsData.delinquent,
  ];
  const totalStakedLamports = allVotes.reduce(
    (sum, vote) => sum + (vote.activatedStake || 0),
    0,
  );

  const currentEpoch = epochInfo.epoch;

  let data = proposalAccs.map((acc, index) =>
    mapProposalDto(
      acc.data,
      acc.address,
      index,
      currentEpoch,
      totalStakedLamports,
      epochConstants,
      governanceConfig.clusterSupportPctMinBps,
    ),
  );

  if (filters) {
    if (filters.voting !== undefined) {
      data = data.filter((proposal) => proposal.voting === filters.voting);
    }
    if (filters.finalized !== undefined) {
      data = data.filter(
        (proposal) => proposal.finalized === filters.finalized,
      );
    }
  }

  data = data.sort((a, b) => b.creationTimestamp - a.creationTimestamp);

  return data;
};

export function mapProposalDto(
  raw: Proposal,
  address: string,
  index: number,
  currentEpoch: number,
  totalStakedLamports: number,
  epochConstants: EpochConstants,
  clusterSupportPctMinBps: number,
): ProposalRecord {
  const creationEpoch = Number(raw.creationEpoch);
  const startEpoch = Number(raw.startEpoch);
  const endEpoch = Number(raw.endEpoch);
  const clusterSupportLamports = Number(raw.clusterSupportLamports);
  const consensusResult = unwrapOption(raw.consensusResult);
  const consensusResultPublicKey = consensusResult
    ? toLegacyPublicKey(consensusResult)
    : undefined;
  const finalized = raw.finalized;

  const status = getProposalStatus({
    creationEpoch,
    startEpoch,
    endEpoch,
    currentEpoch,
    clusterSupportLamports,
    totalStakedLamports,
    clusterSupportPctMinBps,
    consensusResult: consensusResultPublicKey,
    finalized,
    voting: raw.voting,
    epochConstants,
  });

  const proposalRef = getProposalRefFromUrl(raw.description);

  return {
    publicKey: toLegacyPublicKey(address),
    id: index.toString(),
    proposalRef,
    title: raw.title,
    description: raw.description,
    author: raw.author,

    creationEpoch,
    startEpoch,
    endEpoch,
    creationTimestamp: Number(raw.creationTimestamp),

    clusterSupportLamports,
    forVotesLamports: raw.forVotesLamports
      ? Number(raw.forVotesLamports)
      : 0,
    againstVotesLamports: raw.againstVotesLamports
      ? Number(raw.againstVotesLamports)
      : 0,
    abstainVotesLamports: raw.abstainVotesLamports
      ? Number(raw.abstainVotesLamports)
      : 0,
    voteCount: raw.voteCount,

    quorumPercent: 60, // TODO ?
    proposerStakeWeightBp: Number(raw.proposerStakeWeightBp),

    status,
    voting: raw.voting,
    finalized,

    consensusResult: consensusResultPublicKey,
    snapshotSlot: Number(raw.snapshotSlot),

    proposalBump: raw.proposalBump,
    index: raw.index,

    vote: {
      state: status,
      lastUpdated: "raw.voteCount.toString()",
    },
  };
}
