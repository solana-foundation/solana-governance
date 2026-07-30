//! Vote tally integration suite: exercises `cast_vote` (validator),
//! `cast_vote_override` (delegator), and `finalize_proposal` end-to-end
//! against the real ncn-snapshot program (merkle proofs verified via CPI).
//!
//! Matrix covered (groups 1-3 each with major-yes / mixed / major-no
//! distributions):
//! 1. only validators vote
//! 2. validators and delegator overrides together — in BOTH orders
//!    (override after the validator voted, and override before), proving
//!    order-independence of the tally
//! 3. only delegator overrides, validators never vote — the regression case
//!    for the audit finding where cached override votes were dropped from
//!    the final tally when the validator never cast a vote
//! 4. overrides both before AND after the same validator's vote
//! 5. vote modifications: `modify_vote` on an overridden validator, and
//!    `modify_vote_override` both before and after the validator's vote
//! 6. both delegators override before the validator votes (multi-entry
//!    cache drain at `cast_vote`)
//! 7. `modify_vote_override` then finalize with no validator vote
//! 8. overrides alone can produce a for-majority that survives finalize
//!    (validators never cast a vote)
mod common;

use common::*;

/// Meta-leaf active stake per snapshot validator (1 SOL).
const V_ACTIVE: u64 = 1_000_000_000;
/// Stake-leaf stakes of the two delegators under each validator.
const D0: u64 = 300_000_000;
const D1: u64 = 200_000_000;
/// Validator self-stake remaining if both delegators override.
const V_REMAINDER: u64 = V_ACTIVE - D0 - D1;

/// Mirrors `calculate_vote_lamports!`: stake * bp / 10_000, floored.
fn lam(stake: u64, bp: u64) -> u64 {
    ((stake as u128) * (bp as u128) / 10_000) as u64
}

fn voting_fixture() -> (Harness, VotingScenario) {
    // 3 validators, all must support: threshold = 10_000 bps.
    let mut h = setup_harness(1, 3, 3);
    let s = setup_voting_scenario(
        &mut h,
        42,
        "Vote tally",
        &[
            (0, V_ACTIVE, &[D0, D1]),
            (1, V_ACTIVE, &[D0, D1]),
            (2, V_ACTIVE, &[D0, D1]),
        ],
    );
    (h, s)
}

fn assert_totals(state: &ProposalAccount, for_l: u64, against_l: u64, abstain_l: u64) {
    assert_eq!(
        (
            state.for_votes_lamports,
            state.against_votes_lamports,
            state.abstain_votes_lamports,
        ),
        (for_l, against_l, abstain_l),
        "proposal tallies mismatch"
    );
}

// ---------------------------------------------------------------------------
// 1. Only validators vote.
// ---------------------------------------------------------------------------

/// All three validators vote with the given distributions; totals must be the
/// stake-weighted sum and survive finalization.
fn only_validators_case(dists: [(u64, u64, u64); 3]) -> ProposalAccount {
    let (mut h, s) = voting_fixture();
    let mut expected = (0u64, 0u64, 0u64);
    for (voter, (f, a, ab)) in dists.iter().enumerate() {
        cast_validator_vote(&mut h, &s, voter, *f, *a, *ab);
        expected.0 += lam(V_ACTIVE, *f);
        expected.1 += lam(V_ACTIVE, *a);
        expected.2 += lam(V_ACTIVE, *ab);
    }
    let state = finalize_proposal(&mut h, &s);
    assert_totals(&state, expected.0, expected.1, expected.2);
    assert_eq!(state.vote_count, 3);
    state
}

#[test]
fn only_validators_major_yes() {
    let state = only_validators_case([(10_000, 0, 0), (9_000, 1_000, 0), (8_000, 0, 2_000)]);
    assert_totals(&state, 2_700_000_000, 100_000_000, 200_000_000);
    assert!(state.for_votes_lamports > state.against_votes_lamports + state.abstain_votes_lamports);
}

#[test]
fn only_validators_mixed() {
    let state = only_validators_case([
        (4_000, 4_000, 2_000),
        (3_000, 5_000, 2_000),
        (5_000, 3_000, 2_000),
    ]);
    assert_totals(&state, 1_200_000_000, 1_200_000_000, 600_000_000);
}

