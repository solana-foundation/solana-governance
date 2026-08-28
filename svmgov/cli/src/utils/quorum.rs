//! Vote breakdown and SGP-0001 outcome, for display by the CLI.
//!
//! Mirrors the website's Vote Breakdown card: participation against snapshot
//! stake (Art. IV.3, one third), For / Against / Abstain as a share of votes
//! cast, and a pass/fail reading of Art. IV.4 as the product FAQ states it —
//! two thirds of participating stake (`For + Against + Abstain`) must vote For.
//! If quorum is not met the outcome is inconclusive, not a fail.
//!
//! The snapshot total is required for quorum. It is taken from the proposal's
//! own snapshot (Art. IV.2), never a live cluster sum. `/meta?slot=` returns
//! that snapshot's total; a total from a different slot is refused.

use anchor_client::solana_sdk::native_token::LAMPORTS_PER_SOL;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::{Cell, CellAlignment, Table, presets::UTF8_FULL};

/// The `/meta` fields quorum needs. Kept separate from the HTTP response type
/// so this module stays RPC-free and unit-testable.
#[derive(Debug, Clone)]
pub struct SnapshotTotalSource {
    pub slot: u64,
    pub total_active_stake: Option<u64>,
}

pub const QUORUM_NUMERATOR: u64 = 1;
pub const QUORUM_DENOMINATOR: u64 = 3;
pub const SUPERMAJORITY_NUMERATOR: u64 = 2;
pub const SUPERMAJORITY_DENOMINATOR: u64 = 3;

const BAR_WIDTH: usize = 30;

/// Why the snapshot total could not be used as a quorum denominator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DenominatorError {
    NoMeta,
    /// `snapshot_slot` is 0 until voting activates.
    Unactivated,
    SlotMismatch {
        meta_slot: u64,
        proposal_slot: u64,
    },
    MissingTotal,
}

impl DenominatorError {
    pub fn hint(&self) -> String {
        match self {
            DenominatorError::NoMeta | DenominatorError::MissingTotal => {
                "snapshot total unavailable".to_string()
            }
            DenominatorError::Unactivated => "voting has not started (no snapshot yet)".to_string(),
            DenominatorError::SlotMismatch {
                meta_slot,
                proposal_slot,
            } => format!(
                "snapshot total unavailable (/meta is slot {meta_slot}, proposal is slot {proposal_slot})"
            ),
        }
    }
}

