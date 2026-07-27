//! Support threshold suite at 1500 supporters out of 2000 validators (75%
//! threshold) — the same scenario as `support_150_of_1000.rs` but near the
//! `MAX_SUPPORTERS_LIMIT` scale, exercising the zero-copy supporter storage
//! and per-support full re-tally with a four-digit list.

mod common;

use {common::*, solana_signer::Signer};

const VALIDATOR_COUNT: usize = 2_000;
const SUPPORTER_COUNT: usize = 1_500; // 1500/2000 = 75% => 7_500 bps threshold

/// 75% of the cluster shows support in a single epoch; voting activates on
/// the 1500th support, with every prior supporter re-tallied each time.
#[test_log::test]
fn support_proposal_reaches_threshold_at_1500_of_2000() {
    const CREATION_EPOCH: u64 = 10;
    let mut h = setup_harness(CREATION_EPOCH, VALIDATOR_COUNT, SUPPORTER_COUNT);

    // Same-epoch activation: ballot box for crossing_epoch == creation_epoch.
    let ballot_box = seed_ballot_box(&mut h.svm, expected_snapshot_slot(CREATION_EPOCH));
    let proposal = create_proposal(&mut h, 42, "same-epoch support at cap scale");

    for i in 0..SUPPORTER_COUNT {
        support_one(&mut h, proposal, i, ballot_box);
        if (i + 1) % 250 == 0 {
            println!("supported {}/{} (same epoch)", i + 1, SUPPORTER_COUNT);
        }
    }

    let state = fetch_proposal(&h.svm, &proposal);
    assert_eq!(state.creation_epoch, CREATION_EPOCH);
    assert_threshold_reached(&h, &state, CREATION_EPOCH);
    // Entry-level check: all 1500 supporter keys intact, in insertion order.
    let expected: Vec<[u8; 32]> = h.validators[..SUPPORTER_COUNT]
        .iter()
        .map(|v| pk_bytes(&v.vote.pubkey()))
        .collect();
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
