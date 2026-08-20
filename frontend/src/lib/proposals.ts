import type { GovernanceConfigDto } from "@/lib/getGovernanceConfig";
import type { ProposalStatus } from "@/types";
import type { Address } from "@solana/kit";

export interface EpochConstants {
  SUPPORT_EPOCHS: bigint;
  DISCUSSION_EPOCHS: bigint;
  SNAPSHOT_EPOCHS: bigint;
  VOTING_EPOCHS: bigint;
}

/** Derives lifecycle epoch lengths from the on-chain GlobalConfig (see GovernanceConfigDto). */
export function epochConstantsFromGovernanceConfig(
  dto: GovernanceConfigDto,
): EpochConstants {
  return {
    SUPPORT_EPOCHS: dto.maxSupportEpochs,
    DISCUSSION_EPOCHS: dto.discussionEpochs,
    SNAPSHOT_EPOCHS: dto.snapshotEpochExtension,
    VOTING_EPOCHS: dto.votingEpochs,
  };
}

/**
 * Fallback support threshold while the on-chain config is loading (or failed
 * to load). Matches the current mainnet GlobalConfig value; the real value is
 * always preferred once fetched.
 */
export const DEFAULT_SUPPORT_THRESHOLD_PERCENT = 15;

/** Support threshold as a percentage (e.g. 15 for 15%), from the on-chain GlobalConfig. */
export function supportThresholdPercentFromConfig(
  dto: GovernanceConfigDto | undefined,
): string {
  return dto
    ? formatBasisPoints(dto.clusterSupportPctMinBps)
    : DEFAULT_SUPPORT_THRESHOLD_PERCENT.toString();
}

/** Shared copy for the support-phase requirement. */
export function supportPhaseRequirementCopy(thresholdPercent: string): string {
  return `The support phase requires ${thresholdPercent}% of total validator stake expressing support for the proposal before it can move on to discussion and voting phase`;
}

export interface GetProposalStatusParams {
  creationEpoch: bigint;
  startEpoch: bigint;
  endEpoch: bigint;
  currentEpoch: bigint;
  clusterSupportLamports: bigint;
  totalStakedLamports: bigint;
  /** GlobalConfig.clusterSupportPctMinBps — support threshold in basis points. */
  clusterSupportPctMinBps: bigint;
  consensusResult: Address | undefined;
  finalized: boolean;
  voting: boolean;
  epochConstants: EpochConstants;
}

export interface ProposalPhaseEpochs {
  supportStartEpoch: bigint;
  supportEndEpoch: bigint;
  phaseBaseEpoch: bigint;
  discussionStartEpoch: bigint;
  discussionEndEpoch: bigint;
  snapshotEpoch: bigint;
}

/**
 * On-chain phase anchors from the proposal account. Once support succeeds
 * (`voting === true`) the program records the definitive voting start/end
 * epochs, which supersede any creation-based projection: a proposal that
 * reaches the threshold before its full support window enters discussion
 * (and voting) earlier than the worst-case estimate.
 */
export interface ProposalPhaseAnchors {
  voting: boolean;
  startEpoch: bigint;
}

export function getProposalPhaseEpochs(
  creationEpoch: bigint,
  epochs: EpochConstants,
  onChain?: ProposalPhaseAnchors,
): ProposalPhaseEpochs {
  // Support phase always uses creationEpoch
  const supportStartEpoch = creationEpoch;
  // Threshold check happens at creationEpoch + SUPPORT_EPOCHS + 1
  // (support phase is epochs [creationEpoch, creationEpoch + SUPPORT_EPOCHS], threshold check at creationEpoch + SUPPORT_EPOCHS + 1)
  const supportEndEpoch = creationEpoch + epochs.SUPPORT_EPOCHS + 1n;
  // When voting === false, calculate phases based on creationEpoch
  const phaseBaseEpoch = supportEndEpoch;
  const discussionStartEpoch = phaseBaseEpoch;
  let discussionEndEpoch = phaseBaseEpoch + epochs.DISCUSSION_EPOCHS;
  let snapshotEpoch =
    phaseBaseEpoch + epochs.DISCUSSION_EPOCHS + epochs.SNAPSHOT_EPOCHS;

  // Support already succeeded: the program has set the real voting start
  // (discussion + snapshot extension counted from when the threshold was
  // met, not from the end of the full support window). Use it.
  if (onChain?.voting && onChain.startEpoch > 0n) {
    discussionEndEpoch = onChain.startEpoch;
    snapshotEpoch = onChain.startEpoch;
  }

  return {
    supportStartEpoch,
    supportEndEpoch,
    phaseBaseEpoch,
    discussionStartEpoch,
    discussionEndEpoch,
    snapshotEpoch,
  };
}