#[test]
fn only_validators_major_no() {
    let state = only_validators_case([(0, 10_000, 0), (1_000, 9_000, 0), (0, 8_000, 2_000)]);
    assert_totals(&state, 100_000_000, 2_700_000_000, 200_000_000);
    assert!(state.against_votes_lamports > state.for_votes_lamports + state.abstain_votes_lamports);
}

// ---------------------------------------------------------------------------
// 2. Validators and delegator overrides together, in both orders.
// ---------------------------------------------------------------------------

/// Validators vote `val` and delegator 0 of validators 0 and 1 overrides with
/// `ovr` — after the validator's vote for validator 0, before it for
/// validator 1. Validator 2 votes unopposed. Both overridden validators must
/// contribute identically regardless of order:
///   remainder (V_ACTIVE - D0) weighted by `val` + D0 weighted by `ovr`.
fn mixed_case(val: (u64, u64, u64), ovr: (u64, u64, u64)) -> ProposalAccount {
    let (mut h, s) = voting_fixture();

    // Validator 0 votes first, then its delegator overrides.
    cast_validator_vote(&mut h, &s, 0, val.0, val.1, val.2);
    cast_delegator_override(&mut h, &s, 0, 0, ovr.0, ovr.1, ovr.2);

    // Validator 1's delegator overrides first, then the validator votes.
    cast_delegator_override(&mut h, &s, 1, 0, ovr.0, ovr.1, ovr.2);
    cast_validator_vote(&mut h, &s, 1, val.0, val.1, val.2);

    // Validator 2 votes with no overrides.
    cast_validator_vote(&mut h, &s, 2, val.0, val.1, val.2);

    // Both orderings must leave the validator's Vote reduced by exactly D0.
    let remainder = V_ACTIVE - D0;
    for voter in [0, 1] {
        let vote = fetch_vote(&h, &s, voter).expect("validator vote must exist");
        assert_eq!(vote.override_lamports, D0, "voter {voter} override stake");
        assert_eq!(
            (
                vote.for_votes_lamports,
                vote.against_votes_lamports,
                vote.abstain_votes_lamports,
            ),
            (
                lam(remainder, val.0),
                lam(remainder, val.1),
                lam(remainder, val.2),
            ),
            "voter {voter} must vote with its remainder stake"
        );
    }
    // The pre-vote override went through the cache path.
    let cache = fetch_override_cache(&h, &s, 1).expect("cache for validator 1");
    assert_eq!(cache.total_stake, D0);

    let expected_for = 2 * (lam(remainder, val.0) + lam(D0, ovr.0)) + lam(V_ACTIVE, val.0);
    let expected_against = 2 * (lam(remainder, val.1) + lam(D0, ovr.1)) + lam(V_ACTIVE, val.1);
    let expected_abstain = 2 * (lam(remainder, val.2) + lam(D0, ovr.2)) + lam(V_ACTIVE, val.2);

    let state = finalize_proposal(&mut h, &s);
    assert_totals(&state, expected_for, expected_against, expected_abstain);
    // 3 validator votes + 2 overrides.
    assert_eq!(state.vote_count, 5);
    state
}

#[test]
fn mixed_votes_major_yes() {
    let state = mixed_case((10_000, 0, 0), (0, 10_000, 0));
    assert_totals(&state, 2_400_000_000, 600_000_000, 0);
    assert!(state.for_votes_lamports > state.against_votes_lamports + state.abstain_votes_lamports);
}

#[test]
fn mixed_votes_mixed() {
    let state = mixed_case((5_000, 3_000, 2_000), (2_000, 5_000, 3_000));
    assert_totals(&state, 1_320_000_000, 1_020_000_000, 660_000_000);
}

#[test]
fn mixed_votes_major_no() {
    let state = mixed_case((0, 10_000, 0), (10_000, 0, 0));
    assert_totals(&state, 600_000_000, 2_400_000_000, 0);
    assert!(state.against_votes_lamports > state.for_votes_lamports + state.abstain_votes_lamports);
}

// ---------------------------------------------------------------------------
// 3. Only delegator overrides; validators never vote.
//
// Regression for the audit finding: override votes cast while the validator
// had not voted used to be parked in VoteOverrideCache and dropped from the
// final tally when the validator never voted at all.
// ---------------------------------------------------------------------------

