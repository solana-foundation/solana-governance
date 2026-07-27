//! Support/retally integration suite at 150 supporters out of 1000
//! validators (15% threshold). Harness lives in `common`; see
//! `support_1500_of_2000.rs` for the same scenario at the supporter cap.

mod common;

use {
    common::*, solana_signer::Signer, solana_transaction_error::TransactionError,
    svmgov_program::GovernanceError,
};

const VALIDATOR_COUNT: usize = 1_000;
const SUPPORTER_COUNT: usize = 150; // 150/1000 = 15% => 1_500 bps threshold

fn setup(creation_epoch: u64) -> Harness {
    setup_harness(creation_epoch, VALIDATOR_COUNT, SUPPORTER_COUNT)
}

/// Stake drift alone (no new supporter) can activate voting via `retally_support`.
///
/// 149 supporters leave the proposal under the 15% threshold; after one of those
/// supporters' epoch stake grows, a permissionless retally re-measures and flips
/// `voting` without another `support_proposal`.
#[test_log::test]
fn retally_support_activates_voting_after_stake_drift() {
    const CREATION_EPOCH: u64 = 10;
    const UNDER_THRESHOLD: usize = SUPPORTER_COUNT - 1; // 149 / 1000 = 14.9%
    let mut h = setup(CREATION_EPOCH);

    // Placeholder ballot box for the under-threshold supports (activation skipped).
    let early_ballot = seed_ballot_box(&mut h.svm, expected_snapshot_slot(CREATION_EPOCH));
    let proposal = create_proposal(&mut h, 99, "retally after stake drift");

    for i in 0..UNDER_THRESHOLD {
        support_one(&mut h, proposal, i, early_ballot);
    }

    let before = fetch_proposal(&h.svm, &proposal);
    assert_eq!(before.supporters.len(), UNDER_THRESHOLD);
    assert_eq!(
        before.cluster_support_lamports,
        STAKE_PER_VALIDATOR * UNDER_THRESHOLD as u64
    );
    assert!(!before.voting);

    // Drift: +1 SOL on an existing supporter, −1 SOL on a non-supporter so the
    // cluster total stays 1000 SOL (threshold remains exactly 150 SOL). Bumping
    // without a matching decrease would lift the lamport threshold above 150 SOL
    // (1001e9 * 1500 / 10_000 = 150.15 SOL) and leave 150 SOL still short.
    let boosted = h.validators[0].vote.pubkey();
    let non_supporter = h.validators[VALIDATOR_COUNT - 1].vote.pubkey();
    h.svm
        .set_epoch_stake(boosted, 2 * STAKE_PER_VALIDATOR)
        .unwrap();
    h.svm.set_epoch_stake(non_supporter, 0).unwrap();
    assert_eq!(
        h.svm.epoch_total_stake(),
        STAKE_PER_VALIDATOR * VALIDATOR_COUNT as u64
    );

    // Retally in a later epoch inside the window (re-measure at current stake).
    let crossing_epoch = CREATION_EPOCH + 1;
    set_clock(&mut h.svm, crossing_epoch);
    let ballot_box = seed_ballot_box(&mut h.svm, expected_snapshot_slot(crossing_epoch));
    // Permissionless: any signer may call (use a non-supporter identity).
    retally_one(&mut h, proposal, UNDER_THRESHOLD, ballot_box);

    let after = fetch_proposal(&h.svm, &proposal);
    assert_eq!(after.supporters.len(), UNDER_THRESHOLD);
    assert_eq!(
        after.cluster_support_lamports,
        STAKE_PER_VALIDATOR * UNDER_THRESHOLD as u64 + STAKE_PER_VALIDATOR
    );
    assert!(
        after.voting,
        "retally should activate voting after stake drift"
    );
    assert_eq!(after.snapshot_slot, expected_snapshot_slot(crossing_epoch));
    assert_eq!(after.start_epoch, crossing_epoch + DISCUSSION_EPOCHS + 1);
}

/// `support_proposal` re-measures prior supporters at the current epoch before
/// adding the newcomer — so stake drift on an earlier supporter can be what
/// pushes the tally over the threshold (without `retally_support`).
#[test_log::test]
fn support_proposal_remeasures_prior_stake_across_epochs() {
    const CREATION_EPOCH: u64 = 10;
    // 148 @ 1 SOL; after +1 SOL drift on one prior, a 149th support remeasures
    // to 149 + 1 = 150. Stale (support-time) weights would only reach 149.
    const PRIOR: usize = SUPPORTER_COUNT - 2;
    let mut h = setup(CREATION_EPOCH);
    let early_ballot = seed_ballot_box(&mut h.svm, expected_snapshot_slot(CREATION_EPOCH));
    let proposal = create_proposal(&mut h, 98, "remeasure on support");

    for i in 0..PRIOR {
        support_one(&mut h, proposal, i, early_ballot);
    }
    assert!(!fetch_proposal(&h.svm, &proposal).voting);

    h.svm
        .set_epoch_stake(h.validators[0].vote.pubkey(), 2 * STAKE_PER_VALIDATOR)
        .unwrap();
    h.svm
        .set_epoch_stake(h.validators[VALIDATOR_COUNT - 1].vote.pubkey(), 0)
        .unwrap();

    let crossing_epoch = CREATION_EPOCH + 1;
    set_clock(&mut h.svm, crossing_epoch);
    let ballot_box = seed_ballot_box(&mut h.svm, expected_snapshot_slot(crossing_epoch));
    support_one(&mut h, proposal, PRIOR, ballot_box);

    let after = fetch_proposal(&h.svm, &proposal);
    assert_eq!(after.supporters.len(), PRIOR + 1);
    assert_eq!(
        after.cluster_support_lamports,
        STAKE_PER_VALIDATOR * (PRIOR as u64 + 2) // 148 + boost + newcomer
    );
    assert!(after.voting);
    assert_eq!(after.snapshot_slot, expected_snapshot_slot(crossing_epoch));
}

