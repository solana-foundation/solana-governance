import type { GetBlockTimeApi, GetEpochInfoApi, Rpc } from "@solana/kit";

const TARGET_SLOT_TIME_MS = 400n;
const MIN_EXPIRY_BUFFER_SECONDS = 3_600n;
const BLOCK_TIME_LOOKBACK_SLOTS = 8;

/**
 * Estimates the Unix timestamp at which a proposal's voting ends, for use as
 * the close timestamp of a meta merkle proof account. The start of `endEpoch`
 * is projected from a recent block time at the 400ms target slot time, then
 * padded by 20% (at least one hour) so the proof outlives the vote even if
 * slots run slow.
 *
 * @throws If none of the most recent slots has a block time.
 */
export async function computeProofCloseTimestamp(
  rpc: Rpc<GetBlockTimeApi & GetEpochInfoApi>,
  endEpoch: bigint,
): Promise<bigint> {
  const epochInfo = await rpc.getEpochInfo().send();
  const epochStartSlot = epochInfo.absoluteSlot - epochInfo.slotIndex;
  const targetSlot =
    epochStartSlot + (endEpoch - epochInfo.epoch) * epochInfo.slotsInEpoch;

  let blockTime: bigint | null = null;
  let referenceSlot = epochInfo.absoluteSlot;
  for (
    let attempt = 0;
    attempt < BLOCK_TIME_LOOKBACK_SLOTS && referenceSlot >= 0n;
    attempt += 1
  ) {
    try {
      blockTime = await rpc.getBlockTime(referenceSlot).send();
      if (blockTime !== null) break;
    } catch {
      // Skipped slots and pruned RPC history can both lack a block time.
    }
    referenceSlot -= 1n;
  }
  if (blockTime === null) {
    throw new Error("Failed to fetch a recent block time for the proof expiry");
  }

  const projectedSeconds =
    ((targetSlot - referenceSlot) * TARGET_SLOT_TIME_MS) / 1_000n;
  if (projectedSeconds <= 0n) return blockTime + projectedSeconds;

  const proportionalBuffer = (projectedSeconds * 20n) / 100n;
  const bufferSeconds =
    proportionalBuffer > MIN_EXPIRY_BUFFER_SECONDS
      ? proportionalBuffer
      : MIN_EXPIRY_BUFFER_SECONDS;
  return blockTime + projectedSeconds + bufferSeconds;
}

/**
 * Makes sure a shared meta merkle proof account exists, calling `initialize`
 * to create it when `isInitialized` reports it missing.
 *
 * Every delegator of a validator shares the account, so another wallet may
 * create it first and make `initialize` fail. That failure is ignored when the
 * account exists afterwards.
 *
 * @throws The error from `initialize` if the account is still missing.
 */
export async function ensureMetaMerkleProofInitialized(input: {
  initialize: () => Promise<unknown>;
  isInitialized: () => Promise<boolean>;
}): Promise<void> {
  if (await input.isInitialized()) return;
  try {
    await input.initialize();
  } catch (error) {
    if (!(await input.isInitialized())) throw error;
  }
}