/// Every delegator (2 per validator, all 3 validators) overrides; no
/// validator ever casts a vote. `d0`/`d1` are the distributions used by the
/// first/second delegator of each validator.
fn only_overrides_case(d0: (u64, u64, u64), d1: (u64, u64, u64)) -> ProposalAccount {
    let (mut h, s) = voting_fixture();
    for voter in 0..3 {
        cast_delegator_override(&mut h, &s, voter, 0, d0.0, d0.1, d0.2);
        cast_delegator_override(&mut h, &s, voter, 1, d1.0, d1.1, d1.2);
    }

    for voter in 0..3 {
        assert!(
            fetch_vote(&h, &s, voter).is_none(),
            "validator {voter} must not have voted"
        );
        let cache = fetch_override_cache(&h, &s, voter).expect("override cache");
        assert_eq!(cache.total_stake, D0 + D1);
        assert_eq!(
            (
                cache.for_votes_lamports,
                cache.against_votes_lamports,
                cache.abstain_votes_lamports,
            ),
            (
                lam(D0, d0.0) + lam(D1, d1.0),
                lam(D0, d0.1) + lam(D1, d1.1),
                lam(D0, d0.2) + lam(D1, d1.2),
            ),
            "cache must mirror the pending override lamports"
        );
    }

    let per_validator = (
        lam(D0, d0.0) + lam(D1, d1.0),
        lam(D0, d0.1) + lam(D1, d1.1),
        lam(D0, d0.2) + lam(D1, d1.2),
    );
    let state = finalize_proposal(&mut h, &s);
    assert_totals(
        &state,
        3 * per_validator.0,
        3 * per_validator.1,
        3 * per_validator.2,
    );
    assert_eq!(state.vote_count, 6);
    // The validators' unused remainder stake must NOT appear anywhere.
    assert_eq!(
        state.for_votes_lamports + state.against_votes_lamports + state.abstain_votes_lamports,
        3 * (D0 + D1),
        "only overridden stake may be tallied"
    );
    state
}

#[test]
fn only_overrides_major_yes() {
    let state = only_overrides_case((10_000, 0, 0), (10_000, 0, 0));
    assert_totals(&state, 1_500_000_000, 0, 0);
}

/// Overrides alone can carry a for-majority: every delegator votes for, no
/// validator ever casts a `Vote`, and the finalized tallies still show
/// `for > against + abstain`.
///
/// Note: this is the voting-phase tally outcome only. The earlier support
/// phase that flips `proposal.voting` has no override path — only validators
/// can `support_proposal`.
#[test]
fn only_overrides_can_carry_for_majority() {
    let (mut h, s) = voting_fixture();

    // All six delegators override for; validators stay silent.
    for voter in 0..3 {
        cast_delegator_override(&mut h, &s, voter, 0, 10_000, 0, 0);
        cast_delegator_override(&mut h, &s, voter, 1, 10_000, 0, 0);
        assert!(
            fetch_vote(&h, &s, voter).is_none(),
            "validator {voter} must not have voted"
        );
    }

    let state = finalize_proposal(&mut h, &s);
    assert_totals(&state, 3 * (D0 + D1), 0, 0);
    assert_eq!(state.vote_count, 6);
    assert!(
        state.for_votes_lamports > state.against_votes_lamports + state.abstain_votes_lamports,
        "override-only for-votes must constitute a majority"
    );
    // Validator remainder stake never enters the tally.
    assert_eq!(
        state.for_votes_lamports + state.against_votes_lamports + state.abstain_votes_lamports,
        3 * (D0 + D1)
    );
}

#[test]
fn only_overrides_mixed() {
    let state = only_overrides_case((5_000, 5_000, 0), (0, 5_000, 5_000));
    assert_totals(&state, 450_000_000, 750_000_000, 300_000_000);
}

#[test]
fn only_overrides_major_no() {
    let state = only_overrides_case((0, 10_000, 0), (0, 10_000, 0));
    assert_totals(&state, 0, 1_500_000_000, 0);
}

// ---------------------------------------------------------------------------
// 4. Overrides both before AND after the same validator's vote.
//
// The only sequence exercising the cache drain (`override_lamports =
// cache.total_stake` at cast_vote) and the post-vote incremental
// `override_lamports += delegator_stake` on one validator, where a
// double-count would surface.
// ---------------------------------------------------------------------------

