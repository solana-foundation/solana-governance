/**
 * Converts only a bounded bigint for browser APIs that accept numbers (notably
 * Date). All on-chain values remain bigint in DTOs and are formatted as strings
 * elsewhere, so precision cannot be silently lost.
 */
export function bigintToSafeNumber(value: bigint, context: string): number {
  const limit = BigInt(Number.MAX_SAFE_INTEGER);
  if (value < -limit || value > limit) {
    throw new RangeError(`${context} exceeds JavaScript's safe integer range`);
  }
  return Number(value);
}

/** Formats a bigint ratio as a decimal percentage without a Number coercion. */
export function formatBigintPercentage(
  numerator: bigint,
  denominator: bigint,
  decimalPlaces = 2,
): string {
  if (denominator === 0n) return "0.00";
  const scale = 10n ** BigInt(decimalPlaces);
  const scaled = (numerator * 100n * scale) / denominator;
  const whole = scaled / scale;
  const fraction = (scaled % scale).toString().padStart(decimalPlaces, "0");
  return decimalPlaces > 0 ? `${whole}.${fraction}` : whole.toString();
}
