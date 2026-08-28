//! Placing a proposal in its lifecycle, for display by the CLI.
//!
//! Answers two questions about a proposal: which phase it is in right now, and
//! what the epoch boundaries of every phase are. Both are needed by the detail
//! view, the list table, the `--status` filter and the JSON output, so they live
//! here rather than being recomputed at each call site.
//!
//! The program commits a proposal's schedule on-chain the moment support
//! crosses its threshold, writing `voting`, `start_epoch` and `end_epoch`. Those
//! fields are the source of truth wherever they are set; the `GlobalConfig`
//! durations are consulted only to project a schedule for a proposal that has
//! not activated yet, and that projection is flagged so callers can label it.
//!
//! Input is a [`PhaseInputs`] built from the `Proposal` and `GlobalConfig`
//! accounts — plain values, so the logic is unit-testable without an RPC.
//! Output is a [`ProposalPhase`] (where it is now), a [`PhaseTimeline`] (the
//! epoch windows, with anything the chain does not record left as `None`), and
//! [`PhaseInputs::epochs_remaining`] (how long the current phase has left).

use crate::svmgov_program::accounts::{GlobalConfig, Proposal};

/// Everything needed to place a proposal in its lifecycle, copied off the two
/// accounts so the logic below does not depend on the generated types.
#[derive(Debug, Clone, Copy)]
pub struct PhaseInputs {
    pub creation_epoch: u64,
    pub start_epoch: u64,
    pub end_epoch: u64,
    pub voting: bool,
    pub finalized: bool,
    pub max_support_epochs: u64,
    pub discussion_epochs: u64,
    pub snapshot_epoch_extension: u64,
    pub voting_epochs: u64,
}

impl PhaseInputs {
    pub fn new(proposal: &Proposal, config: &GlobalConfig) -> Self {
        Self {
            creation_epoch: proposal.creation_epoch,
            start_epoch: proposal.start_epoch,
            end_epoch: proposal.end_epoch,
            voting: proposal.voting,
            finalized: proposal.finalized,
            max_support_epochs: config.max_support_epochs,
            discussion_epochs: config.discussion_epochs,
            snapshot_epoch_extension: config.snapshot_epoch_extension,
            voting_epochs: config.voting_epochs,
        }
    }