#[test]
fn override_before_and_after_validator_vote() {
    let (mut h, s) = voting_fixture();

    // d0 overrides against BEFORE the vote, the validator votes for, then d1
    // overrides abstain AFTER the vote.
    cast_delegator_override(&mut h, &s, 0, 0, 0, 10_000, 0);
    cast_validator_vote(&mut h, &s, 0, 10_000, 0, 0);
    cast_delegator_override(&mut h, &s, 0, 1, 0, 0, 10_000);

    let vote = fetch_vote(&h, &s, 0).expect("validator vote");
    assert_eq!(vote.override_lamports, D0 + D1);
    assert_eq!(
        (
            vote.for_votes_lamports,
            vote.against_votes_lamports,
            vote.abstain_votes_lamports,
        ),
        (V_REMAINDER, 0, 0),
        "validator must end up voting with only its non-overridden remainder"
    );

    let state = finalize_proposal(&mut h, &s);
    assert_totals(&state, V_REMAINDER, D0, D1);
    assert_eq!(state.vote_count, 3);
    // Exactly the validator's leaf stake is tallied — nothing dropped or
    // double-counted across the drain + incremental paths.
    assert_eq!(
        state.for_votes_lamports + state.against_votes_lamports + state.abstain_votes_lamports,
        V_ACTIVE
    );
}

// ---------------------------------------------------------------------------
// 5. Vote modifications, across the same ordering matrix.
// ---------------------------------------------------------------------------

/// `modify_vote` on a validator that has been overridden: the swap must
/// re-weight only the validator's remainder share, leaving the delegator's
/// lamports untouched.
#[test]
fn modify_vote_after_override() {
    let (mut h, s) = voting_fixture();

    cast_delegator_override(&mut h, &s, 0, 0, 0, 10_000, 0);
    cast_validator_vote(&mut h, &s, 0, 10_000, 0, 0);
    assert_totals(&fetch_proposal(&h.svm, &s.proposal), V_ACTIVE - D0, D0, 0);

    modify_validator_vote(&mut h, &s, 0, 0, 0, 10_000);

    let vote = fetch_vote(&h, &s, 0).expect("validator vote");
    assert_eq!(
        vote.override_lamports, D0,
        "override share must survive modify"
    );
    assert_eq!(
        (
            vote.for_votes_lamports,
            vote.against_votes_lamports,
            vote.abstain_votes_lamports,
        ),
        (0, 0, V_ACTIVE - D0),
    );

    let state = finalize_proposal(&mut h, &s);
    assert_totals(&state, 0, D0, V_ACTIVE - D0);
    // modify_* does not add a vote.
    assert_eq!(state.vote_count, 2);
}

/// `modify_vote_override` while the validator has NOT voted — the path
/// rewritten by the audit fix: the swap must land in the proposal totals
/// immediately and the cache must stay in sync, so the validator's later vote
/// neither drops nor double-counts anything.
#[test]
fn modify_override_before_validator_vote() {
    let (mut h, s) = voting_fixture();

    cast_delegator_override(&mut h, &s, 0, 0, 10_000, 0, 0);
    assert_totals(&fetch_proposal(&h.svm, &s.proposal), D0, 0, 0);

    modify_delegator_override(&mut h, &s, 0, 0, 0, 10_000, 0);
    assert_totals(&fetch_proposal(&h.svm, &s.proposal), 0, D0, 0);

    let cache = fetch_override_cache(&h, &s, 0).expect("override cache");
    assert_eq!(cache.total_stake, D0);
    assert_eq!(
        (
            cache.for_votes_lamports,
            cache.against_votes_lamports,
            cache.abstain_votes_lamports,
        ),
        (0, D0, 0),
        "cache must mirror the modified pending override"
    );

    cast_validator_vote(&mut h, &s, 0, 10_000, 0, 0);

    let state = finalize_proposal(&mut h, &s);
    assert_totals(&state, V_ACTIVE - D0, D0, 0);
    assert_eq!(state.vote_count, 2);
    assert_eq!(
        state.for_votes_lamports + state.against_votes_lamports + state.abstain_votes_lamports,
        V_ACTIVE
    );
}

