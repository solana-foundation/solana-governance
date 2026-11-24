import { bigintPercentage } from "@/helpers/bigint";

/**
 * Quorum per SGP-0001 Art. IV.3: one-third of network stake must participate,
 * participation being `For + Against + Abstain`.
 *
 * Art. IV.4's supermajority is not implemented here. Whether `Abstain` belongs
 * in its denominator is unresolved on the SGP-0001 pull request, so a pass/fail
 * verdict would have to change mid-vote. Quorum is unaffected — both readings
 * count `Abstain` toward it.
 */
export const QUORUM_NUMERATOR = 1;
export const QUORUM_DENOMINATOR = 3;
export const QUORUM_FRACTION = QUORUM_NUMERATOR / QUORUM_DENOMINATOR;

export interface QuorumInput {
  forLamports: bigint;
  againstLamports: bigint;
  abstainLamports: bigint;
  /**
   * Total active stake in the proposal's snapshot. Art. IV.2 fixes the stake
   * distribution for the whole voting period, so a live cluster total is not a
   * substitute — it drifts as stake moves.
   */
  totalActiveStake: bigint | undefined;
}

/**
 * `known: false` is not zero participation. Collapsing the two would report
 * "0%, quorum not met" for a vote that may have met it.
 */
export type QuorumStatus =
  | { known: false }
  | {
      known: true;
      totalActiveStake: bigint;
      /** 0–100. */
      participationPercent: number;
      isMet: boolean;
    };

/** Percentages for the three recorded vote options. */
export type VotePercentageBreakdown =
  | { known: false }
  | {
      known: true;
      forPercent: number;
      againstPercent: number;
      abstainPercent: number;
    };

/** The distinct percentage views shown for a proposal. */
export interface ProposalVoteStats {
  /** Overall participation measured against the frozen snapshot stake. */
  quorum: QuorumStatus;
  /** Each option measured against all votes cast. */
  voteShare: VotePercentageBreakdown;
  /** Each option measured against the frozen snapshot stake. */
  stakeShare: VotePercentageBreakdown;
}

export function computeQuorum({
  forLamports,
  againstLamports,
  abstainLamports,
  totalActiveStake,
}: QuorumInput): QuorumStatus {
  if (totalActiveStake === undefined || totalActiveStake <= 0n) {
    return { known: false };
  }

  const participatingLamports = forLamports + againstLamports + abstainLamports;

  return {
    known: true,
    totalActiveStake,
    participationPercent: bigintPercentage(participatingLamports, totalActiveStake),
    isMet:
      participatingLamports >=
      (totalActiveStake * BigInt(QUORUM_NUMERATOR)) /
        BigInt(QUORUM_DENOMINATOR),
  };
}

/**
 * Computes the participation and vote-percentage views shown for a proposal.
 *
 * Quorum and stake share use the frozen snapshot stake. Vote share uses only
 * recorded votes, so it remains available when snapshot metadata is unavailable.
 */
export function computeProposalVoteStats(
  input: QuorumInput,
): ProposalVoteStats {
  const quorum = computeQuorum(input);
  const participatingLamports =
    input.forLamports + input.againstLamports + input.abstainLamports;

  const voteShare: VotePercentageBreakdown =
    participatingLamports > 0n
      ? {
          known: true,
          forPercent: bigintPercentage(input.forLamports, participatingLamports),
          againstPercent: bigintPercentage(input.againstLamports, participatingLamports),
          abstainPercent: bigintPercentage(input.abstainLamports, participatingLamports),
        }
      : { known: false };

  const stakeShare: VotePercentageBreakdown = quorum.known
    ? {
        known: true,
        forPercent: bigintPercentage(input.forLamports, quorum.totalActiveStake),
        againstPercent: bigintPercentage(input.againstLamports, quorum.totalActiveStake),
        abstainPercent: bigintPercentage(input.abstainLamports, quorum.totalActiveStake),
      }
    : { known: false };

  return {
    quorum,
    voteShare,
    stakeShare,
  };
}

/** The `/meta` fields quorum needs. */
export interface SnapshotTotalSource {
  slot: bigint;
  total_active_stake?: bigint | null;
}

/**
 * The total to measure a proposal's quorum against, or `undefined` if it cannot
 * be established.
 *
 * `/meta` serves only the newest snapshot, while a proposal votes against the
 * one frozen at activation. Once uploads resume the newest moves past any
 * proposal still being voted on, and its total describes a different stake
 * distribution — so it is used only when the served snapshot is the proposal's.
 * A `/meta?slot=` lookup would resolve this for any proposal.
 */
export function resolveQuorumDenominator(
  meta: SnapshotTotalSource | undefined,
  proposalSnapshotSlot: bigint | undefined,
): bigint | undefined {
  if (!meta || !proposalSnapshotSlot) return undefined;
  if (meta.slot !== proposalSnapshotSlot) return undefined;
  return meta.total_active_stake ?? undefined;
}
