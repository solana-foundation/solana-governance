import { createProgramWitDummyWallet } from "@/chain";
import { getSimd } from "@/hooks";
import type { GovernanceConfigDto } from "@/lib/getGovernanceConfig";
import {
  epochConstantsFromGovernanceConfig,
  getProposalStatus,
  type EpochConstants,
} from "@/lib/proposals";
import type { ProposalRecord, RawProposalAccount } from "@/types";
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
  const program = createProgramWitDummyWallet(endpoint);
  const epochConstants = epochConstantsFromGovernanceConfig(governanceConfig);

  // Fetch proposals
  const proposalAccs = await program.account.proposal.all();

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
      acc,
      index,
      currentEpoch,
      totalStakedLamports,
      epochConstants,
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
  rawAccount: RawProposalAccount,
  index: number,
  currentEpoch: number,
  totalStakedLamports: number,
  epochConstants: EpochConstants,
): ProposalRecord {
  const raw = rawAccount.account;
  const creationEpoch = raw.creationEpoch.toNumber();
  const startEpoch = raw.startEpoch.toNumber();
  const endEpoch = raw.endEpoch.toNumber();
  const clusterSupportLamports = +raw.clusterSupportLamports?.toString() || 0;
  const consensusResult = rawAccount.account.consensusResult || undefined;
  const finalized = raw.finalized;

  const status = getProposalStatus({
    creationEpoch,
    startEpoch,
    endEpoch,
    currentEpoch,
    clusterSupportLamports,
    totalStakedLamports,
    consensusResult,
    finalized,
    voting: raw.voting,
    epochConstants,
  });

  const simd = getSimd(raw.description);

  return {
    publicKey: rawAccount.publicKey,
    id: index.toString(),
    simd,
    title: raw.title,
    description: raw.description,
    author: raw.author.toBase58(),

    creationEpoch,
    startEpoch,
    endEpoch,
    creationTimestamp: raw.creationTimestamp?.toNumber() || 0,

    clusterSupportLamports,
    forVotesLamports: raw.forVotesLamports
      ? +raw.forVotesLamports.toString()
      : 0,
    againstVotesLamports: raw.againstVotesLamports
      ? +raw.againstVotesLamports.toString()
      : 0,
    abstainVotesLamports: raw.abstainVotesLamports
      ? +raw.abstainVotesLamports.toString()
      : 0,
    voteCount: raw.voteCount,

    quorumPercent: 60, // TODO ?
    proposerStakeWeightBp: raw.proposerStakeWeightBp?.toNumber() || 0,

    status,
    voting: raw.voting,
    finalized,

    consensusResult,

    proposalBump: raw.proposalBump,
    index: raw.index,

    vote: {
      state: status,
      lastUpdated: "raw.voteCount.toString()",
    },
  };
}
