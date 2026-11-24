export const LAMPORTS_PER_SOL = 1e9;

const isValidNumber = (value: number | null | undefined): value is number =>
  typeof value === "number" && !Number.isNaN(value);

export type LamportsDisplay = {
  value: string;
  rawValue?: string;
  showRaw: boolean;
};

const LAMPORTS_PER_SOL_BIGINT = 1_000_000_000n;

/** `value / unit` rounded half up to two decimals, e.g. "1,234,567.90". */
function formatHundredths(value: bigint, unit: bigint, group: boolean): string {
  const hundredths = (value * 100n + unit / 2n) / unit;
  const whole = hundredths / 100n;
  const fraction = (hundredths % 100n).toString().padStart(2, "0");
  return `${group ? whole.toLocaleString("en-US") : whole}.${fraction}`;
}

const formatGroupedSOL = (lamports: bigint) =>
  formatHundredths(lamports, LAMPORTS_PER_SOL_BIGINT, true);

/**
 * Formats lamports as SOL with two decimals. Amounts of 10M SOL or more use
 * compact notation ("12.35M", "1.20B"); smaller ones are grouped by thousands.
 */
export function formatSOL(lamports: bigint): string {
  if (lamports >= 1_000_000_000n * LAMPORTS_PER_SOL_BIGINT) {
    return `${formatHundredths(lamports, 1_000_000_000n * LAMPORTS_PER_SOL_BIGINT, false)}B`;
  }
  if (lamports >= 10_000_000n * LAMPORTS_PER_SOL_BIGINT) {
    return `${formatHundredths(lamports, 1_000_000n * LAMPORTS_PER_SOL_BIGINT, false)}M`;
  }
  return formatGroupedSOL(lamports);
}

export function formatPercentage(percentage: number, decimals?: number) {
  const d = decimals ?? 0;
  return `(${percentage.toFixed(d)}%)`;
}

export function formatAddress(address: string, length: number = 4): string {
  if (!address) return "";
  if (address.length <= length * 2) return address;
  return `${address.slice(0, length)}...${address.slice(-length)}`;
}

export function formatCommission(commission: number | undefined): string {
  if (commission === undefined) return "N/A";
  return `${commission}%`;
}

export function formatOptionalSlot(
  slot: bigint | null | undefined,
): string {
  return slot === null || slot === undefined ? "-" : slot.toString();
}

export function formatOptionalCount(
  count: number | bigint | null | undefined,
): string | number {
  if (typeof count === "bigint") return count.toString();
  return isValidNumber(count) ? count : "-";
}

export function formatLamportsDisplay(
  lamports: bigint | null | undefined,
): LamportsDisplay {
  if (lamports === null || lamports === undefined) {
    return {
      value: "-",
      showRaw: false,
    };
  }

  const showRaw = lamports >= 10_000_000n * LAMPORTS_PER_SOL_BIGINT;

  return {
    value: `${formatSOL(lamports)} SOL`,
    rawValue: showRaw ? `${formatGroupedSOL(lamports)} SOL` : undefined,
    showRaw,
  };
}
