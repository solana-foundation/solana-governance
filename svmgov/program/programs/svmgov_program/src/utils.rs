/// Calculates the validator's stake weight in basis points (1 bp = 0.01%) relative to the cluster stake.
///
/// This macro uses integer arithmetic to compute the stake weight by multiplying the validator's stake
/// by 10,000 (to convert to basis points) and dividing by the total cluster stake.
/// Uses u128 internally to prevent overflow during multiplication.
///
/// # Arguments
///
/// * `validator_stake` - The stake of the validator, voter (u64).
/// * `cluster_stake` - The total stake in the cluster (u64). Must be non-zero.
///
/// # Example
///
/// ```rust
/// let validator_stake = 40_001u64;
/// let cluster_stake = 380_000_000u64;
/// let weight_bp = stake_weight_bp!(validator_stake, cluster_stake)?;
/// // Returns approximately 1 bp
/// ```
#[macro_export]
macro_rules! stake_weight_bp {
    ($validator_stake:expr, $cluster_stake:expr) => {{
        ($validator_stake as u128)
            .checked_mul($crate::constants::BASIS_POINTS_MAX as u128)
            .and_then(|product| product.checked_div($cluster_stake as u128))
            .ok_or($crate::error::GovernanceError::ArithmeticOverflow)
            .map(|result| result as u64)
    }};
}

/// Calculates stake-weighted vote amounts from basis points
///
/// This macro calculates the vote lamports by multiplying stake amount
/// by basis points and dividing by 10,000 (the total basis points for 100%).
/// Uses u128 internally to prevent overflow during multiplication.
///
/// # Arguments
///
/// * `stake` - The stake amount in lamports (u64)
/// * `basis_points` - The vote distribution in basis points (u64, 0-10,000)
///
/// # Example
///
/// ```rust
/// let stake = 1_000_000u64; // 1 SOL in lamports
/// let basis_points = 2_500u64; // 25% of stake
/// let vote_lamports = calculate_vote_lamports!(stake, basis_points)?;
/// // Returns 250,000 lamports (25% of 1 SOL)
/// ```
#[macro_export]
macro_rules! calculate_vote_lamports {
    ($stake:expr, $basis_points:expr) => {{
        ($stake as u128)
            .checked_mul($basis_points as u128)
            .and_then(|product| product.checked_div($crate::constants::BASIS_POINTS_MAX as u128))
            .ok_or($crate::error::GovernanceError::ArithmeticOverflow)
            .map(|result| result as u64)
    }};
}

/// Validates if the input is a well-formed GitHub repository or issue link.
pub fn is_valid_github_link(link: &str) -> bool {
    const PREFIX: &str = "https://github.com/";
    const MAX_SEGMENTS: usize = 10;
    const MIN_SEGMENTS: usize = 2;

    if !link.starts_with(PREFIX) {
        return false;
    }

    let mut path = &link[PREFIX.len()..];
    if path.ends_with('/') {
        if path.len() == 1 {
            // If only '/', path would be empty after trim
            return false;
        }
        path = &path[..path.len() - 1];
    }
    if path.is_empty() || path.starts_with('/') {
        return false;
    }

    let mut segment_count = 0;
    let mut in_segment = false;
    let mut has_invalid_char = false;

    for c in path.chars() {
        match c {
            '/' => {
                if !in_segment {
                    // Consecutive '/' -> empty segment
                    return false;
                }
                in_segment = false;
                segment_count += 1;
                if segment_count > MAX_SEGMENTS {
                    return false;
                }
            }
            ' ' | '?' | '#' => {
                has_invalid_char = true;
                break; // Early exit on forbidden chars
            }
            _ => {
                if !in_segment {
                    in_segment = true;
                }
                if !c.is_alphanumeric() && !matches!(c, '-' | '_' | '.') {
                    has_invalid_char = true;
                    break;
                }
            }
        }
    }

    if has_invalid_char {
        return false;
    }

    // Account for the last segment if it was being processed
    if in_segment {
        segment_count += 1;
    }

    // Check trailing '/' was handled (no empty last segment)
    (MIN_SEGMENTS..=MAX_SEGMENTS).contains(&segment_count)
}

