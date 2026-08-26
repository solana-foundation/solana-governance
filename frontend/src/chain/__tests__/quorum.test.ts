import {
  computeQuorum,
  QUORUM_DENOMINATOR,
  QUORUM_NUMERATOR,
  resolveQuorumDenominator,
} from "../quorum";

/** Mainnet-scale bigint lamport balance. */
const NETWORK_STAKE = 400_000_000n * 1_000_000_000n;
const ONE_THIRD = NETWORK_STAKE / 3n;
const ONE_SOL = 1_000_000_000n;

function quorumOf(
  totalActiveStake: bigint | undefined,
  votes: Partial<{ f: bigint; a: bigint; ab: bigint }> = {},
) {
  return computeQuorum({
    forLamports: votes.f ?? 0n,
    againstLamports: votes.a ?? 0n,
    abstainLamports: votes.ab ?? 0n,
    totalActiveStake,
  });
}

describe("computeQuorum", () => {
  it("requires one third, as SGP-0001 Art. IV.3 says", () => {
    // The value the display is built on; it was 60%, from a `// TODO ?`.
    expect(QUORUM_NUMERATOR / QUORUM_DENOMINATOR).toBeCloseTo(1 / 3, 12);
  });

  it("reports unknown for a missing or zero total", () => {
    expect(quorumOf(undefined, { f: 1n, a: 2n, ab: 3n })).toEqual({
      known: false,
    });
    expect(quorumOf(0n, { f: 1n })).toEqual({ known: false });
  });

  it("distinguishes no votes from an unknown denominator", () => {
    const result = quorumOf(NETWORK_STAKE);

    expect(result).toEqual({
      known: true,
      totalActiveStake: NETWORK_STAKE,
      participationPercent: 0,
      isMet: false,
    });
  });

  it("counts abstain toward participation", () => {
    // Both readings of the disputed Art. IV.4 agree on this much.
    const result = quorumOf(NETWORK_STAKE, { ab: ONE_THIRD });

    expect(result.known && result.isMet).toBe(true);
  });

  it("is met exactly at one third, and not just below", () => {
    // One SOL rather than one lamport keeps the boundary readable.
    const at = quorumOf(NETWORK_STAKE, { f: ONE_THIRD });
    const below = quorumOf(NETWORK_STAKE, { f: ONE_THIRD - ONE_SOL });

    expect(at.known && at.isMet).toBe(true);
    expect(below.known && below.isMet).toBe(false);
  });

  it("does not let display rounding decide a borderline vote", () => {
    // Rounds to 33.33% but is genuinely short; the lamport comparison decides.
    const result = quorumOf(NETWORK_STAKE, { f: ONE_THIRD - ONE_SOL });

    expect(result.known).toBe(true);
    if (!result.known) throw new Error("unreachable");
    expect(result.participationPercent.toFixed(2)).toBe("33.33");
    expect(result.isMet).toBe(false);
  });

  it("sums all three buckets", () => {
    const result = quorumOf(NETWORK_STAKE, {
      f: NETWORK_STAKE / 2n,
      a: NETWORK_STAKE / 4n,
      ab: NETWORK_STAKE / 8n,
    });

    expect(result.known).toBe(true);
    if (!result.known) throw new Error("unreachable");
    expect(result.participationPercent).toBeCloseTo(87.5, 9);
    expect(result.isMet).toBe(true);
  });
});

describe("resolveQuorumDenominator", () => {
  const meta = { slot: 500n, total_active_stake: NETWORK_STAKE };

  it("uses the total when the served snapshot is the proposal's", () => {
    expect(resolveQuorumDenominator(meta, 500n)).toBe(NETWORK_STAKE);
  });

  it("refuses a total from any other snapshot", () => {
    // `/meta` serves the newest, which describes a different distribution.
    expect(resolveQuorumDenominator(meta, 499n)).toBeUndefined();
    expect(resolveQuorumDenominator(meta, 501n)).toBeUndefined();
  });

  it("handles a total that was never recorded, and absent meta", () => {
    expect(
      resolveQuorumDenominator({ slot: 500n, total_active_stake: null }, 500n),
    ).toBeUndefined();
    expect(resolveQuorumDenominator({ slot: 500n }, 500n)).toBeUndefined();
    expect(resolveQuorumDenominator(undefined, 500n)).toBeUndefined();
  });

  it("treats an unactivated proposal's zero slot as unknown", () => {
    // `snapshot_slot` is 0 until activate_voting sets it.
    expect(resolveQuorumDenominator({ slot: 0n }, 0n)).toBeUndefined();
  });
});
