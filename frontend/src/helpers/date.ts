import {
  createSolanaRpc,
  type GetRecentPerformanceSamplesApi,
} from "@solana/kit";
import type { EpochInfoData } from "@/hooks/useEpochInfo";
import { bigintToSafeNumber } from "./bigint";

type PerfSample = ReturnType<
  GetRecentPerformanceSamplesApi["getRecentPerformanceSamples"]
>[number];

const DEFAULT_SLOT_TIME_MS = 350;
const PERFORMANCE_SAMPLE_LIMIT = 60;
const MIN_PLAUSIBLE_SLOT_TIME_MS = 100;
const MAX_PLAUSIBLE_SLOT_TIME_MS = 1_000;

/**
 * Calculates a slot-duration estimate weighted by the number of slots in each RPC sample.
 * Falls back to the configured 350ms slot time when an RPC omits or returns unusable samples.
 */
export function estimateSlotTimeMs(
  samples: ReadonlyArray<Pick<PerfSample, "numSlots" | "samplePeriodSecs">>,
): number {
  const totals = samples.reduce(
    (acc, sample) => {
      if (sample.numSlots > 0n && sample.samplePeriodSecs > 0) {
        acc.slots += sample.numSlots;
        acc.seconds += sample.samplePeriodSecs;
      }
      return acc;
    },
    { slots: 0n, seconds: 0 },
  );

  if (totals.slots === 0n) return DEFAULT_SLOT_TIME_MS;

  const slotTimeMs =
    (totals.seconds * 1_000) /
    bigintToSafeNumber(totals.slots, "performance sample slot count");
  if (
    !Number.isFinite(slotTimeMs) ||
    slotTimeMs < MIN_PLAUSIBLE_SLOT_TIME_MS ||
    slotTimeMs > MAX_PLAUSIBLE_SLOT_TIME_MS
  ) {
    return DEFAULT_SLOT_TIME_MS;
  }

  return slotTimeMs;
}

export const getDaysLeft = (futureDate: Date) => {
  const now = new Date();
  const diffMs = futureDate.getTime() - now.getTime(); // convert both to milliseconds
  const diffDays = Math.ceil(diffMs / (1000 * 60 * 60 * 24)); // ms to days

  return diffDays;
};

/**
 * Converts a Solana epoch number to a Date by calculating when that epoch will start
 * @param epoch - The target epoch number
 * @param epochInfo - The current epoch info from Solana
 * @param epochSchedule - The epoch schedule from Solana
 * @param endpoint - The Solana RPC endpoint URL (used for getBlockTime calls)
 * @returns Promise<Date> - The estimated date when the epoch will start
 */
export async function epochToDate(
  epoch: bigint,
  epochInfo: EpochInfoData["epochInfo"],
  epochSchedule: EpochInfoData["epochSchedule"],
  endpoint: string
): Promise<Date> {
  const rpc = createSolanaRpc(endpoint);

  // Calculate the target epoch (creationEpoch + 3)
  const targetEpoch = epoch;

  // If target epoch is in the past or current, use current time
  if (targetEpoch <= epochInfo.epoch) {
    // Get the first slot of the target epoch
    const targetSlot = getFirstSlotInEpoch(targetEpoch, epochSchedule);
    try {
      // Try to get block time for that slot
      const blockTime = await rpc.getBlockTime(targetSlot).send();
      if (blockTime) {
        return new Date(bigintToSafeNumber(blockTime, "block time") * 1000); // Convert to milliseconds
      }
    } catch {
      console.warn("Failed to get block time for epoch", targetEpoch);
      // If we can't get block time, estimate based on current time
      return new Date();
    }
  }

  // Get the first slot of the target epoch
  const targetSlot = getFirstSlotInEpoch(targetEpoch, epochSchedule);

  const slotsUntilTarget = targetSlot - epochInfo.absoluteSlot;

  // Anchor the projection to chain time and derive the current slot rate from the last hour of
  // RPC performance samples. Slot production can run materially faster than the 400ms target,
  // shortening epoch phases by hours; a fixed slot duration makes proposal countdowns stale.
  const [blockTimeResult, performanceSamplesResult] = await Promise.allSettled([
    rpc.getBlockTime(epochInfo.absoluteSlot).send(),
    rpc.getRecentPerformanceSamples(PERFORMANCE_SAMPLE_LIMIT).send(),
  ]);

  let currentBlockTime = Date.now();
  if (blockTimeResult.status === "fulfilled" && blockTimeResult.value !== null) {
    currentBlockTime =
      bigintToSafeNumber(blockTimeResult.value, "block time") * 1_000;
  } else {
    console.warn(
      "Failed to get block time for current epoch",
      epochInfo.absoluteSlot,
    );
  }

  const slotTimeMs =
    performanceSamplesResult.status === "fulfilled"
      ? estimateSlotTimeMs(performanceSamplesResult.value)
      : DEFAULT_SLOT_TIME_MS;

  if (performanceSamplesResult.status === "rejected") {
    console.warn(
      "Failed to get recent performance samples; using the default slot time",
    );
  }

  const estimatedTime =
    currentBlockTime +
    bigintToSafeNumber(slotsUntilTarget, "slot delta") * slotTimeMs;

  return new Date(estimatedTime);
}

