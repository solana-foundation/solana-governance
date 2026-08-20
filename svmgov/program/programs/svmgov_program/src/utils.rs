use anchor_lang::prelude::Pubkey;

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
/// ```
/// use svmgov_program::stake_weight_bp;
///
/// let weight_bp = stake_weight_bp!(40_001u64, 380_000_000u64).unwrap();
/// assert_eq!(weight_bp, 1);
/// ```
#[macro_export]
macro_rules! stake_weight_bp {
    ($validator_stake:expr, $cluster_stake:expr) => {{
        ($validator_stake as u128)
            .checked_mul($crate::BASIS_POINTS_MAX as u128)
            .and_then(|product| product.checked_div($cluster_stake as u128))
            .ok_or($crate::GovernanceError::ArithmeticOverflow)
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
/// ```
/// use svmgov_program::calculate_vote_lamports;
///
/// let vote_lamports = calculate_vote_lamports!(1_000_000u64, 2_500u64).unwrap();
/// assert_eq!(vote_lamports, 250_000); // 25% of 1 SOL
/// ```
#[macro_export]
macro_rules! calculate_vote_lamports {
    ($stake:expr, $basis_points:expr) => {{
        ($stake as u128)
            .checked_mul($basis_points as u128)
            .and_then(|product| product.checked_div($crate::BASIS_POINTS_MAX as u128))
            .ok_or($crate::GovernanceError::ArithmeticOverflow)
            .map(|result| result as u64)
    }};
}

/// Validates that the input points to the approved GitHub proposal repository.
pub fn is_valid_github_link(link: &str) -> bool {
    const PREFIX: &str = "https://github.com/";
    const REPOSITORY_PATH: &str = "solana-foundation/solana-governance-proposals";
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

    // path must start with repository path followed by another segment
    if !path
        .strip_prefix(REPOSITORY_PATH)
        .is_some_and(|suffix| suffix.starts_with('/'))
    {
        return false;
    }

    // URL consumers normalize `..` by walking to a parent path. Reject it here so the
    // raw on-chain string and the URL that clients resolve always identify the same repo.
    if path.contains("/../") || path.ends_with("/..") {
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
/// ```
/// use svmgov_program::get_epoch_slot_range;
///
/// assert_eq!(get_epoch_slot_range(0), (0, 431_999));
/// assert_eq!(get_epoch_slot_range(1), (432_000, 863_999));
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
/// `support_proposal` and `retally_support` call this with the epoch in which
/// the threshold-crossing call lands — any epoch inside the support window
/// `[creation_epoch, creation_epoch + max_support_epochs]` — so the initial
/// voting schedule includes the full `discussion_epochs` window from that
/// point. A proposal that crosses the threshold early votes earlier.
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

/// Validates that `current_epoch` falls inside a proposal's support window:
/// `creation_epoch <= current_epoch <= creation_epoch + max_support_epochs`
/// (inclusive on both ends, so `max_support_epochs == 0` means "creation epoch
/// only", exactly as the `GlobalConfig` field documents).
///
/// Returns `SupportPeriodExpired` past the window end and `NotInSupportPeriod`
/// before the creation epoch (defensive: unreachable in practice since the
/// clock is monotonic and `creation_epoch` is stamped from it).
pub fn check_support_window(
    current_epoch: u64,
    creation_epoch: u64,
    max_support_epochs: u64,
) -> core::result::Result<(), crate::error::GovernanceError> {
    if current_epoch < creation_epoch {
        return Err(crate::error::GovernanceError::NotInSupportPeriod);
    }
    let window_end = creation_epoch
        .checked_add(max_support_epochs)
        .ok_or(crate::error::GovernanceError::ArithmeticOverflow)?;
    if current_epoch > window_end {
        return Err(crate::error::GovernanceError::SupportPeriodExpired);
    }
    Ok(())
}

/// Re-tallies total supporter stake by looking up each supporter's stake *at
/// the current epoch* through `get_stake` (on-chain: the `sol_get_epoch_stake`
/// syscall). Called on every support/retally so earlier epochs' stake readings
/// are never reused — a supporter whose stake changed since they signed is
/// counted at their current weight.
pub fn tally_supporter_stakes<F>(
    supporters: &[Pubkey],
    get_stake: F,
) -> core::result::Result<u64, crate::error::GovernanceError>
where
    F: Fn(&Pubkey) -> u64,
{
    let mut total: u64 = 0;
    for vote_pubkey in supporters {
        total = total
            .checked_add(get_stake(vote_pubkey))
            .ok_or(crate::error::GovernanceError::ArithmeticOverflow)?;
    }
    Ok(total)
}

/// Computes the minimum cluster stake (in lamports) a proposal must gather to
/// activate voting: `cluster_stake * min_bps / 10_000`, using u128 internally
/// so `u64::MAX * 10_000` cannot overflow.
pub fn min_stake_threshold(
    cluster_stake: u64,
    min_bps: u64,
) -> core::result::Result<u64, crate::error::GovernanceError> {
    (cluster_stake as u128)
        .checked_mul(min_bps as u128)
        .and_then(|v| v.checked_div(crate::constants::BASIS_POINTS_MAX as u128))
        .ok_or(crate::error::GovernanceError::ArithmeticOverflow)
        .map(|result| result as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::GovernanceError;

    #[test]
    fn github_link_requires_the_allowed_repository_without_traversal() {
        assert!(is_valid_github_link(
            "https://github.com/solana-foundation/solana-governance-proposals/blob/main/proposals/sgp-0001-title.md"
        ));
        assert!(!is_valid_github_link(
            "https://github.com/attacker/repo/blob/ref/0022-x.md"
        ));
        assert!(!is_valid_github_link(
            "https://github.com/solana-foundation/solana-governance-proposals/../../attacker/repo/blob/ref/0022-x.md"
        ));
        assert!(!is_valid_github_link(
            "https://github.com/solana-foundation/solana-governance-proposals"
        ));
    }

    // --- check_support_window ---

    #[test]
    fn support_window_accepts_creation_epoch() {
        // Window start is inclusive: supporting in the epoch the proposal was
        // created must be valid (this is all that worked when the gate was
        // strict equality with max_support_epochs = 0).
        assert!(check_support_window(100, 100, 3).is_ok());
    }

    #[test]
    fn support_window_accepts_mid_window_epoch() {
        assert!(check_support_window(102, 100, 3).is_ok());
    }

    #[test]
    fn support_window_accepts_last_epoch_inclusive() {
        // Window end is inclusive: creation 100 + max 3 => epoch 103 is valid.
        assert!(check_support_window(103, 100, 3).is_ok());
    }

    #[test]
    fn support_window_rejects_one_past_end() {
        assert!(matches!(
            check_support_window(104, 100, 3),
            Err(GovernanceError::SupportPeriodExpired)
        ));
    }

    #[test]
    fn support_window_zero_max_means_creation_epoch_only() {
        // Backward-compatible meaning of max_support_epochs = 0 (deployed config).
        assert!(check_support_window(100, 100, 0).is_ok());
        assert!(matches!(
            check_support_window(101, 100, 0),
            Err(GovernanceError::SupportPeriodExpired)
        ));
    }

    #[test]
    fn support_window_rejects_epoch_before_creation() {
        // Defensive only: clock.epoch < creation_epoch cannot happen on-chain.
        assert!(matches!(
            check_support_window(99, 100, 3),
            Err(GovernanceError::NotInSupportPeriod)
        ));
    }

    #[test]
    fn support_window_rejects_end_overflow() {
        // creation + max overflowing u64 must surface a clean error, not wrap.
        // current == creation so the lower-bound check passes and the overflow
        // path is actually exercised.
        assert!(matches!(
            check_support_window(u64::MAX, u64::MAX, 1),
            Err(GovernanceError::ArithmeticOverflow)
        ));
    }

    // --- tally_supporter_stakes ---

    #[test]
    fn tally_sums_all_supporters_via_lookup() {
        let a = Pubkey::new_unique();
        let b = Pubkey::new_unique();
        let c = Pubkey::new_unique();
        let stake = |pk: &Pubkey| -> u64 {
            if *pk == a {
                100
            } else if *pk == b {
                250
            } else {
                7
            }
        };
        assert_eq!(tally_supporter_stakes(&[a, b, c], stake).unwrap(), 357);
    }

    #[test]
    fn tally_empty_list_is_zero() {
        let called = std::cell::Cell::new(false);
        let stake = |_: &Pubkey| -> u64 {
            called.set(true);
            1
        };
        assert_eq!(tally_supporter_stakes(&[], stake).unwrap(), 0);
        assert!(!called.get(), "lookup must not run for an empty list");
    }

    #[test]
    fn tally_reflects_current_stake_not_recorded_stake() {
        // The core multi-epoch requirement: the same supporter list tallied
        // against a different epoch's stake table gives a different total —
        // nothing from the earlier reading is retained.
        let a = Pubkey::new_unique();
        let b = Pubkey::new_unique();
        let supporters = [a, b];

        let epoch_n = |pk: &Pubkey| -> u64 {
            if *pk == a {
                500
            } else {
                300
            }
        };
        let epoch_n_plus_2 = |pk: &Pubkey| -> u64 {
            if *pk == a {
                50
            } else {
                300
            }
        };

        assert_eq!(tally_supporter_stakes(&supporters, epoch_n).unwrap(), 800);
        assert_eq!(
            tally_supporter_stakes(&supporters, epoch_n_plus_2).unwrap(),
            350
        );
    }

    #[test]
    fn tally_rejects_overflow() {
        let supporters = [Pubkey::new_unique(), Pubkey::new_unique()];
        let stake = |_: &Pubkey| -> u64 { u64::MAX };
        assert!(matches!(
            tally_supporter_stakes(&supporters, stake),
            Err(GovernanceError::ArithmeticOverflow)
        ));
    }

    // --- min_stake_threshold ---

    #[test]
    fn threshold_computes_bps_percentage() {
        // 10% (1000 bps) of 1000 lamports = 100.
        assert_eq!(min_stake_threshold(1_000, 1_000).unwrap(), 100);
        // 0.5% (50 bps) of 1_000_000 = 5_000.
        assert_eq!(min_stake_threshold(1_000_000, 50).unwrap(), 5_000);
    }

    #[test]
    fn threshold_zero_bps_is_zero() {
        assert_eq!(min_stake_threshold(u64::MAX, 0).unwrap(), 0);
    }

    #[test]
    fn threshold_full_stake_at_max_bps_no_overflow() {
        // u64::MAX * 10_000 exceeds u64 but must not overflow (u128 internally),
        // and 100% of the cluster is the cluster itself.
        assert_eq!(min_stake_threshold(u64::MAX, 10_000).unwrap(), u64::MAX);
    }

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