/// `modify_vote_override` after the validator voted: a plain swap of the
/// delegator's lamports in the proposal totals.
#[test]
fn modify_override_after_validator_vote() {
    let (mut h, s) = voting_fixture();

    cast_validator_vote(&mut h, &s, 0, 10_000, 0, 0);
    cast_delegator_override(&mut h, &s, 0, 0, 0, 10_000, 0);
    assert_totals(&fetch_proposal(&h.svm, &s.proposal), V_ACTIVE - D0, D0, 0);

    modify_delegator_override(&mut h, &s, 0, 0, 0, 0, 10_000);

    let state = finalize_proposal(&mut h, &s);
    assert_totals(&state, V_ACTIVE - D0, 0, D0);
    assert_eq!(state.vote_count, 2);
}

// ---------------------------------------------------------------------------
// 6. Both delegators override before the validator votes.
//
// Drains a multi-entry cache at `cast_vote` (`total_stake = D0 + D1`). Group 3
// never has the validator vote after a multi-delegator cache; group 4 only
// drains a single pre-vote override.
// ---------------------------------------------------------------------------

#[test]
fn both_overrides_before_validator_vote() {
    let (mut h, s) = voting_fixture();

    // d0 against, d1 abstain — both before the validator votes for.
    cast_delegator_override(&mut h, &s, 0, 0, 0, 10_000, 0);
    cast_delegator_override(&mut h, &s, 0, 1, 0, 0, 10_000);

    let cache = fetch_override_cache(&h, &s, 0).expect("override cache");
    assert_eq!(cache.total_stake, D0 + D1);
    assert_eq!(
        (
            cache.for_votes_lamports,
            cache.against_votes_lamports,
            cache.abstain_votes_lamports,
        ),
        (0, D0, D1),
    );
    assert_totals(&fetch_proposal(&h.svm, &s.proposal), 0, D0, D1);

    cast_validator_vote(&mut h, &s, 0, 10_000, 0, 0);

    let vote = fetch_vote(&h, &s, 0).expect("validator vote");
    assert_eq!(vote.override_lamports, D0 + D1);
    assert_eq!(
        (
            vote.for_votes_lamports,
            vote.against_votes_lamports,
            vote.abstain_votes_lamports,
        ),
        (V_REMAINDER, 0, 0),
        "validator must vote with only non-overridden remainder"
    );

    let state = finalize_proposal(&mut h, &s);
    assert_totals(&state, V_REMAINDER, D0, D1);
    assert_eq!(state.vote_count, 3);
    assert_eq!(
        state.for_votes_lamports + state.against_votes_lamports + state.abstain_votes_lamports,
        V_ACTIVE,
        "multi-entry cache drain must neither drop nor double-count"
    );
}

// ---------------------------------------------------------------------------
// 7. Modify override, then finalize with no validator vote.
//
// Complements `modify_override_before_validator_vote`, which always has the
// validator vote after the modification. The rewrite must keep the modified
// override in the proposal totals even when that later vote never arrives.
// ---------------------------------------------------------------------------

#[test]
fn modify_override_then_finalize_without_validator_vote() {
    let (mut h, s) = voting_fixture();

    cast_delegator_override(&mut h, &s, 0, 0, 10_000, 0, 0);
    assert_totals(&fetch_proposal(&h.svm, &s.proposal), D0, 0, 0);

    modify_delegator_override(&mut h, &s, 0, 0, 0, 10_000, 0);
    assert_totals(&fetch_proposal(&h.svm, &s.proposal), 0, D0, 0);

    let cache = fetch_override_cache(&h, &s, 0).expect("override cache");
    assert_eq!(cache.total_stake, D0);
    assert_eq!(
        (
            cache.for_votes_lamports,
            cache.against_votes_lamports,
            cache.abstain_votes_lamports,
        ),
        (0, D0, 0),
        "cache must mirror the modified pending override"
    );
    assert!(
        fetch_vote(&h, &s, 0).is_none(),
        "validator must not have voted"
    );

    let state = finalize_proposal(&mut h, &s);
    assert_totals(&state, 0, D0, 0);
    assert_eq!(state.vote_count, 1);
    assert_eq!(
        state.for_votes_lamports + state.against_votes_lamports + state.abstain_votes_lamports,
        D0,
        "modified override must survive finalization without a validator vote"
    );
}
