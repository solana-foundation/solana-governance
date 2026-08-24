import { getProposalRefFromUrl } from "@/lib/github";
import { fetchProposals, type Proposal } from "@/lib/governance/programAccounts";
import type { GovernanceConfigDto } from "@/lib/getGovernanceConfig";
import {
  epochConstantsFromGovernanceConfig,
  getProposalStatus,
  type EpochConstants,
} from "@/lib/proposals";
import type { RawVoteAccountsData } from "@/lib/rpcVoteAccounts";
import type { ProposalRecord } from "@/types";
import { createSolanaRpc, type Address, unwrapOption } from "@solana/kit";

export interface EpochInfoData {
  absoluteSlot: bigint;
  epoch: bigint;
}

export const getProposals = async (
  endpoint: string,
  filters:
    | {
      voting?: boolean;
      finalized?: boolean;
    }
    | undefined,
  epochInfo: EpochInfoData,
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
    (sum, vote) => sum + vote.activatedStake,
    0n,
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

  data = data.sort((a, b) =>
    a.creationTimestamp === b.creationTimestamp
      ? 0
      : a.creationTimestamp > b.creationTimestamp
        ? -1
        : 1,
  );

  return data;
};

export function mapProposalDto(
  raw: Proposal,
  address: Address,
  index: number,
  currentEpoch: bigint,
  totalStakedLamports: bigint,
  epochConstants: EpochConstants,
  clusterSupportPctMinBps: bigint,
): ProposalRecord {
  const creationEpoch = raw.creationEpoch;
  const startEpoch = raw.startEpoch;
  const endEpoch = raw.endEpoch;
  const clusterSupportLamports = raw.clusterSupportLamports;
  const consensusResult = unwrapOption(raw.consensusResult);
  const consensusResultPublicKey = consensusResult ?? undefined;
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
    publicKey: address,
    id: index.toString(),
    proposalRef,
    title: raw.title,
    description: raw.description,
    author: raw.author,

    creationEpoch,
    startEpoch,
    endEpoch,
    creationTimestamp: raw.creationTimestamp,

    clusterSupportLamports,
    forVotesLamports: raw.forVotesLamports ?? 0n,
    againstVotesLamports: raw.againstVotesLamports ?? 0n,
    abstainVotesLamports: raw.abstainVotesLamports ?? 0n,
    voteCount: raw.voteCount,

    proposerStakeWeightBp: raw.proposerStakeWeightBp,

    status,
    voting: raw.voting,
    finalized,

    consensusResult: consensusResultPublicKey,
    snapshotSlot: raw.snapshotSlot,

    proposalBump: raw.proposalBump,
    index: raw.index,

    vote: {
      state: status,
      lastUpdated: "raw.voteCount.toString()",
    },
  };
}
