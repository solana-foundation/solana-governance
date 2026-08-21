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
  forLamports: number;
  againstLamports: number;
  abstainLamports: number;
  /**
   * Total active stake in the proposal's snapshot. Art. IV.2 fixes the stake
   * distribution for the whole voting period, so a live cluster total is not a
   * substitute — it drifts as stake moves.
   */
  totalActiveStake: number | undefined;
}

/**
 * `known: false` is not zero participation. Collapsing the two would report
 * "0%, quorum not met" for a vote that may have met it.
 */
export type QuorumStatus =
  | { known: false }
  | {
      known: true;
      totalActiveStake: number;
      /** 0–100. */
      participationPercent: number;
      isMet: boolean;
    };

export function computeQuorum({
  forLamports,
  againstLamports,
  abstainLamports,
  totalActiveStake,
}: QuorumInput): QuorumStatus {
  if (totalActiveStake === undefined || totalActiveStake <= 0) {
    return { known: false };
  }

  const participatingLamports = forLamports + againstLamports + abstainLamports;

  return {
    known: true,
    totalActiveStake,
    participationPercent: (participatingLamports / totalActiveStake) * 100,
    // Compared in lamports, not on the rounded percentage, so display precision
    // cannot decide a borderline vote. Totals at mainnet scale exceed
    // Number.MAX_SAFE_INTEGER, leaving ~64 lamports of resolution — the
    // precision every other *Lamports field here already carries.
    isMet:
      participatingLamports >=
      (totalActiveStake * QUORUM_NUMERATOR) / QUORUM_DENOMINATOR,
  };
}

/** The `/meta` fields quorum needs. */
export interface SnapshotTotalSource {
  slot: number;
  total_active_stake?: number | null;
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
  proposalSnapshotSlot: number | undefined,
): number | undefined {
  if (!meta || !proposalSnapshotSlot) return undefined;
  if (meta.slot !== proposalSnapshotSlot) return undefined;
  return meta.total_active_stake ?? undefined;
}