    /// How many epochs until the current phase gives way to the next, with a
    /// label for what comes after. `None` once the proposal has stopped moving.
    pub fn epochs_remaining(&self, current_epoch: u64) -> Option<(u64, &'static str)> {
        let timeline = PhaseTimeline::new(self);
        match ProposalPhase::new(self, current_epoch) {
            ProposalPhase::Support => Some((
                timeline
                    .support
                    .map(|(_, end)| end.saturating_sub(current_epoch))
                    .unwrap_or(0),
                "until the support window closes",
            )),
            ProposalPhase::Discussion => Some((
                timeline.snapshot.saturating_sub(current_epoch),
                "until snapshot",
            )),
            ProposalPhase::Snapshot => Some((
                timeline.voting.0.saturating_sub(current_epoch),
                "until voting opens",
            )),
            ProposalPhase::Voting => Some((
                self.end_epoch.saturating_sub(current_epoch),
                "until voting ends",
            )),
            ProposalPhase::Ended | ProposalPhase::Finalized | ProposalPhase::Failed => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalPhase {
    Support,
    Discussion,
    Snapshot,
    Voting,
    Ended,
    Finalized,
    Failed,
}

impl ProposalPhase {
    /// The phase `p` is in at `current_epoch`.
    pub fn new(p: &PhaseInputs, current_epoch: u64) -> Self {
        if p.finalized {
            return Self::Finalized;
        }

        if p.voting {
            // Support succeeded and the program pinned the schedule, so the
            // config model is not consulted from here on.
            if current_epoch >= p.end_epoch {
                return Self::Ended;
            }
            if current_epoch >= p.start_epoch {
                return Self::Voting;
            }
            // The snapshot is taken in the epoch immediately before voting
            // opens. Written as an addition so a start_epoch of 0 cannot
            // underflow.
            if current_epoch + 1 >= p.start_epoch {
                return Self::Snapshot;
            }
            return Self::Discussion;
        }

        // Not activated. The program only accepts support inside
        // [creation_epoch, creation_epoch + max_support_epochs]; past that with
        // `voting` still unset, the proposal can never advance.
        if current_epoch <= p.creation_epoch.saturating_add(p.max_support_epochs) {
            Self::Support
        } else {
            Self::Failed
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Support => "Support",
            Self::Discussion => "Discussion",
            Self::Snapshot => "Snapshot",
            Self::Voting => "Voting",
            Self::Ended => "Ended (awaiting finalization)",
            Self::Finalized => "Finalized",
            Self::Failed => "Failed (support threshold not reached)",
        }
    }

    /// Voting is over, so an SGP-0001 pass/fail is past tense (`Passed` /
    /// `Failed`). `Finalized` is the on-chain lock; `Ended` is the same result
    /// waiting for that lock. Inconclusive does not change tense.
    pub fn vote_is_settled(self) -> bool {
        matches!(self, Self::Finalized | Self::Ended)
    }

    /// Lowercase identifier used by `--status` filters and JSON output.
    pub fn id(self) -> &'static str {
        match self {
            Self::Support => "support",
            Self::Discussion => "discussion",
            Self::Snapshot => "snapshot",
            Self::Voting => "voting",
            Self::Ended => "ended",
            Self::Finalized => "finalized",
            Self::Failed => "failed",
        }
    }
}

/// Epoch ranges for each phase.
///
/// The epoch support crossed the threshold is **not recorded on-chain** and
/// cannot be recovered from `start_epoch`: `flush_merkle_root` re-anchors with a
/// different formula from `activate_voting` (it omits `discussion_epochs`), and
/// `update_config` can change the durations after a proposal has activated.
/// Inverting the activation formula against a flushed or reconfigured proposal
/// produces a crossing epoch that can predate the proposal itself. So once
/// activated, the support window's end and the discussion window's start are
/// reported as unknown rather than guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseTimeline {
    pub created: u64,
    /// `None` once activated — see the note above.
    pub support: Option<(u64, u64)>,
    /// Start is `None` once activated; the end is always known.
    pub discussion: (Option<u64>, u64),
    pub snapshot: u64,
    pub voting: (u64, u64),
    /// The windows were projected from the config because the proposal has not
    /// activated; they move if it crosses earlier than its window's last epoch.
    pub projected: bool,
}

impl PhaseTimeline {
    pub fn new(p: &PhaseInputs) -> Self {
        if p.voting {
            // Everything here comes straight from stored state. `start_epoch - 1`
            // is the snapshot epoch by construction in *both* the activate_voting
            // and flush_merkle_root paths, so it survives a re-anchor.
            let snapshot = p.start_epoch.saturating_sub(1);
            return Self {
                created: p.creation_epoch,
                support: None,
                discussion: (None, snapshot.saturating_sub(1)),
                snapshot,
                voting: (p.start_epoch, p.end_epoch),
                projected: false,
            };
        }

        // Not activated: project from the config, assuming the latest epoch it
        // could still cross.
        let crossing = p.creation_epoch.saturating_add(p.max_support_epochs);
        let snapshot = crossing
            .saturating_add(p.discussion_epochs)
            .saturating_add(p.snapshot_epoch_extension);
        let voting_start = snapshot.saturating_add(1);

        Self {
            created: p.creation_epoch,
            support: Some((p.creation_epoch, crossing)),
            discussion: (Some(crossing), snapshot.saturating_sub(1)),
            snapshot,
            voting: (voting_start, voting_start.saturating_add(p.voting_epochs)),
            projected: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "Double Disinflation" as it stood on-chain in epoch 1012: created in
    /// 1011, crossed the threshold in 1012 — early in a seven-epoch support
    /// window — so the program scheduled voting for 1021-1024.
    fn double_disinflation() -> PhaseInputs {
        PhaseInputs {
            creation_epoch: 1011,
            start_epoch: 1021,
            end_epoch: 1024,
            voting: true,
            finalized: false,
            max_support_epochs: 7,
            discussion_epochs: 7,
            snapshot_epoch_extension: 1,
            voting_epochs: 3,
        }
    }

    #[test]
    fn an_early_crossing_is_not_reported_as_still_gathering_support() {
        // The reported bug: the CLI said "Support" with "6 epoch(s) until
        // discussion" for a proposal that had already advanced, because
        // current_epoch was still inside the config-derived support window.
        let p = double_disinflation();
        assert_eq!(ProposalPhase::new(&p, 1012), ProposalPhase::Discussion);
        assert_eq!(
            p.epochs_remaining(1012).map(|(n, _)| n),
            Some(8) // 1020 - 1012
        );
    }

    #[test]
    fn an_activated_timeline_uses_only_stored_state() {
        let t = PhaseTimeline::new(&double_disinflation());
        assert!(!t.projected);
        assert_eq!(t.created, 1011);
        // The epoch support crossed is not recorded on-chain, so it is not
        // claimed. `start_epoch - 1` gives the snapshot epoch in both the
        // activate_voting and flush_merkle_root paths; 1020 matches the stored
        // snapshot_slot of 440_641_000.
        assert_eq!(t.support, None);
        assert_eq!(t.discussion, (None, 1019));
        assert_eq!(t.snapshot, 1020);
        assert_eq!(t.voting, (1021, 1024));
    }

    #[test]
    fn a_flushed_proposal_does_not_get_a_fabricated_crossing_epoch() {
        // flush_merkle_root re-anchors with target = current + snapshot_extension,
        // omitting discussion_epochs. Inverting activate_voting's formula against
        // it produced a crossing epoch before the proposal existed: a recovery at
        // epoch 1015 sets start_epoch 1017, and 1017 - 1 - 7 - 1 = 1008 < 1011.
        let flushed = PhaseInputs {
            start_epoch: 1017,
            end_epoch: 1020,
            ..double_disinflation()
        };
        let t = PhaseTimeline::new(&flushed);
        assert_eq!(t.support, None);
        assert_eq!(t.discussion.0, None);
        // What is printed stays correct across the re-anchor.
        assert_eq!(t.snapshot, 1016);
        assert_eq!(t.voting, (1017, 1020));
        assert!(t.created <= t.snapshot);
    }

    #[test]
    fn changing_the_config_durations_cannot_corrupt_an_activated_timeline() {
        // update_config can change durations after activation; the stored epochs
        // were computed with the old ones, so the timeline must not depend on
        // whatever the config says now.
        let base = PhaseTimeline::new(&double_disinflation());
        let reconfigured = PhaseTimeline::new(&PhaseInputs {
            discussion_epochs: 2,
            snapshot_epoch_extension: 4,
            max_support_epochs: 1,
            ..double_disinflation()
        });
        assert_eq!(base, reconfigured);
    }

    #[test]
    fn the_timeline_agrees_with_the_phase_at_every_epoch() {
        // The bug this module exists to fix was a timeline that disagreed with
        // the reported status, so pin that they cannot drift apart.
        let p = double_disinflation();
        let t = PhaseTimeline::new(&p);
        for epoch in t.discussion.0.unwrap_or(t.created)..=t.discussion.1 {
            assert_eq!(
                ProposalPhase::new(&p, epoch),
                ProposalPhase::Discussion,
                "epoch {epoch} is inside the discussion window"
            );
        }
        assert_eq!(ProposalPhase::new(&p, t.snapshot), ProposalPhase::Snapshot);
        for epoch in t.voting.0..t.voting.1 {
            assert_eq!(ProposalPhase::new(&p, epoch), ProposalPhase::Voting);
        }
    }

    #[test]
    fn the_timeline_is_internally_ordered() {
        // The old output had voting (1021-1024) starting before the snapshot
        // (1026) and before discussion ended (1025).
        for p in [
            double_disinflation(),
            PhaseInputs {
                voting: false,
                start_epoch: 0,
                end_epoch: 0,
                ..double_disinflation()
            },
        ] {
            let t = PhaseTimeline::new(&p);
            if let Some((from, to)) = t.support {
                assert!(from <= to);
                assert!(to <= t.discussion.0.unwrap_or(to));
            }
            assert!(t.discussion.0.unwrap_or(t.discussion.1) <= t.discussion.1);
            assert!(t.discussion.1 < t.snapshot);
            assert!(t.snapshot < t.voting.0);
            assert!(t.voting.0 < t.voting.1);
        }
    }

    #[test]
    fn walks_the_whole_lifecycle() {
        let p = double_disinflation();
        assert_eq!(ProposalPhase::new(&p, 1011), ProposalPhase::Discussion);
        assert_eq!(ProposalPhase::new(&p, 1019), ProposalPhase::Discussion);
        assert_eq!(ProposalPhase::new(&p, 1020), ProposalPhase::Snapshot);
        assert_eq!(ProposalPhase::new(&p, 1021), ProposalPhase::Voting);
        assert_eq!(ProposalPhase::new(&p, 1023), ProposalPhase::Voting);
        assert!(!ProposalPhase::new(&p, 1023).vote_is_settled());
        assert_eq!(ProposalPhase::new(&p, 1024), ProposalPhase::Ended);
        assert!(ProposalPhase::new(&p, 1024).vote_is_settled());
    }

    #[test]
    fn an_unactivated_proposal_stays_in_support_until_its_window_closes() {
        let p = PhaseInputs {
            voting: false,
            start_epoch: 0,
            end_epoch: 0,
            ..double_disinflation()
        };
        assert_eq!(ProposalPhase::new(&p, 1011), ProposalPhase::Support);
        assert_eq!(ProposalPhase::new(&p, 1018), ProposalPhase::Support);
    }

    #[test]
    fn an_unactivated_proposal_past_its_window_has_failed() {
        // The old code reported "support" forever here. Support and retally
        // both reject past creation_epoch + max_support_epochs, so the proposal
        // can never advance.
        let p = PhaseInputs {
            voting: false,
            start_epoch: 0,
            end_epoch: 0,
            ..double_disinflation()
        };
        assert_eq!(ProposalPhase::new(&p, 1019), ProposalPhase::Failed);
        assert_eq!(p.epochs_remaining(1019), None);
    }

    #[test]
    fn an_unactivated_timeline_is_marked_projected() {
        let p = PhaseInputs {
            voting: false,
            start_epoch: 0,
            end_epoch: 0,
            ..double_disinflation()
        };
        let t = PhaseTimeline::new(&p);
        assert!(t.projected);
        // Latest possible crossing is the last epoch of the support window.
        assert_eq!(t.support, Some((1011, 1018)));
        assert_eq!(t.discussion, (Some(1018), 1025));
        assert_eq!(t.snapshot, 1026);
        assert_eq!(t.voting, (1027, 1030));
    }

    #[test]
    fn finalized_wins_over_everything() {
        let p = PhaseInputs {
            finalized: true,
            ..double_disinflation()
        };
        assert_eq!(ProposalPhase::new(&p, 1012), ProposalPhase::Finalized);
        assert!(ProposalPhase::new(&p, 1012).vote_is_settled());
        assert_eq!(p.epochs_remaining(1012), None);
    }

    #[test]
    fn a_zero_start_epoch_does_not_underflow() {
        let p = PhaseInputs {
            voting: true,
            start_epoch: 0,
            end_epoch: 0,
            creation_epoch: 0,
            ..double_disinflation()
        };
        // end_epoch 0 means current >= end, so this is Ended rather than a panic.
        assert_eq!(ProposalPhase::new(&p, 0), ProposalPhase::Ended);
        let _ = PhaseTimeline::new(&p);
    }
}
