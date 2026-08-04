import { computeSupportStats } from "../stats";

const TOTAL = 100_000_000_000;

const base = {
  currentSupportLamports: 0,
  totalStakedLamports: TOTAL,
  thresholdPercent: 15,
  validatorCount: 0,
  numOfValidators: 100,
};

describe("computeSupportStats", () => {
  it("measures progress against the configured threshold", () => {
    // 9% of stake against a 15% threshold is 60% of the way there.
    const stats = computeSupportStats({
      ...base,
      currentSupportLamports: TOTAL * 0.09,
    });
    expect(stats.progressPercent).toBeCloseTo(60, 10);
    expect(stats.supportPercentOfTotal).toBeCloseTo(9, 10);
    expect(stats.isThresholdMet).toBe(false);
    expect(stats.remainingLamports).toBeCloseTo(TOTAL * 0.06, 0);
  });

  it("meets the threshold exactly at the configured percentage", () => {
    const stats = computeSupportStats({
      ...base,
      currentSupportLamports: TOTAL * 0.15,
    });
    expect(stats.isThresholdMet).toBe(true);
    expect(stats.remainingLamports).toBe(0);
  });

  it("does not report success before validator stake is known", () => {
    // Regression: validator stake arrives from a separate query, so the required
    // threshold is 0 on first paint and `support >= 0` was true — the panel
    // showed "Support threshold reached! Proposal advancing to next phase."
    const stats = computeSupportStats({
      ...base,
      currentSupportLamports: TOTAL * 0.01,
      totalStakedLamports: 0,
    });
    expect(stats.isThresholdMet).toBe(false);
    expect(stats.progressPercent).toBe(0);
    expect(stats.supportPercentOfTotal).toBe(0);
  });

  it("treats a configured 0% threshold as met, matching the program", () => {
    // The program permits 0..=10000 bps and its own check is
    // `support >= cluster_min_stake`, which any support satisfies at 0 bps.
    // Gating on stake rather than on the derived threshold keeps the two in
    // agreement instead of reporting unmet for a legitimate config.
    const stats = computeSupportStats({
      ...base,
      thresholdPercent: 0,
      currentSupportLamports: 1,
    });
    expect(stats.requiredThresholdLamports).toBe(0);
    expect(stats.isThresholdMet).toBe(true);
  });

  it("still withholds success at 0% while stake is unknown", () => {
    // The two zero-threshold causes must not collapse into one another.
    const stats = computeSupportStats({
      ...base,
      thresholdPercent: 0,
      totalStakedLamports: 0,
      currentSupportLamports: 1,
    });
    expect(stats.isThresholdMet).toBe(false);
  });

  it("does not divide by a validator count of zero", () => {
    // Regression: participationPercent was NaN whenever the validator query had
    // not resolved, which rendered as "NaN%".
    const stats = computeSupportStats({
      ...base,
      validatorCount: 0,
      numOfValidators: 0,
    });
    expect(stats.participationPercent).toBe(0);
    expect(stats.avgStakePerValidator).toBe(0);
  });

  it("reports participation against the validator count", () => {
    const stats = computeSupportStats({
      ...base,
      currentSupportLamports: TOTAL * 0.2,
      validatorCount: 25,
      numOfValidators: 100,
    });
    expect(stats.participationPercent).toBe(25);
    expect(stats.avgStakePerValidator).toBeCloseTo((TOTAL * 0.2) / 25, 0);
  });

  it("allows progress to exceed 100% once the threshold is passed", () => {
    const stats = computeSupportStats({
      ...base,
      currentSupportLamports: TOTAL * 0.3,
    });
    expect(stats.progressPercent).toBeCloseTo(200, 10);
    expect(stats.isThresholdMet).toBe(true);
  });

  describe("thresholdCrossed (on-chain verdict)", () => {
    it("overrides a live estimate that falls just short", () => {
      // The program measured the crossing against the epoch-stakes total of the
      // crossing epoch, which the RPC does not expose. Live stake has since
      // grown, so a proposal the chain already advanced computes to ~99.9%
      // with tens of thousands of SOL "missing".
      const stats = computeSupportStats({
        ...base,
        currentSupportLamports: TOTAL * 0.1499,
        thresholdCrossed: true,
      });
      expect(stats.isThresholdMet).toBe(true);
      expect(stats.progressPercent).toBe(100);
      expect(stats.remainingLamports).toBe(0);
    });

    it("reports met even while live stake is unknown", () => {
      // The chain's verdict does not depend on the validator query resolving.
      const stats = computeSupportStats({
        ...base,
        totalStakedLamports: 0,
        thresholdCrossed: true,
      });
      expect(stats.isThresholdMet).toBe(true);
      expect(stats.progressPercent).toBe(100);
      expect(stats.remainingLamports).toBe(0);
    });

    it("does not cap live progress that already exceeds 100%", () => {
      const stats = computeSupportStats({
        ...base,
        currentSupportLamports: TOTAL * 0.3,
        thresholdCrossed: true,
      });
      expect(stats.progressPercent).toBeCloseTo(200, 10);
    });

    it("changes nothing while the proposal has not crossed", () => {
      const stats = computeSupportStats({
        ...base,
        currentSupportLamports: TOTAL * 0.1499,
        thresholdCrossed: false,
      });
      expect(stats.isThresholdMet).toBe(false);
      expect(stats.progressPercent).toBeCloseTo(99.9333, 3);
      expect(stats.remainingLamports).toBeCloseTo(TOTAL * 0.0001, 0);
    });
  });
});
