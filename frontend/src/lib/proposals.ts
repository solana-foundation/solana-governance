import { SUPPORT_THRESHOLD_PERCENT } from "@/components/proposals/detail/support-phase-progress";
import type { ProposalStatus } from "@/types";
import type { RPCEndpoint } from "@/types";
import { PublicKey } from "@solana/web3.js";

export interface GetProposalStatusParams {
  creationEpoch: number;
  startEpoch: number;
  endEpoch: number;
  currentEpoch: number;
  clusterSupportLamports: number;
  totalStakedLamports: number;
  consensusResult: PublicKey | undefined;
  finalized: boolean;
  voting: boolean;
  endpointType?: RPCEndpoint;
}

interface EpochConstants {
  SUPPORT_EPOCHS: number;
  DISCUSSION_EPOCHS: number;
  SNAPSHOT_EPOCHS: number;
  VOTING_EPOCHS: number;
}

/**
 * TODO: Replace with on-chain GlobalConfig fetch. These values should come from
 * the GlobalConfig PDA instead of being hardcoded per-network.
 *
 * Returns epoch constants based on the network endpoint type.
 * Testnet values (default):
 * - SUPPORT_EPOCHS: 1
 * - DISCUSSION_EPOCHS: 2
 * - SNAPSHOT_EPOCHS: 1
 * - VOTING_EPOCHS: 4
 *
 * Mainnet values:
 * - SUPPORT_EPOCHS: 1
 * - DISCUSSION_EPOCHS: 6
 * - SNAPSHOT_EPOCHS: 1
 * - VOTING_EPOCHS: 4
 */
export function getEpochConstants(
  endpointType: RPCEndpoint = "testnet",
): EpochConstants {
  if (endpointType === "mainnet") {
    return {
      SUPPORT_EPOCHS: 1,
      DISCUSSION_EPOCHS: 0,
      SNAPSHOT_EPOCHS: 1,
      VOTING_EPOCHS: 3,
    };
  }

  // Default to testnet values
  return {
    SUPPORT_EPOCHS: 1,
    DISCUSSION_EPOCHS: 2,
    SNAPSHOT_EPOCHS: 1,
    VOTING_EPOCHS: 4,
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
  consensusResult,
  finalized,
  voting,
  endpointType = "testnet",
}: GetProposalStatusParams): ProposalStatus => {
  // If finalized, always return finalized
  if (finalized) {
    return "finalized";
  }

  // Voting ends when currentEpoch >= endEpoch (inclusive)
  // If voting has ended but not finalized, check if proposal failed first
  // If voting === false, proposal failed (didn't get enough support) - show failed even if past endEpoch
  if (currentEpoch >= endEpoch && endEpoch !== 0) {
    if (!voting) {
      return "failed";
    }
    // If voting === true, return "finalized" since it's eligible for finalization
    return "finalized";
  }

  // Get epoch constants based on endpoint type
  const epochs = getEpochConstants(endpointType);

  // Support phase always uses creationEpoch
  const supportStartEpoch = creationEpoch; // epoch 800 for creationEpoch 800
  // Threshold check happens at creationEpoch + SUPPORT_EPOCHS + 1
  // (support phase is epochs [creationEpoch, creationEpoch + SUPPORT_EPOCHS], threshold check at creationEpoch + SUPPORT_EPOCHS + 1)
  const supportEndEpoch = creationEpoch + epochs.SUPPORT_EPOCHS + 1; // epoch 802 for creationEpoch 800 (threshold check)

  // When voting === true, startEpoch is when voting phase will start (in the future)
  // Before startEpoch, the proposal is in discussion phase
  // When voting === false, calculate phases based on creationEpoch
  const phaseBaseEpoch = creationEpoch + epochs.SUPPORT_EPOCHS + 1; // epoch 802 for creationEpoch 800
  const discussionStartEpoch = phaseBaseEpoch; // epoch 802 for creationEpoch 800
  const discussionEndEpoch = phaseBaseEpoch + epochs.DISCUSSION_EPOCHS; // epoch 804 for creationEpoch 800
  const snapshotEpoch =
    phaseBaseEpoch + epochs.DISCUSSION_EPOCHS + epochs.SNAPSHOT_EPOCHS; // epoch 805 for creationEpoch 800
  // When voting === false, voting starts right after snapshot phase
  // When voting === true, use startEpoch directly as the voting start epoch
  const votingStartEpoch = voting ? startEpoch : snapshotEpoch + 1; // epoch 806 for creationEpoch 800 (or startEpoch if voting = true)

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
      totalStakedLamports * (SUPPORT_THRESHOLD_PERCENT / 100);
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