/// Calculates the starting and ending slot for a given epoch.
///
/// Solana epochs consist of 432,000 slots each. This function calculates:
/// - First slot: epoch * 432000
/// - Last slot: (epoch + 1) * 432000 - 1
///
/// # Arguments
///
/// * `epoch` - The epoch number (u64)
///
/// # Returns
///
/// A tuple `(start_slot, end_slot)` representing the slot range for the epoch.
///
/// # Example
///
/// ```rust
/// let (start, end) = get_epoch_slot_range(0);
/// // Returns (0, 431999)
///
/// let (start, end) = get_epoch_slot_range(1);
/// // Returns (432000, 863999)
/// ```
pub fn get_epoch_slot_range(epoch: u64) -> (u64, u64) {
    const SLOTS_PER_EPOCH: u64 = 432_000;

    let start_slot = epoch * SLOTS_PER_EPOCH;
    let end_slot = (epoch + 1) * SLOTS_PER_EPOCH - 1;

    (start_slot, end_slot)
}

/// Computes the schedule anchor epoch for a proposal: the epoch whose start slot
/// drives `snapshot_slot`, and from which `start_epoch` (anchor + 1) and
/// `end_epoch` are derived.
///
/// `support_proposal` calls this with the support epoch (`creation_epoch +
/// max_support_epochs`, which equals `clock.epoch` when support activates), so the
/// initial voting schedule includes the full `discussion_epochs` window.
///
/// `flush_merkle_root` does NOT use this helper. It is an admin-only recovery path
/// that intentionally re-anchors the snapshot/voting window forward off the *current*
/// epoch (`current_epoch + snapshot_epoch_extension`, omitting the discussion window)
/// so a proposal whose NCN snapshot failed can be rescheduled. Because only the admin
/// multisig can call flush, the author-driven postponement that an immutable anchor
/// previously guarded against cannot occur there.
///
/// Returns `ArithmeticOverflow` if the summed epoch exceeds `u64`.
pub fn proposal_target_epoch(
    support_epoch: u64,
    discussion_epochs: u64,
    snapshot_epoch_extension: u64,
) -> core::result::Result<u64, crate::error::GovernanceError> {
    support_epoch
        .checked_add(discussion_epochs)
        .and_then(|v| v.checked_add(snapshot_epoch_extension))
        .ok_or(crate::error::GovernanceError::ArithmeticOverflow)
}

