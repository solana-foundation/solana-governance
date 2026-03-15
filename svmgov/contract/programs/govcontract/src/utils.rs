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
