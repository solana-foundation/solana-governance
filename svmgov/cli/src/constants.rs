// Default RPC endpoints
pub const DEFAULT_RPC_URL: &str = "https://api.mainnet-beta.solana.com";
pub const DEFAULT_WSS_URL: &str = "wss://api.mainnet-beta.solana.com";

// Network-specific default RPC URLs
pub const DEFAULT_MAINNET_RPC_URL: &str = "https://api.mainnet-beta.solana.com";
pub const DEFAULT_TESTNET_RPC_URL: &str = "https://api.testnet.solana.com";
pub const DEFAULT_OPERATOR_API_URL: &str = "https://ncn-governance.solana.com";

// Voting constants
pub const BASIS_POINTS_TOTAL: u64 = 10_000;

// --- Compute budget for support_proposal / retally_support -----------------
//
// Both instructions re-tally the whole supporter list on every call, so their
// cost is linear in `num_supporters`. Requesting a flat worst-case limit would
// overshoot a typical call by ~10x; priority fees price the *requested* limit
// rather than what is consumed, so the request is modelled instead.
//
// Keep in sync with the mirror in `frontend/src/chain/instructions/types.ts`.
// `tests/support_compute_budget.rs` asserts the model covers measured cost.

/// Fixed cost before the per-supporter re-tally. Measured at 22,434 CU for a
/// support with no prior supporters.
pub const SUPPORT_CU_BASE: u32 = 22_500;

/// Added per existing supporter: one `sol_get_epoch_stake` syscall (a fixed
/// ~110 CU under SIMD-0133) plus loop overhead. Measured slope ~132.
pub const SUPPORT_CU_PER_SUPPORTER: u32 = 132;

/// Extra units for the call that crosses the support threshold: it activates
/// voting and, unless the ballot box already exists, creates it through the
/// `init_ballot_box` CPI. Measured at ~26.8k above a non-activating call, at
/// the 64-operator whitelist maximum — `init_ballot_box` clones the whitelist
/// into the ballot box, so a longer list costs more. `tests/ncn_flow.rs`
/// exercises the real CPI and asserts this covers it. Always included: a caller
/// cannot know whether its own call will be the one that crosses.
pub const SUPPORT_CU_ACTIVATION: u32 = 28_000;

/// Margin over the modelled cost. Chiefly covers supporters that land between
/// reading `num_supporters` and the transaction executing, at ~132 CU each.
pub const SUPPORT_CU_HEADROOM_PERCENT: u32 = 15;

/// Per-transaction maximum a client may request.
pub const MAX_COMPUTE_UNIT_LIMIT: u32 = 1_400_000;

/// The program's `MAX_SUPPORTERS_LIMIT`. Used as the supporter count when the
/// real one cannot be read, so the request still covers the largest list the
/// program permits.
pub const MAX_SUPPORTERS_LIMIT: u32 = 2_000;

/// Compute-unit limit to request for a support/retally against a proposal that
/// currently has `num_supporters` supporters.
pub fn support_compute_unit_limit(num_supporters: u32) -> u32 {
    let modelled = SUPPORT_CU_BASE
        .saturating_add(SUPPORT_CU_PER_SUPPORTER.saturating_mul(num_supporters))
        .saturating_add(SUPPORT_CU_ACTIVATION);
    // div_ceil, matching Math.ceil in the TypeScript mirror — plain integer
    // division would round the two clients apart on inexact results.
    let with_headroom =
        (u64::from(modelled) * u64::from(100 + SUPPORT_CU_HEADROOM_PERCENT)).div_ceil(100);
    with_headroom.min(u64::from(MAX_COMPUTE_UNIT_LIMIT)) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mirrors the peak measured in `tests/support_compute_budget.rs` at the
    /// 2000-supporter cap (284,953 CU, ballot box already created).
    const MEASURED_AT_CAP: u32 = 284_953;

    #[test]
    fn covers_measured_cost_with_headroom() {
        // No supporters yet, and this call could be the one that activates:
        // ncn_flow measured 43,817-47,610 CU for that path.
        assert!(support_compute_unit_limit(0) > 49_473);
        assert!(support_compute_unit_limit(2_000) > MEASURED_AT_CAP);
    }

    #[test]
    fn scales_with_the_supporter_count() {
        // A small list must request far less than the old flat 600k, otherwise
        // the modelling gains nothing.
        assert!(support_compute_unit_limit(0) < 100_000);
        assert!(support_compute_unit_limit(50) < support_compute_unit_limit(500));
        assert!(support_compute_unit_limit(500) < support_compute_unit_limit(2_000));
    }

    #[test]
    fn matches_the_typescript_mirror() {
        // Both clients must request the same budget for the same state; these
        // values are pinned identically in the frontend's
        // supportComputeUnitLimit test.
        assert_eq!(support_compute_unit_limit(0), 58_075);
        assert_eq!(support_compute_unit_limit(2_000), 361_675);
    }

    #[test]
    fn the_fallback_supporter_count_covers_the_worst_case() {
        // When the supporter count cannot be read, callers pass
        // MAX_SUPPORTERS_LIMIT, which must request at least as much as any
        // real list could need.
        let fallback = support_compute_unit_limit(MAX_SUPPORTERS_LIMIT);
        assert!(fallback > MEASURED_AT_CAP);
        for n in [0u32, 1, 500, 1_999, MAX_SUPPORTERS_LIMIT] {
            assert!(support_compute_unit_limit(n) <= fallback);
        }
    }

    #[test]
    fn never_exceeds_the_per_transaction_maximum() {
        assert_eq!(support_compute_unit_limit(u32::MAX), MAX_COMPUTE_UNIT_LIMIT);
    }
}

// UI constants
pub const SPINNER_TICK_DURATION_MS: u64 = 100;

// Environment variable names
pub const SVMGOV_KEY_ENV: &str = "SVMGOV_KEY";
pub const SVMGOV_RPC_ENV: &str = "SVMGOV_RPC";