/// Once voting is active, both `support_proposal` and `retally_support` return
/// `ProposalClosed`.
#[test_log::test]
fn support_and_retally_reject_after_voting_activated() {
    const CREATION_EPOCH: u64 = 10;
    const UNDER_THRESHOLD: usize = SUPPORTER_COUNT - 1;
    let mut h = setup(CREATION_EPOCH);
    let ballot_box = seed_ballot_box(&mut h.svm, expected_snapshot_slot(CREATION_EPOCH));
    let proposal = create_proposal(&mut h, 97, "closed after voting");

    for i in 0..UNDER_THRESHOLD {
        support_one(&mut h, proposal, i, ballot_box);
    }
    // Cross threshold via stake drift + retally (leaves validator UNDER_THRESHOLD free).
    h.svm
        .set_epoch_stake(h.validators[0].vote.pubkey(), 2 * STAKE_PER_VALIDATOR)
        .unwrap();
    h.svm
        .set_epoch_stake(h.validators[VALIDATOR_COUNT - 1].vote.pubkey(), 0)
        .unwrap();
    retally_one(&mut h, proposal, UNDER_THRESHOLD, ballot_box);
    assert!(fetch_proposal(&h.svm, &proposal).voting);

    let closed = TransactionError::InstructionError(
        1,
        anchor_custom_error(GovernanceError::ProposalClosed),
    );
    let support_err = try_support_one(&mut h, proposal, UNDER_THRESHOLD, ballot_box)
        .expect_err("support after voting should fail");
    assert_eq!(support_err.err, closed);

    // Different caller than the activating retally (avoid LiteSVM AlreadyProcessed).
    let retally_err = try_retally_one(&mut h, proposal, 1, ballot_box)
        .expect_err("retally after voting should fail");
    assert_eq!(retally_err.err, closed);
}

/// Retally is rejected once the clock moves past the support window
/// (same inclusive bound as `support_proposal`).
#[test_log::test]
fn retally_support_rejects_after_support_window() {
    const CREATION_EPOCH: u64 = 10;
    let mut h = setup(CREATION_EPOCH);
    let ballot_box = seed_ballot_box(&mut h.svm, expected_snapshot_slot(CREATION_EPOCH));
    let proposal = create_proposal(&mut h, 8, "expired retally window");

    // Need ≥1 supporter so the failure is the window check, not NoSupporters.
    support_one(&mut h, proposal, 0, ballot_box);

    // Last inclusive epoch still allows retally.
    set_clock(&mut h.svm, CREATION_EPOCH + MAX_SUPPORT_EPOCHS);
    retally_one(&mut h, proposal, 1, ballot_box);
    assert!(!fetch_proposal(&h.svm, &proposal).voting);

    // One epoch past the window end → SupportPeriodExpired.
    // Different caller so LiteSVM does not treat this as a duplicate tx.
    set_clock(&mut h.svm, CREATION_EPOCH + MAX_SUPPORT_EPOCHS + 1);
    let err = try_retally_one(&mut h, proposal, 2, ballot_box)
        .expect_err("retally after window end should fail");
    assert_eq!(
        err.err,
        TransactionError::InstructionError(
            1,
            anchor_custom_error(GovernanceError::SupportPeriodExpired),
        )
    );
    assert!(!fetch_proposal(&h.svm, &proposal).voting);
}