/// Computes the snapshot slot for a proposal's voting lineage from the snapshot
/// `target_epoch` and the configured `snapshot_slot_offset`, enforcing that the
/// resulting slot is strictly in the future relative to `current_slot`.
///
/// The `snapshot_slot > current_slot` invariant mirrors the guard inside
/// `ncn_snapshot::init_ballot_box`. Both `support_proposal` and
/// `flush_merkle_root` skip that CPI whenever the supplied `ballot_box` account
/// already exists, so without re-checking here a caller could bind a proposal to
/// an already-finalized `ConsensusResult` for a past slot (proposal backdating).
///
/// Returns the validated `snapshot_slot`, or an error if the offset underflows
/// below zero or the resulting slot is not in the future.
pub fn compute_future_snapshot_slot(
    target_epoch: u64,
    snapshot_slot_offset: i64,
    current_slot: u64,
) -> core::result::Result<u64, crate::error::GovernanceError> {
    let (start_slot, _) = get_epoch_slot_range(target_epoch);
    let offset_result = (start_slot as i64)
        .checked_add(snapshot_slot_offset)
        .ok_or(crate::error::GovernanceError::ArithmeticOverflow)?;
    if offset_result < 0 {
        return Err(crate::error::GovernanceError::ArithmeticOverflow);
    }
    let snapshot_slot = offset_result as u64;
    if snapshot_slot <= current_slot {
        return Err(crate::error::GovernanceError::SnapshotSlotNotInFuture);
    }
    Ok(snapshot_slot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::GovernanceError;

    #[test]
    fn epoch_slot_range_is_correct() {
        assert_eq!(get_epoch_slot_range(0), (0, 431_999));
        assert_eq!(get_epoch_slot_range(1), (432_000, 863_999));
    }

    #[test]
    fn future_snapshot_slot_accepts_future_slot() {
        // Epoch 2 starts at slot 864_000; current slot is well before that.
        assert_eq!(
            compute_future_snapshot_slot(2, 0, 500_000).unwrap(),
            864_000
        );
    }

    #[test]
    fn future_snapshot_slot_applies_positive_offset() {
        assert_eq!(
            compute_future_snapshot_slot(2, 100, 500_000).unwrap(),
            864_100
        );
    }

    #[test]
    fn future_snapshot_slot_rejects_past_slot() {
        // Recomputed snapshot slot (864_000) is behind the current slot.
        assert!(matches!(
            compute_future_snapshot_slot(2, 0, 900_000),
            Err(GovernanceError::SnapshotSlotNotInFuture)
        ));
    }

    #[test]
    fn future_snapshot_slot_rejects_current_slot() {
        // Must be strictly greater than the current slot, not equal to it.
        assert!(matches!(
            compute_future_snapshot_slot(2, 0, 864_000),
            Err(GovernanceError::SnapshotSlotNotInFuture)
        ));
    }

    #[test]
    fn future_snapshot_slot_rejects_negative_offset_underflow() {
        // A negative offset that drives the slot below zero is an overflow error.
        assert!(matches!(
            compute_future_snapshot_slot(0, -1, 0),
            Err(GovernanceError::ArithmeticOverflow)
        ));
    }

    #[test]
    fn future_snapshot_slot_rejects_backdating_via_negative_offset() {
        // Mirrors the reported exploit: a negative `snapshot_slot_offset` pulls the
        // recomputed snapshot slot into an already-past slot. It must be rejected
        // rather than silently accepted (which previously let a proposal bind onto
        // a stale, already-finalized ConsensusResult by skipping init_ballot_box).
        let target_epoch = 5;
        let (start_slot, _) = get_epoch_slot_range(target_epoch); // 2_160_000
        let current_slot = start_slot + 10; // we are already past the snapshot start
        let offset = -20i64; // recomputed slot = start_slot - 20 < current_slot
        assert!(matches!(
            compute_future_snapshot_slot(target_epoch, offset, current_slot),
            Err(GovernanceError::SnapshotSlotNotInFuture)
        ));
    }

    #[test]
    fn flush_recovery_target_moves_forward_excluding_discussion() {
        // flush_merkle_root is an admin-only recovery path. It does NOT use
        // proposal_target_epoch; it re-anchors the snapshot window forward off the
        // *current* epoch and intentionally omits the discussion window:
        //     target_epoch = current_epoch + snapshot_epoch_extension
        // (mirrors the computation in flush_merkle_root.rs). This pins that formula
        // and the intentional divergence from support_proposal's schedule.
        let current_epoch = 20u64;
        let discussion_epochs = 3u64;
        let snapshot_epoch_extension = 1u64;

        let flush_target = current_epoch + snapshot_epoch_extension;
        assert_eq!(flush_target, 21);

        // support_proposal, anchored on the same epoch, includes the discussion
        // window, so its target is exactly `discussion_epochs` later than flush's.
        let support_target =
            proposal_target_epoch(current_epoch, discussion_epochs, snapshot_epoch_extension)
                .unwrap();
        assert_eq!(support_target - flush_target, discussion_epochs);
    }

    #[test]
    fn target_epoch_includes_discussion_period() {
        // The discussion window must remain part of the schedule. Dropping it (as
        // the old flush did) shortened time-to-vote by exactly `discussion_epochs`.
        let with_discussion = proposal_target_epoch(9, 3, 1).unwrap();
        let without_discussion = proposal_target_epoch(9, 0, 1).unwrap();
        assert_eq!(with_discussion - without_discussion, 3);
    }

    #[test]
    fn target_epoch_rejects_overflow() {
        // Bounded, admin-set inputs should never reach this, but the checked math
        // surfaces a clean error instead of relying on the release overflow-checks panic.
        assert!(matches!(
            proposal_target_epoch(u64::MAX, 1, 0),
            Err(GovernanceError::ArithmeticOverflow)
        ));
    }
}
