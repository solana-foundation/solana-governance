export const LAMPORTS_PER_SOL = 1e9;

const isValidNumber = (value: number | null | undefined): value is number =>
  typeof value === "number" && !Number.isNaN(value);

export type LamportsDisplay = {
  value: string;
  rawValue?: string;
  showRaw: boolean;
};

export function formatSOL(lamports: bigint | number): string {
  if (typeof lamports === "number") {
    const sol = lamports / LAMPORTS_PER_SOL;
    return sol.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  }
  const whole = lamports / 1_000_000_000n;
  const fraction = (lamports % 1_000_000_000n).toString().padStart(9, "0").slice(0, 2);

  // Only use compact notation for amounts >= 10M
  if (whole >= 1_000_000_000n) {
    return `${whole / 1_000_000_000n}.${((whole % 1_000_000_000n) / 10_000_000n).toString().padStart(2, "0")}B`;
  } else if (whole >= 10_000_000n) {
    return `${whole / 1_000_000n}.${((whole % 1_000_000n) / 10_000n).toString().padStart(2, "0")}M`;
  }

  return `${whole}.${fraction}`;
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
  slot: number | null | undefined,
): string | number {
  return isValidNumber(slot) ? slot : "-";
}

export function formatOptionalCount(
  count: number | bigint | null | undefined,
): string | number {
  if (typeof count === "bigint") return count.toString();
  return isValidNumber(count) ? count : "-";
}

export function formatLamportsDisplay(
  lamports: bigint | number | null | undefined,
): LamportsDisplay {
  if (lamports === null || lamports === undefined) {
    return {
      value: "-",
      showRaw: false,
    };
  }

  const compact = formatSOL(lamports);
  const value = compact.includes("SOL") ? compact : `${compact} SOL`;

  const rawValue = formatSOL(lamports);

  const showRaw = typeof lamports === "bigint"
    ? lamports >= 10_000_000n * 1_000_000_000n
    : lamports >= 10_000_000 * LAMPORTS_PER_SOL;

  return {
    value,
    rawValue: showRaw ? `${rawValue} SOL` : undefined,
    showRaw,
  };
}
