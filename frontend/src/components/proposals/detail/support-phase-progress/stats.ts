export interface SupportStatsInput {
  currentSupportLamports: number;
  totalStakedLamports: number;
  /** Already resolved via `supportThresholdPercentFromConfig`. */
  thresholdPercent: number;
  validatorCount: number;
  numOfValidators: number;
  /**
   * On-chain record that the threshold was crossed (`proposal.voting`, set by
   * the program's `activate_voting`). The program measured against the
   * epoch-stakes total of the crossing epoch, which the RPC does not expose,
   * so the live-stake math below can disagree — a proposal that barely crossed
   * renders as ~99.9% otherwise. When set, the account's verdict wins.
   */
  thresholdCrossed?: boolean;
}

export interface SupportStats {
  currentSupportLamports: number;
  totalStakedLamports: number;
  requiredThresholdLamports: number;
  thresholdPercent: number;
  progressPercent: number;
  supportPercentOfTotal: number;
  remainingLamports: number;
  isThresholdMet: boolean;
  validatorCount: number;
  participationPercent: number;
  avgStakePerValidator: number;
}

/**
 * Support-phase figures for the proposal detail page. Pure so the zero states
 * are testable: validator stake and the supporter list arrive from separate
 * queries, and either being absent must not read as progress.
 */
export function computeSupportStats({
  currentSupportLamports,
  totalStakedLamports,
  thresholdPercent,
  validatorCount,
  numOfValidators,
  thresholdCrossed = false,
}: SupportStatsInput): SupportStats {
  const requiredThresholdLamports =
    totalStakedLamports * (thresholdPercent / 100);

  const liveProgressPercent =
    requiredThresholdLamports > 0
      ? (currentSupportLamports / requiredThresholdLamports) * 100
      : 0;

  // A crossed proposal is never shown below 100%, even when the live estimate
  // disagrees with the frozen on-chain tally.
  const progressPercent = thresholdCrossed
    ? Math.max(liveProgressPercent, 100)
    : liveProgressPercent;

  const supportPercentOfTotal =
    totalStakedLamports > 0
      ? (currentSupportLamports / totalStakedLamports) * 100
      : 0;

  const remainingLamports = thresholdCrossed
    ? 0
    : Math.max(0, requiredThresholdLamports - currentSupportLamports);

  // Gate on total stake, not on the derived threshold: a zero threshold has two
  // very different causes. Stake not loaded yet means nothing is known, and
  // `support >= 0` would claim success on first paint. A configured 0% (the
  // program permits 0..=10000 bps) genuinely is satisfied by any support, which
  // is what the on-chain check does, so it must still report met.
  const isThresholdMet =
    thresholdCrossed ||
    (totalStakedLamports > 0 &&
      currentSupportLamports >= requiredThresholdLamports);

  const participationPercent =
    numOfValidators > 0 ? (validatorCount / numOfValidators) * 100 : 0;
  const avgStakePerValidator =
    validatorCount > 0 ? currentSupportLamports / validatorCount : 0;

  return {
    currentSupportLamports,
    totalStakedLamports,
    requiredThresholdLamports,
    thresholdPercent,
    progressPercent,
    supportPercentOfTotal,
    remainingLamports,
    isThresholdMet,
    validatorCount,
    participationPercent,
    avgStakePerValidator,
  };
}