/// The total to measure quorum against, or why it cannot be established.
///
/// `/meta?slot=` is the proposal's own snapshot. A total from any other slot
/// is a different stake distribution and must not be used.
pub fn resolve_quorum_denominator(
    meta: Option<&SnapshotTotalSource>,
    proposal_snapshot_slot: u64,
) -> Result<u64, DenominatorError> {
    if proposal_snapshot_slot == 0 {
        return Err(DenominatorError::Unactivated);
    }
    let meta = meta.ok_or(DenominatorError::NoMeta)?;
    if meta.slot != proposal_snapshot_slot {
        return Err(DenominatorError::SlotMismatch {
            meta_slot: meta.slot,
            proposal_slot: proposal_snapshot_slot,
        });
    }
    match meta.total_active_stake {
        Some(total) if total > 0 => Ok(total),
        _ => Err(DenominatorError::MissingTotal),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VoteTally {
    pub for_lamports: u64,
    pub against_lamports: u64,
    pub abstain_lamports: u64,
    pub total_active_stake: Result<u64, DenominatorError>,
}

impl VoteTally {
    pub fn participating_lamports(&self) -> u64 {
        self.for_lamports
            .saturating_add(self.against_lamports)
            .saturating_add(self.abstain_lamports)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum QuorumStatus {
    Unknown {
        hint: String,
    },
    Known {
        total_active_stake: u64,
        /// 0–100, for display only. Quorum itself is decided in lamports.
        participation_percent: f64,
        is_met: bool,
    },
}

/// Art. IV.3: participating stake (`For + Against + Abstain`) must be at least
/// one third of snapshot stake. Compared in integer lamports so a displayed
/// `33.33%` cannot decide a borderline vote.
pub fn compute_quorum(tally: &VoteTally) -> QuorumStatus {
    match tally.total_active_stake {
        Err(ref err) => QuorumStatus::Unknown { hint: err.hint() },
        Ok(total) => {
            let participating = tally.participating_lamports();
            QuorumStatus::Known {
                total_active_stake: total,
                participation_percent: percent(participating, total),
                is_met: meets_fraction(participating, total, QUORUM_NUMERATOR, QUORUM_DENOMINATOR),
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalOutcome {
    /// Snapshot total unknown, so quorum (and therefore pass/fail) cannot be judged.
    Unknown,
    /// Quorum not met. SGP-0001: does not pass, does not fail.
    Inconclusive,
    Passing,
    Failing,
}

impl ProposalOutcome {
    /// `settled` is true once voting can no longer change the result
    /// (finalized, or the voting window has closed). SGP-0001's results are
    /// then Passed / Failed; while the vote is open they are Passing / Failing.
    /// Inconclusive is the quorum miss in either case — it is not a fail.
    pub fn label(self, settled: bool) -> &'static str {
        match (self, settled) {
            (ProposalOutcome::Unknown, _) => "Unknown (snapshot total unavailable)",
            (ProposalOutcome::Inconclusive, _) => "Inconclusive (quorum not met)",
            (ProposalOutcome::Passing, false) => "Passing",
            (ProposalOutcome::Passing, true) => "Passed",
            (ProposalOutcome::Failing, false) => "Failing",
            (ProposalOutcome::Failing, true) => "Failed",
        }
    }

    /// Compact label for `list-proposals`. Unknown is a dash so a support-phase
    /// row does not look like a failed vote.
    pub fn short_label(self, settled: bool) -> &'static str {
        match (self, settled) {
            (ProposalOutcome::Unknown, _) => "—",
            (ProposalOutcome::Inconclusive, _) => "Inconclusive",
            (ProposalOutcome::Passing, false) => "Passing",
            (ProposalOutcome::Passing, true) => "Passed",
            (ProposalOutcome::Failing, false) => "Failing",
            (ProposalOutcome::Failing, true) => "Failed",
        }
    }

    pub fn id(self, settled: bool) -> &'static str {
        match (self, settled) {
            (ProposalOutcome::Unknown, _) => "unknown",
            (ProposalOutcome::Inconclusive, _) => "inconclusive",
            (ProposalOutcome::Passing, false) => "passing",
            (ProposalOutcome::Passing, true) => "passed",
            (ProposalOutcome::Failing, false) => "failing",
            (ProposalOutcome::Failing, true) => "failed",
        }
    }
}

/// FAQ reading of SGP-0001: quorum of one third of snapshot stake, then two
/// thirds of participating stake For. Abstain counts toward quorum and sits in
/// the supermajority denominator, so a large abstain can block a pass even
/// with no Against votes.
pub fn compute_outcome(tally: &VoteTally) -> ProposalOutcome {
    match compute_quorum(tally) {
        QuorumStatus::Unknown { .. } => ProposalOutcome::Unknown,
        QuorumStatus::Known { is_met: false, .. } => ProposalOutcome::Inconclusive,
        QuorumStatus::Known { is_met: true, .. } => {
            let participating = tally.participating_lamports();
            if meets_fraction(
                tally.for_lamports,
                participating,
                SUPERMAJORITY_NUMERATOR,
                SUPERMAJORITY_DENOMINATOR,
            ) {
                ProposalOutcome::Passing
            } else {
                ProposalOutcome::Failing
            }
        }
    }
}

/// Compact For / Against / Abstain plus SGP-0001 outcome for `list-proposals`.
pub fn list_vote_summary(tally: &VoteTally) -> ListVoteSummary {
    ListVoteSummary {
        outcome: compute_outcome(tally),
        for_stake: format_sol_display(tally.for_lamports),
        against_stake: format_sol_display(tally.against_lamports),
        abstain_stake: format_sol_display(tally.abstain_lamports),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListVoteSummary {
    pub outcome: ProposalOutcome,
    pub for_stake: String,
    pub against_stake: String,
    pub abstain_stake: String,
}

/// `numerator >= (denominator * num) / den`, in u128 so mainnet stake cannot
/// wrap. Matches the website: compared in lamports, using truncating division
/// the same way `(total * 1) / 3` is written in `frontend/src/chain/quorum.ts`.
fn meets_fraction(numerator: u64, denominator: u64, num: u64, den: u64) -> bool {
    if denominator == 0 {
        return false;
    }
    (numerator as u128) >= (denominator as u128) * (num as u128) / (den as u128)
}

fn percent(part: u64, whole: u64) -> f64 {
    if whole == 0 {
        0.0
    } else {
        (part as f64 / whole as f64) * 100.0
    }
}

pub fn format_sol_display(lamports: u64) -> String {
    let sol = lamports as f64 / LAMPORTS_PER_SOL as f64;
    if sol >= 1_000_000_000.0 {
        format!("{:.2}B SOL", sol / 1_000_000_000.0)
    } else if sol >= 10_000_000.0 {
        format!("{:.2}M SOL", sol / 1_000_000.0)
    } else {
        format!("{} SOL", with_thousands(sol))
    }
}

fn with_thousands(sol: f64) -> String {
    let formatted = format!("{sol:.2}");
    let (whole, frac) = formatted
        .split_once('.')
        .expect("format!.2 always has a dot");
    format!("{}.{}", group_thousands(whole), frac)
}

fn group_thousands(whole: &str) -> String {
    let mut grouped = String::new();
    for (i, c) in whole.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    grouped.chars().rev().collect()
}

fn format_percent(value: f64) -> String {
    format!("{value:.2}%")
}

fn participation_bar(participation_percent: f64) -> String {
    let frac = (participation_percent / 100.0).clamp(0.0, 1.0);
    let filled = ((frac * BAR_WIDTH as f64).round() as usize).min(BAR_WIDTH);
    format!("{}{}", "█".repeat(filled), "░".repeat(BAR_WIDTH - filled))
}

/// Lines above the For/Against/Abstain table. Unit-tested so the website card
/// and the CLI stay aligned without an RPC.
pub fn breakdown_summary_lines(tally: &VoteTally, settled: bool) -> Vec<String> {
    let quorum = compute_quorum(tally);
    let outcome = compute_outcome(tally);
    let participating = tally.participating_lamports();

    let mut lines = Vec::new();
    match &quorum {
        QuorumStatus::Known {
            total_active_stake,
            participation_percent,
            is_met,
        } => {
            lines.push(format!(
                "Snapshot stake:              {}",
                format_sol_display(*total_active_stake)
            ));
            let met = if *is_met { " — quorum met" } else { "" };
            lines.push(format!(
                "Participation (1/3 needed):  {}  {}{}",
                format_percent(*participation_percent),
                participation_bar(*participation_percent),
                met
            ));
        }
        QuorumStatus::Unknown { hint } => {
            lines.push(format!("Participation (1/3 needed):  — ({hint})"));
        }
    }

    if participating == 0 {
        lines.push("Supermajority (2/3 needed):  — (no votes yet)".to_string());
    } else {
        lines.push(format!(
            "Supermajority (2/3 needed):  {} For",
            format_percent(percent(tally.for_lamports, participating))
        ));
    }
    lines.push(format!(
        "Outcome:                     {}",
        outcome.label(settled)
    ));
    lines
}

pub fn print_vote_breakdown(tally: &VoteTally, terminal_width: u16, settled: bool) {
    println!("\nVote breakdown (SGP-0001)");
    for line in breakdown_summary_lines(tally, settled) {
        println!("  {line}");
    }

    let participating = tally.participating_lamports();
    let of_votes = |part: u64| {
        if participating == 0 {
            "—".to_string()
        } else {
            format_percent(percent(part, participating))
        }
    };
    let of_snapshot = |part: u64| match tally.total_active_stake {
        Ok(total) => format_percent(percent(part, total)),
        Err(_) => "—".to_string(),
    };

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_width(terminal_width);
    table.set_header(vec!["Choice", "Stake", "Of votes", "Of snapshot"]);

    for (label, lamports) in [
        ("For", tally.for_lamports),
        ("Against", tally.against_lamports),
        ("Abstain", tally.abstain_lamports),
    ] {
        table.add_row(vec![
            Cell::new(label),
            Cell::new(format_sol_display(lamports)).set_alignment(CellAlignment::Right),
            Cell::new(of_votes(lamports)).set_alignment(CellAlignment::Right),
            Cell::new(of_snapshot(lamports)).set_alignment(CellAlignment::Right),
        ]);
    }

    println!("{table}");
}

#[cfg(test)]
mod tests {
    use super::*;

    const NETWORK_STAKE: u64 = 400_000_000 * LAMPORTS_PER_SOL;
    const ONE_THIRD: u64 = NETWORK_STAKE / 3;
    const ONE_SOL: u64 = LAMPORTS_PER_SOL;

    fn tally(f: u64, a: u64, ab: u64, total: Result<u64, DenominatorError>) -> VoteTally {
        VoteTally {
            for_lamports: f,
            against_lamports: a,
            abstain_lamports: ab,
            total_active_stake: total,
        }
    }

    fn known(total: u64) -> Result<u64, DenominatorError> {
        Ok(total)
    }

    fn meta(slot: u64, total: Option<u64>) -> SnapshotTotalSource {
        SnapshotTotalSource {
            slot,
            total_active_stake: total,
        }
    }

    #[test]
    fn quorum_is_one_third() {
        assert_eq!(QUORUM_NUMERATOR, 1);
        assert_eq!(QUORUM_DENOMINATOR, 3);
    }

    #[test]
    fn missing_or_zero_total_is_unknown_not_zero() {
        let unknown = compute_quorum(&tally(1, 2, 3, Err(DenominatorError::NoMeta)));
        assert!(matches!(unknown, QuorumStatus::Unknown { .. }));
        assert_eq!(
            compute_outcome(&tally(1, 2, 3, Err(DenominatorError::NoMeta))),
            ProposalOutcome::Unknown
        );

        let zero = compute_quorum(&tally(1, 0, 0, Err(DenominatorError::MissingTotal)));
        assert!(matches!(zero, QuorumStatus::Unknown { .. }));
    }

    #[test]
    fn no_votes_is_known_and_not_met() {
        let status = compute_quorum(&tally(0, 0, 0, known(NETWORK_STAKE)));
        match status {
            QuorumStatus::Known {
                participation_percent,
                is_met,
                ..
            } => {
                assert_eq!(participation_percent, 0.0);
                assert!(!is_met);
            }
            QuorumStatus::Unknown { .. } => panic!("expected known"),
        }
        assert_eq!(
            compute_outcome(&tally(0, 0, 0, known(NETWORK_STAKE))),
            ProposalOutcome::Inconclusive
        );
    }

    #[test]
    fn abstain_counts_toward_quorum() {
        let status = compute_quorum(&tally(0, 0, ONE_THIRD, known(NETWORK_STAKE)));
        assert!(matches!(status, QuorumStatus::Known { is_met: true, .. }));
    }

    #[test]
    fn quorum_is_met_exactly_at_one_third_and_not_just_below() {
        let at = compute_quorum(&tally(ONE_THIRD, 0, 0, known(NETWORK_STAKE)));
        let below = compute_quorum(&tally(ONE_THIRD - ONE_SOL, 0, 0, known(NETWORK_STAKE)));
        assert!(matches!(at, QuorumStatus::Known { is_met: true, .. }));
        assert!(matches!(below, QuorumStatus::Known { is_met: false, .. }));
    }

    #[test]
    fn display_rounding_does_not_decide_a_borderline_vote() {
        let below = compute_quorum(&tally(ONE_THIRD - ONE_SOL, 0, 0, known(NETWORK_STAKE)));
        match below {
            QuorumStatus::Known {
                participation_percent,
                is_met,
                ..
            } => {
                assert_eq!(format!("{participation_percent:.2}"), "33.33");
                assert!(!is_met);
            }
            QuorumStatus::Unknown { .. } => panic!("expected known"),
        }
    }

    #[test]
    fn all_three_buckets_count_as_participation() {
        let status = compute_quorum(&tally(
            NETWORK_STAKE / 2,
            NETWORK_STAKE / 4,
            NETWORK_STAKE / 8,
            known(NETWORK_STAKE),
        ));
        match status {
            QuorumStatus::Known {
                participation_percent,
                is_met,
                ..
            } => {
                assert!((participation_percent - 87.5).abs() < 1e-9);
                assert!(is_met);
            }
            QuorumStatus::Unknown { .. } => panic!("expected known"),
        }
    }

    #[test]
    fn two_thirds_of_participating_for_is_a_pass() {
        // Quorum: all of network votes. Supermajority: exactly 2/3 For.
        let passing = tally(
            NETWORK_STAKE * 2 / 3,
            NETWORK_STAKE / 3,
            0,
            known(NETWORK_STAKE),
        );
        assert_eq!(compute_outcome(&passing), ProposalOutcome::Passing);

        let failing = tally(
            NETWORK_STAKE * 2 / 3 - ONE_SOL,
            NETWORK_STAKE / 3 + ONE_SOL,
            0,
            known(NETWORK_STAKE),
        );
        assert_eq!(compute_outcome(&failing), ProposalOutcome::Failing);
    }

    #[test]
    fn abstain_sits_in_the_supermajority_denominator() {
        // 60% For, 40% Abstain, no Against: would pass if Abstain were ignored,
        // fails under the FAQ reading (2/3 of participating).
        let tally = tally(
            NETWORK_STAKE * 6 / 10,
            0,
            NETWORK_STAKE * 4 / 10,
            known(NETWORK_STAKE),
        );
        assert!(matches!(
            compute_quorum(&tally),
            QuorumStatus::Known { is_met: true, .. }
        ));
        assert_eq!(compute_outcome(&tally), ProposalOutcome::Failing);
    }

    #[test]
    fn unanimous_for_without_quorum_is_inconclusive_not_passing() {
        // 100% For of a 10% turnout.
        let tally = tally(NETWORK_STAKE / 10, 0, 0, known(NETWORK_STAKE));
        assert_eq!(compute_outcome(&tally), ProposalOutcome::Inconclusive);
    }

    #[test]
    fn screenshot_percentages_are_share_of_votes_cast() {
        // From governance.solana.com/proposal/4aFA8K65zYZjmx16qaXhMLW9QY7URRvwyk4KQo2zLz8k
        let for_lamports = 155_220_000 * LAMPORTS_PER_SOL;
        let against_lamports = 367_701_08 * LAMPORTS_PER_SOL / 100; // 367,701.08 SOL
        let abstain_lamports = 7_251_964_70 * LAMPORTS_PER_SOL / 100; // 7,251,964.70 SOL
        let participating = for_lamports + against_lamports + abstain_lamports;
        assert_eq!(
            format!("{:.2}", percent(for_lamports, participating)),
            "95.32"
        );
        assert_eq!(
            format!("{:.2}", percent(against_lamports, participating)),
            "0.23"
        );
        assert_eq!(
            format!("{:.2}", percent(abstain_lamports, participating)),
            "4.45"
        );
    }

    #[test]
    fn compact_sol_matches_the_website_thresholds() {
        assert_eq!(
            format_sol_display(433_490_000 * LAMPORTS_PER_SOL),
            "433.49M SOL"
        );
        assert_eq!(
            format_sol_display(155_220_000 * LAMPORTS_PER_SOL),
            "155.22M SOL"
        );
        assert_eq!(
            format_sol_display(367_701 * LAMPORTS_PER_SOL + LAMPORTS_PER_SOL * 8 / 100),
            "367,701.08 SOL"
        );
        assert_eq!(format_sol_display(LAMPORTS_PER_SOL), "1.00 SOL");
    }

    #[test]
    fn resolve_uses_the_total_only_when_the_slot_matches() {
        let m = meta(500, Some(NETWORK_STAKE));
        assert_eq!(resolve_quorum_denominator(Some(&m), 500), Ok(NETWORK_STAKE));
        assert_eq!(
            resolve_quorum_denominator(Some(&m), 499),
            Err(DenominatorError::SlotMismatch {
                meta_slot: 500,
                proposal_slot: 499,
            })
        );
        assert_eq!(
            resolve_quorum_denominator(Some(&meta(500, None)), 500),
            Err(DenominatorError::MissingTotal)
        );
        assert_eq!(
            resolve_quorum_denominator(None, 500),
            Err(DenominatorError::NoMeta)
        );
        assert_eq!(
            resolve_quorum_denominator(Some(&meta(0, Some(1))), 0),
            Err(DenominatorError::Unactivated)
        );
    }

    #[test]
    fn summary_names_quorum_and_outcome() {
        let lines = breakdown_summary_lines(
            &tally(
                NETWORK_STAKE / 2,
                NETWORK_STAKE / 10,
                0,
                known(NETWORK_STAKE),
            ),
            false,
        );
        assert!(lines.iter().any(|l| l.contains("quorum met")));
        assert!(lines.iter().any(|l| l.contains("Passing")));
        assert!(lines.iter().any(|l| l.contains("1/3 needed")));
        assert!(lines.iter().any(|l| l.contains("2/3 needed")));
    }

    #[test]
    fn settled_summary_uses_past_tense() {
        let lines = breakdown_summary_lines(
            &tally(
                NETWORK_STAKE / 2,
                NETWORK_STAKE / 10,
                0,
                known(NETWORK_STAKE),
            ),
            true,
        );
        assert!(lines.iter().any(|l| l.contains("Passed")));
        assert!(!lines.iter().any(|l| l.contains("Passing")));
    }

    #[test]
    fn summary_without_a_total_does_not_pretend_quorum_failed() {
        let lines = breakdown_summary_lines(&tally(1, 0, 0, Err(DenominatorError::NoMeta)), false);
        assert!(
            lines
                .iter()
                .any(|l| l.contains("snapshot total unavailable"))
        );
        assert!(lines.iter().any(|l| l.contains("Unknown")));
        assert!(!lines.iter().any(|l| l.contains("quorum met")));
        assert!(!lines.iter().any(|l| l.contains("Failing")));
    }

    #[test]
    fn list_summary_is_compact_outcome_and_stake() {
        let passing = list_vote_summary(&tally(
            NETWORK_STAKE / 2,
            NETWORK_STAKE / 10,
            0,
            known(NETWORK_STAKE),
        ));
        assert_eq!(passing.outcome.short_label(false), "Passing");
        assert_eq!(passing.outcome.id(false), "passing");
        assert_eq!(passing.outcome.short_label(true), "Passed");
        assert_eq!(passing.outcome.id(true), "passed");
        assert_eq!(passing.for_stake, "200.00M SOL");
        assert_eq!(passing.against_stake, "40.00M SOL");
        assert_eq!(passing.abstain_stake, "0.00 SOL");

        let unknown = list_vote_summary(&tally(1, 0, 0, Err(DenominatorError::Unactivated)));
        assert_eq!(unknown.outcome.short_label(false), "—");
        assert_eq!(unknown.outcome.id(false), "unknown");

        let no_quorum = list_vote_summary(&tally(NETWORK_STAKE / 10, 0, 0, known(NETWORK_STAKE)));
        assert_eq!(no_quorum.outcome.short_label(true), "Inconclusive");
        assert_eq!(no_quorum.outcome.id(true), "inconclusive");

        let failing = list_vote_summary(&tally(
            NETWORK_STAKE * 2 / 3 - ONE_SOL,
            NETWORK_STAKE / 3 + ONE_SOL,
            0,
            known(NETWORK_STAKE),
        ));
        assert_eq!(failing.outcome.short_label(false), "Failing");
        assert_eq!(failing.outcome.short_label(true), "Failed");
        assert_eq!(failing.outcome.id(true), "failed");
    }
}