/// Support is rejected once the clock moves past
/// `creation_epoch + max_support_epochs` (inclusive window end).
#[test_log::test]
fn support_proposal_rejects_after_support_window() {
    const CREATION_EPOCH: u64 = 10;
    let mut h = setup(CREATION_EPOCH);
    // Ballot box is unused on the failing path (activation never runs), but the
    // account meta is still required by the instruction.
    let ballot_box = seed_ballot_box(&mut h.svm, expected_snapshot_slot(CREATION_EPOCH));
    let proposal = create_proposal(&mut h, 7, "expired support window");

    // Last inclusive epoch of the window must still accept support.
    set_clock(&mut h.svm, CREATION_EPOCH + MAX_SUPPORT_EPOCHS);
    support_one(&mut h, proposal, 0, ballot_box);
    let mid = fetch_proposal(&h.svm, &proposal);
    assert_eq!(mid.supporters.len(), 1);
    assert!(!mid.voting);

    // One epoch past the window end → SupportPeriodExpired.
    // Ix index 1: compute-budget CU limit is instruction 0.
    set_clock(&mut h.svm, CREATION_EPOCH + MAX_SUPPORT_EPOCHS + 1);
    let err = try_support_one(&mut h, proposal, 1, ballot_box)
        .expect_err("support after window end should fail");
    assert_eq!(
        err.err,
        TransactionError::InstructionError(
            1,
            anchor_custom_error(GovernanceError::SupportPeriodExpired),
        )
    );

    let state = fetch_proposal(&h.svm, &proposal);
    assert_eq!(state.supporters.len(), 1, "failed support must not append");
    assert!(!state.voting);
}

/// This test has 15% show support in a single epoch
#[test_log::test]
fn support_proposal_reaches_threshold_at_150_of_1000() {
    const CREATION_EPOCH: u64 = 10;
    let mut h = setup(CREATION_EPOCH);

    // Same-epoch activation: ballot box for crossing_epoch == creation_epoch.
    let ballot_box = seed_ballot_box(&mut h.svm, expected_snapshot_slot(CREATION_EPOCH));
    let proposal = create_proposal(&mut h, 42, "same-epoch support");

    for i in 0..SUPPORTER_COUNT {
        support_one(&mut h, proposal, i, ballot_box);
        if (i + 1) % 50 == 0 {
            println!("supported {}/{} (same epoch)", i + 1, SUPPORTER_COUNT);
        }
    }

    let state = fetch_proposal(&h.svm, &proposal);
    assert_eq!(state.creation_epoch, CREATION_EPOCH);
    assert_threshold_reached(&h, &state, CREATION_EPOCH);
    // Entry-level check: the raw supporter keys written past the Borsh
    // capacity boundary must be intact after voting activation.
    let expected: Vec<[u8; 32]> = h.validators[..SUPPORTER_COUNT]
        .iter()
        .map(|v| pk_bytes(&v.vote.pubkey()))
        .collect();
    assert_eq!(state.supporters.len(), expected.len());
    assert_eq!(
        state.supporters, expected,
        "supporter entries must be intact, in insertion order"
    );
    println!(
        "ok same-epoch: {}/{} supported; voting={}",
        state.supporters.len(),
        VALIDATOR_COUNT,
        state.voting
    );
}

/// For each support span of 2..=10 epochs, create a fresh proposal and spread
/// 150 supports across that span so the threshold is crossed only on the last
/// epoch (re-tallying prior supporters each time under the current clock).
#[test_log::test]
fn support_proposal_reaches_threshold_across_2_to_10_epochs() {
    const CREATION_EPOCH: u64 = 100;

    for span in 2u64..=10 {
        let mut h = setup(CREATION_EPOCH);
        let crossing_epoch = CREATION_EPOCH + span - 1;
        let ballot_box = seed_ballot_box(&mut h.svm, expected_snapshot_slot(crossing_epoch));
        let proposal = create_proposal(&mut h, 42, &format!("{span}-epoch support"));

        let mut next = 0usize;
        for offset in 0..span {
            let epoch = CREATION_EPOCH + offset;
            set_clock(&mut h.svm, epoch);

            let remaining_epochs = (span - offset) as usize;
            let remaining_supporters = SUPPORTER_COUNT - next;
            // Evenly drain the remaining quota; last epoch takes the rest so
            // the 150th (threshold) support lands on crossing_epoch.
            let batch = if offset + 1 == span {
                remaining_supporters
            } else {
                remaining_supporters / remaining_epochs
            };

            for idx in next..next + batch {
                support_one(&mut h, proposal, idx, ballot_box);
            }
            next += batch;

            let mid = fetch_proposal(&h.svm, &proposal);
            if offset + 1 < span {
                assert!(
                    !mid.voting,
                    "span={span}: voting must not activate before final epoch (epoch {epoch}, supporters={})",
                    mid.supporters.len()
                );
            }
        }
        assert_eq!(next, SUPPORTER_COUNT);

        let state = fetch_proposal(&h.svm, &proposal);
        assert_eq!(state.creation_epoch, CREATION_EPOCH);
        assert_threshold_reached(&h, &state, crossing_epoch);
        // Schedule anchors on the crossing epoch (discussion_epochs=1).
        assert_eq!(state.start_epoch, crossing_epoch + DISCUSSION_EPOCHS + 1);
        assert_eq!(state.end_epoch, state.start_epoch + VOTING_EPOCHS);

        println!(
            "ok span={span}: crossed at epoch {crossing_epoch}; supporters={}; snapshot_slot={}",
            state.supporters.len(),
            state.snapshot_slot
        );
    }
}