function getFirstSlotInEpoch(
  epoch: bigint,
  schedule: EpochInfoData["epochSchedule"],
): bigint {
  if (!schedule.warmup || epoch >= schedule.firstNormalEpoch) {
    return schedule.firstNormalSlot +
      (epoch - schedule.firstNormalEpoch) * schedule.slotsPerEpoch;
  }

  let firstSlot = 0n;
  let slotsInEpoch = 32n;
  for (let currentEpoch = 0n; currentEpoch < epoch; currentEpoch++) {
    firstSlot += slotsInEpoch;
    slotsInEpoch = slotsInEpoch * 2n;
  }
  return firstSlot;
}

export const getHoursLeft = (futureDate: Date) => {
  const now = new Date();
  const diffMs = futureDate.getTime() - now.getTime(); // milliseconds difference
  const diffHours = Math.ceil(diffMs / (1000 * 60 * 60)); // convert to hours

  return diffHours;
};

export function calculateVotingEndsIn(
  endTime: string | null,
  nowMs: number = Date.now(),
): string | null {
  if (!endTime) return null;

  const now = new Date(nowMs);
  const end = new Date(endTime);

  // Check if the date is valid
  if (isNaN(end.getTime())) return null;

  // Calculate difference in milliseconds
  const diff = end.getTime() - now.getTime();

  // If voting has ended
  if (diff <= 0) return "Ended";

  const days = Math.floor(diff / (1000 * 60 * 60 * 24));
  const hours = Math.floor((diff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
  const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));
  // const seconds = Math.floor((diff % (1000 * 60)) / 1000);

  // Format the output based on the largest unit
  if (days > 30) {
    const months = Math.floor(days / 30);
    return `${months}mo ${days % 30}d`;
  }

  if (days > 0) {
    return `${days}d ${hours}h ${minutes}m`;
  }

  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }

  // Always show minutes, even if 0
  return `${minutes}m`;
}

export function formatDate(dateStr: string | null): string | null {
  if (!dateStr) return null;

  const date = new Date(dateStr);

  if (isNaN(date.getTime())) return null;

  return new Intl.DateTimeFormat("en-US", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  })
    .format(date)
    .replace(",", "");
}

/**
 * Calculate how long ago a timestamp was
 * @param timestamp - Unix timestamp in milliseconds
 * @param nowMs - Optional "now" in ms (use when SSR-safe; omit to use Date.now())
 * @returns Formatted string like "3 days ago", "today", etc.
 */
export function calculateTimeAgo(timestamp: number, nowMs?: number): string {
  // If timestamp is in seconds (10 digits), convert to milliseconds
  if (timestamp < 1e12) {
    timestamp = timestamp * 1000;
  }
  const now = nowMs ?? Date.now();
  const diff = now - timestamp;
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));
  const hours = Math.floor((diff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
  const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));

  if (days === 0) {
    if (hours === 0) {
      if (minutes === 0) return "just now";
      if (minutes === 1) return "1 minute ago";
      return `${minutes} minutes ago`;
    }
    if (hours === 1) return "1 hour ago";
    return `${hours} hours ago`;
  }
  if (days === 1) return "1 day ago";
  if (days < 30) return `${days} days ago`;

  const months = Math.floor(days / 30);
  if (months === 1) return "1 month ago";
  if (months < 12) return `${months} months ago`;

  const years = Math.floor(days / 365);
  if (years === 1) return "1 year ago";
  return `${years} years ago`;
}