/**
 * Determines proposal status based on epoch-based rules:
 *
 * When voting === true, it means the proposal got enough support and discussion phase started.
 * In this case, startEpoch represents when voting phase will start (in the future).
 * - If currentEpoch < startEpoch: proposal is in discussion phase
 * - If currentEpoch >= startEpoch: proposal is in voting phase (if consensusResult exists)
 *
 * Example with creationEpoch = 800:
 * - Epoch 800: "supporting" (before support phase)
 * - Epoch 801: "supporting" (support phase active - time between epoch 800 ending and 802 starting)
 * - Epoch 802: Check 15% threshold at start of epoch
 *   - If NOT met: "failed"
 *   - If met: "discussion" (discussion phase starts, voting = true, startEpoch = 907)
 * - Epochs 802-906: "discussion" (discussion phase - before startEpoch)
 * - Epochs 907+: "voting" (if consensusResult exists) or "discussion" (if snapshot not ready)
 *
 * If proposal gets enough support before epoch ends (voting = true early):
 * - Use startEpoch directly to determine when voting phase starts
 * - Before startEpoch: discussion phase
 * - At or after startEpoch: voting phase (if consensusResult exists)
 */
export const getProposalStatus = ({
  creationEpoch,
  startEpoch,
  endEpoch,
  currentEpoch,
  clusterSupportLamports,
  totalStakedLamports,
  clusterSupportPctMinBps,
  consensusResult,
  finalized,
  voting,
  epochConstants: epochs,
}: GetProposalStatusParams): ProposalStatus => {
  // If finalized, always return finalized
  if (finalized) {
    return "finalized";
  }

  // Voting ends when currentEpoch >= endEpoch (inclusive)
  // If voting has ended but not finalized, check if proposal failed first
  // If voting === false, proposal failed (didn't get enough support) - show failed even if past endEpoch
  if (currentEpoch >= endEpoch && endEpoch !== 0n) {
    if (!voting) {
      return "failed";
    }
    // If voting === true, return "finalized" since it's eligible for finalization
    return "finalized";
  }

  const {
    supportStartEpoch,
    supportEndEpoch,
    discussionStartEpoch,
    discussionEndEpoch,
    snapshotEpoch,
  } = getProposalPhaseEpochs(creationEpoch, epochs);
  // When voting === false, voting starts right after snapshot phase
  // When voting === true, use startEpoch directly as the voting start epoch
  const votingStartEpoch = voting ? startEpoch : snapshotEpoch + 1n; // epoch 806 for creationEpoch 800 (or startEpoch if voting = true)

  // Before support phase starts
  if (currentEpoch < supportStartEpoch) {
    return "supporting";
  }

  // When voting === true, use startEpoch to determine phase
  // startEpoch is when voting phase will start (in the future)
  // If currentEpoch < startEpoch, proposal is in discussion phase
  // If currentEpoch >= startEpoch and currentEpoch < endEpoch, proposal is in voting phase (if consensusResult exists)
  if (voting) {
    if (currentEpoch < votingStartEpoch) {
      // Before voting starts, proposal is in discussion phase
      return "discussion";
    }
    // At or past voting start epoch, but before end epoch
    // Note: endEpoch check is already done above, so we know currentEpoch < endEpoch here
    if (consensusResult) {
      return "voting";
    }
    // Snapshot not available yet, still in discussion
    return "discussion";
  }

  // When voting === false, use normal phase calculation
  // During support phase (epoch 800 for creationEpoch 800)
  if (currentEpoch === supportStartEpoch) {
    return "supporting";
  }

  // Still in support phase (between supportStartEpoch and supportEndEpoch)
  if (currentEpoch < supportEndEpoch) {
    return "supporting";
  }

  // At support end epoch (epoch 802) - check threshold directly
  if (currentEpoch === supportEndEpoch) {
    const requiredThresholdLamports =
      (totalStakedLamports * clusterSupportPctMinBps) / 10_000n;
    const isThresholdMet = clusterSupportLamports >= requiredThresholdLamports;

    if (!isThresholdMet) {
      return "failed";
    }
    // Threshold was met, continue to discussion phase
    return "discussion";
  }

  // When voting === false, use the normal phase calculation
  // During discussion phase (epochs 802-804 for creationEpoch 800)
  // A proposal is only truly in discussion phase IF threshold was met
  if (
    currentEpoch >= discussionStartEpoch &&
    currentEpoch <= discussionEndEpoch
  ) {
    // Threshold wasn't met - proposal failed
    return "failed";
  }

  // If we're past the discussion phase and voting === false, threshold wasn't met
  if (currentEpoch > discussionEndEpoch) {
    return "failed";
  }

  // Fallback (shouldn't reach here, but return supporting as default)
  return "supporting";
};

/** Formats a basis-point value without coercing its on-chain bigint to Number. */
function formatBasisPoints(basisPoints: bigint): string {
  const whole = basisPoints / 100n;
  const fraction = (basisPoints % 100n).toString().padStart(2, "0").replace(/0+$/, "");
  return fraction ? `${whole}.${fraction}` : whole.toString();
}
