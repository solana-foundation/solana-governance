import { createProgramWitDummyWallet } from "@/chain";
import { getProposalRefFromUrl } from "@/lib/github";
import type { GovernanceConfigDto } from "@/lib/getGovernanceConfig";
import {
  epochConstantsFromGovernanceConfig,
  getProposalStatus,
  type EpochConstants,
} from "@/lib/proposals";
import type { ProposalRecord, RawProposalAccount } from "@/types";
import {
  Connection,
  EpochInfo,
  PublicKey,
  VoteAccountInfo,
} from "@solana/web3.js";

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

  const consensusPending = await getConsensusPending(
    program.provider.connection,
    proposalAccs,
    currentEpoch,
  );

  let data = proposalAccs.map((acc, index) =>
    mapProposalDto(
      acc,
      index,
      currentEpoch,
      totalStakedLamports,
      epochConstants,
      governanceConfig.clusterSupportPctMinBps,
      !consensusPending.has(acc.publicKey.toBase58()),
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

/**
 * Proposals store the ConsensusResult PDA as soon as support is reached, but
 * the account only exists once the NCN has finalized the snapshot ballot;
 * until then the program rejects every vote. Returns the set of proposal
 * keys whose ConsensusResult account could not be confirmed to exist.
 *
 * Only proposals inside their voting window are checked. A lookup that
 * still fails after a few retries (RPC error, rate limit) does not block the
 * whole list; that proposal is reported as pending, since enabling vote
 * controls without knowing the account exists would only produce failing
 * transactions, and useProposals keeps polling while any proposal is
 * pending so the status recovers without a reload.
 */
export async function getConsensusPending(
  connection: Connection,
  proposalAccs: RawProposalAccount[],
  currentEpoch: number,
): Promise<Set<string>> {
  const candidates = proposalAccs.filter(
    (acc) =>
      acc.account.voting &&
      !acc.account.finalized &&
      acc.account.consensusResult &&
      currentEpoch >= acc.account.startEpoch.toNumber() &&
      currentEpoch < acc.account.endEpoch.toNumber(),
  );
  const results = await Promise.allSettled(
    candidates.map((acc) =>
      getAccountInfoWithRetry(
        connection,
        acc.account.consensusResult as PublicKey,
      ),
    ),
  );
  return new Set(
    candidates
      .filter((_, i) => {
        const result = results[i];
        return result.status === "rejected" || result.value === null;
      })
      .map((acc) => acc.publicKey.toBase58()),
  );
}

const CONSENSUS_LOOKUP_ATTEMPTS = 3;
const CONSENSUS_LOOKUP_BACKOFF_MS = 500;

async function getAccountInfoWithRetry(
  connection: Connection,
  account: PublicKey,
) {
  let lastError: unknown;
  for (let attempt = 0; attempt < CONSENSUS_LOOKUP_ATTEMPTS; attempt++) {
    if (attempt > 0) {
      await new Promise((resolve) =>
        setTimeout(resolve, CONSENSUS_LOOKUP_BACKOFF_MS * attempt),
      );
    }
    try {
      return await connection.getAccountInfo(account);
    } catch (error) {
      lastError = error;
    }
  }
  throw lastError;
}

export function mapProposalDto(
  rawAccount: RawProposalAccount,
  index: number,
  currentEpoch: number,
  totalStakedLamports: number,
  epochConstants: EpochConstants,
  clusterSupportPctMinBps: number,
  consensusReached: boolean,
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
    clusterSupportPctMinBps,
    consensusResult,
    consensusReached,
    finalized,
    voting: raw.voting,
    epochConstants,
  });

  const proposalRef = getProposalRefFromUrl(raw.description);

  return {
    publicKey: rawAccount.publicKey,
    id: index.toString(),
    proposalRef,
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

    proposerStakeWeightBp: raw.proposerStakeWeightBp?.toNumber() || 0,

    status,
    voting: raw.voting,
    finalized,

    consensusResult,
    consensusReached,
    snapshotSlot: raw.snapshotSlot.toNumber(),

    proposalBump: raw.proposalBump,
    index: raw.index,

    vote: {
      state: status,
      lastUpdated: "raw.voteCount.toString()",
    },
  };
}
