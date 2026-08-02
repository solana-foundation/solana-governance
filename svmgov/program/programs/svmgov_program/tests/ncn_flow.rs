//! LiteSVM port of the old `ncn/tests` anchor-client integration suite
//! (`test_full_program_flow.rs`), covering the full ncn-snapshot program
//! flow: program-config lifecycle, balloting to consensus, tie-breaker
//! resolution, ballot-box reset, and merkle-proof account lifecycle.
//!
//! Improvements over the original:
//! - Runs against the PRODUCTION program build. The old suite needed a
//!   `skip-pda-check` build because it could not sign as a svmgov Proposal
//!   PDA; here the ballot box is created through the real svmgov
//!   `support_proposal` CPI, so the PDA gate itself is exercised.
//! - Covers the `SnapshotSlotNotReached` gate added in PR #109 (which the
//!   original suite was broken by: it voted on a future snapshot slot).
//! - No live validator, no fixture zips: snapshots are fabricated with the
//!   production `ncn-merkle-tree` builder (fixture parsing is covered by the
//!   verifier-service tests).
//!
//! Every transaction is prefixed with a nonce-varied compute-budget
//! instruction so repeated identical instructions produce distinct
//! signatures (LiteSVM otherwise rejects them as already processed); program
//! errors therefore surface at instruction index 1.

mod common;

use {
    common::*,
    ncn_snapshot::{
        error::ErrorCode as NcnError, Ballot, BallotBox, BallotTally, ConsensusResult,
        MetaMerkleLeaf, MetaMerkleProof, OperatorVote, ProgramConfig, StakeMerkleLeaf,
        MAX_BALLOT_TALLIES,
    },
    solana_address::Address,
    solana_clock::Clock,
    solana_compute_budget_interface::ComputeBudgetInstruction,
    solana_instruction::Instruction,
    solana_instruction_error::InstructionError,
    solana_keypair::Keypair,
    solana_native_token::LAMPORTS_PER_SOL,
    solana_signer::Signer,
    solana_transaction_error::TransactionError,
};

const CREATION_EPOCH: u64 = 1;
const THRESHOLD_BPS: u16 = 6666;

fn ballot(root: u8, hash: u8) -> Ballot {
    Ballot {
        meta_merkle_root: [root; 32],
        snapshot_hash: [hash; 32],
    }
}

fn anchor_framework_error(code: anchor_lang::error::ErrorCode) -> InstructionError {
    InstructionError::Custom(code as u32)
}

/// Sends `ixs` behind a nonce-varied compute-budget instruction (see module
/// doc); program errors land at instruction index 1.
fn try_send(
    h: &mut Harness,
    payer: &Keypair,
    nonce: &mut u32,
    ixs: &[Instruction],
) -> Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata> {
    *nonce += 1;
    let mut all = vec![ComputeBudgetInstruction::set_compute_unit_limit(
        800_000 + *nonce,
    )];
    all.extend_from_slice(ixs);
    try_send_ix(&mut h.svm, payer, &all)
}

fn send(h: &mut Harness, payer: &Keypair, nonce: &mut u32, ixs: &[Instruction]) {
    try_send(h, payer, nonce, ixs).unwrap_or_else(|e| {
        panic!("tx failed: {:#?}\nlogs: {:#?}", e.err, e.meta.logs);
    });
}

fn assert_ncn_err(
    result: Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata>,
    expected: NcnError,
) {
    let err = result.expect_err("transaction must fail");
    assert_eq!(
        err.err,
        TransactionError::InstructionError(1, ncn_custom_error(expected)),
        "logs: {:#?}",
        err.meta.logs
    );
}

fn assert_anchor_err(
    result: Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata>,
    expected: anchor_lang::error::ErrorCode,
) {
    let err = result.expect_err("transaction must fail");
    assert_eq!(
        err.err,
        TransactionError::InstructionError(1, anchor_framework_error(expected)),
        "logs: {:#?}",
        err.meta.logs
    );
}

/// A ballot box created through the real svmgov `support_proposal` CPI,
/// governed by a real `ProgramConfig`.
struct FlowCtx {
    admin: Keypair,
    operators: Vec<Keypair>,
    ballot_box: Address,
    consensus_result: Address,
    snapshot_slot: u64,
    /// Clock values at ballot-box creation (activation of the proposal).
    activation_slot: u64,
    activation_timestamp: i64,
    nonce: u32,
}

/// Initializes a real ncn `ProgramConfig` (authority + tie-breaker = `admin`,
/// whitelist = `operator_count` fresh keypairs, svmgov as the authorized
/// governance program), then drives a proposal through support so the
/// activation CPI creates the ballot box — the production `init_ballot_box`
/// path, PDA gate included.
fn setup_flow(operator_count: usize, vote_duration: i64) -> (Harness, FlowCtx) {
    let mut h = setup_harness(CREATION_EPOCH, 3, 3);
    let mut nonce = 0u32;

    // Replace the harness placeholder config so the real init can run.
    clear_account(&mut h.svm, ncn_program_config_pda());

    let admin = Keypair::new();
    h.svm
        .airdrop(&admin.pubkey(), 10 * LAMPORTS_PER_SOL)
        .unwrap();
    let operators: Vec<Keypair> = (0..operator_count).map(|_| Keypair::new()).collect();
    for op in &operators {
        h.svm.airdrop(&op.pubkey(), LAMPORTS_PER_SOL).unwrap();
    }
    let operator_keys: Vec<Address> = operators.iter().map(|k| k.pubkey()).collect();

    send(
        &mut h,
        &admin,
        &mut nonce,
        &[init_ncn_program_config_ix(
            &admin.pubkey(),
            &admin.pubkey(),
            &SVMGOV_PROGRAM_ID,
        )],
    );
    send(
        &mut h,
        &admin,
        &mut nonce,
        &[update_operator_whitelist_ix(
            &admin.pubkey(),
            Some(&operator_keys),
            None,
        )],
    );
    send(
        &mut h,
        &admin,
        &mut nonce,
        &[update_ncn_program_config_ix(
            &admin.pubkey(),
            None,
            Some(THRESHOLD_BPS),
            Some(&admin.pubkey()),
            Some(vote_duration),
            None,
        )],
    );

    // Create + fully support a proposal; the activating support CPIs
    // init_ballot_box with the Proposal PDA as signer.
    let proposal = create_proposal(&mut h, 7, "ncn flow");
    let snapshot_slot = expected_snapshot_slot(CREATION_EPOCH);
    let ballot_box = ballot_box_pda(snapshot_slot);
    let clock = h.svm.get_sysvar::<Clock>();
    for i in 0..h.supporter_count {
        support_one(&mut h, proposal, i, ballot_box);
    }
    assert!(
        fetch_proposal(&h.svm, &proposal).voting,
        "proposal must activate"
    );

    let ctx = FlowCtx {
        admin,
        operators,
        ballot_box,
        consensus_result: consensus_result_pda(snapshot_slot),
        snapshot_slot,
        activation_slot: clock.slot,
        activation_timestamp: clock.unix_timestamp,
        nonce,
    };
    (h, ctx)
}

/// Moves the clock past the snapshot slot so `cast_vote` opens up.
fn advance_past_snapshot_slot(h: &mut Harness) {
    set_clock(&mut h.svm, CREATION_EPOCH + 1);
}

fn fetch_ballot_box(h: &Harness, ctx: &FlowCtx) -> BallotBox {
    fetch_ncn_account(&h.svm, &ctx.ballot_box)
}

fn cast(h: &mut Harness, ctx: &mut FlowCtx, operator: usize, b: &Ballot) {
    let op = ctx.operators[operator].insecure_clone();
    let ix = ncn_cast_vote_ix(&op.pubkey(), ctx.ballot_box, b);
    send(h, &op, &mut ctx.nonce, &[ix]);
}

fn try_cast(
    h: &mut Harness,
    ctx: &mut FlowCtx,
    operator: usize,
    b: &Ballot,
) -> Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata> {
    let op = ctx.operators[operator].insecure_clone();
    let ix = ncn_cast_vote_ix(&op.pubkey(), ctx.ballot_box, b);
    try_send(h, &op, &mut ctx.nonce, &[ix])
}

// ---------------------------------------------------------------------------
// 1. ProgramConfig lifecycle (port of test_program_config).
// ---------------------------------------------------------------------------

#[test]
fn program_config_lifecycle() {
    let mut h = setup_harness(CREATION_EPOCH, 1, 1);
    let mut nonce = 0u32;
    clear_account(&mut h.svm, ncn_program_config_pda());

    let admin = Keypair::new();
    h.svm
        .airdrop(&admin.pubkey(), 10 * LAMPORTS_PER_SOL)
        .unwrap();
    let svmgov_program_id = Address::new_unique();

    send(
        &mut h,
        &admin,
        &mut nonce,
        &[init_ncn_program_config_ix(
            &admin.pubkey(),
            &admin.pubkey(),
            &svmgov_program_id,
        )],
    );

    let config: ProgramConfig = fetch_ncn_account(&h.svm, &ncn_program_config_pda());
    assert_eq!(config.authority, to_pubkey(&admin.pubkey()));
    assert_eq!(config.proposed_authority, None);
    assert_eq!(
        config.tie_breaker_admin,
        anchor_lang::prelude::Pubkey::default()
    );
    assert!(config.whitelisted_operators.is_empty());
    assert_eq!(config.min_consensus_threshold_bps, 0);
    assert_eq!(config.vote_duration, 0);
    assert_eq!(config.svmgov_program_pubkey, to_pubkey(&svmgov_program_id));

    // Add 10 operators.
    let mut operators_to_add: Vec<Address> = (0..10).map(|_| Address::new_unique()).collect();
    send(
        &mut h,
        &admin,
        &mut nonce,
        &[update_operator_whitelist_ix(
            &admin.pubkey(),
            Some(&operators_to_add),
            None,
        )],
    );
    let config: ProgramConfig = fetch_ncn_account(&h.svm, &ncn_program_config_pda());
    let expected: Vec<_> = operators_to_add.iter().map(to_pubkey).collect();
    assert_eq!(config.whitelisted_operators, expected);

    // Adding the same new operator twice registers it only once.
    let new_operator = Address::new_unique();
    operators_to_add.push(new_operator);
    operators_to_add.push(new_operator);
    send(
        &mut h,
        &admin,
        &mut nonce,
        &[update_operator_whitelist_ix(
            &admin.pubkey(),
            Some(&operators_to_add),
            None,
        )],
    );
    let config: ProgramConfig = fetch_ncn_account(&h.svm, &ncn_program_config_pda());
    assert_eq!(
        config.whitelisted_operators.len(),
        operators_to_add.len() - 1
    );
    assert!(config
        .whitelisted_operators
        .contains(&to_pubkey(&new_operator)));

    // Remove everything past the first 8.
    send(
        &mut h,
        &admin,
        &mut nonce,
        &[update_operator_whitelist_ix(
            &admin.pubkey(),
            None,
            Some(&operators_to_add[8..]),
        )],
    );
    let config: ProgramConfig = fetch_ncn_account(&h.svm, &ncn_program_config_pda());
    let expected: Vec<_> = operators_to_add[..8].iter().map(to_pubkey).collect();
    assert_eq!(config.whitelisted_operators, expected);

    // Overlapping add/remove lists are rejected.
    let overlap = vec![Address::new_unique()];
    assert_ncn_err(
        try_send(
            &mut h,
            &admin,
            &mut nonce,
            &[update_operator_whitelist_ix(
                &admin.pubkey(),
                Some(&overlap),
                Some(&overlap),
            )],
        ),
        NcnError::OverlappingWhitelistEntries,
    );

    // Only the authority may touch the whitelist.
    let rando = Keypair::new();
    h.svm.airdrop(&rando.pubkey(), LAMPORTS_PER_SOL).unwrap();
    assert_anchor_err(
        try_send(
            &mut h,
            &rando,
            &mut nonce,
            &[update_operator_whitelist_ix(
                &rando.pubkey(),
                Some(&overlap),
                None,
            )],
        ),
        anchor_lang::error::ErrorCode::ConstraintHasOne,
    );

    // Full config update: propose a new authority, set thresholds.
    let new_authority = Keypair::new();
    h.svm
        .airdrop(&new_authority.pubkey(), LAMPORTS_PER_SOL)
        .unwrap();
    let new_svmgov_program_id = Address::new_unique();
    send(
        &mut h,
        &admin,
        &mut nonce,
        &[update_ncn_program_config_ix(
            &admin.pubkey(),
            Some(&new_authority.pubkey()),
            Some(THRESHOLD_BPS),
            Some(&admin.pubkey()),
            Some(20),
            Some(&new_svmgov_program_id),
        )],
    );
    let config: ProgramConfig = fetch_ncn_account(&h.svm, &ncn_program_config_pda());
    assert_eq!(config.authority, to_pubkey(&admin.pubkey()));
    assert_eq!(
        config.proposed_authority,
        Some(to_pubkey(&new_authority.pubkey()))
    );
    assert_eq!(config.tie_breaker_admin, to_pubkey(&admin.pubkey()));
    assert_eq!(config.min_consensus_threshold_bps, THRESHOLD_BPS);
    assert_eq!(config.vote_duration, 20);
    assert_eq!(
        config.svmgov_program_pubkey,
        to_pubkey(&new_svmgov_program_id)
    );

    // Only the proposed authority can finalize the handoff.
    send(
        &mut h,
        &new_authority,
        &mut nonce,
        &[finalize_proposed_authority_ix(&new_authority.pubkey())],
    );
    let config: ProgramConfig = fetch_ncn_account(&h.svm, &ncn_program_config_pda());
    assert_eq!(config.authority, to_pubkey(&new_authority.pubkey()));
    assert_eq!(config.proposed_authority, None);

    // Hand authority back.
    send(
        &mut h,
        &new_authority,
        &mut nonce,
        &[update_ncn_program_config_ix(
            &new_authority.pubkey(),
            Some(&admin.pubkey()),
            None,
            None,
            None,
            None,
        )],
    );
    send(
        &mut h,
        &admin,
        &mut nonce,
        &[finalize_proposed_authority_ix(&admin.pubkey())],
    );
    let config: ProgramConfig = fetch_ncn_account(&h.svm, &ncn_program_config_pda());
    assert_eq!(config.authority, to_pubkey(&admin.pubkey()));
    assert_eq!(config.proposed_authority, None);
    assert_eq!(config.tie_breaker_admin, to_pubkey(&admin.pubkey()));
}

// ---------------------------------------------------------------------------
// 2. Balloting through consensus and finalization (port of test_balloting).
// ---------------------------------------------------------------------------

#[test]
fn balloting_reaches_consensus_and_finalizes() {
    const VOTE_DURATION: i64 = 100_000;
    let (mut h, mut ctx) = setup_flow(8, VOTE_DURATION);

    // The CPI-created ballot box snapshots the config at activation.
    let bb = fetch_ballot_box(&h, &ctx);
    assert_eq!(bb.epoch, CREATION_EPOCH);
    assert_eq!(bb.slot_created, ctx.activation_slot);
    assert_eq!(bb.snapshot_slot, ctx.snapshot_slot);
    assert_eq!(bb.slot_consensus_reached, 0);
    assert_eq!(bb.min_consensus_threshold_bps, THRESHOLD_BPS);
    assert_eq!(bb.winning_ballot, Ballot::default());
    assert!(bb.operator_votes.is_empty());
    assert!(bb.ballot_tallies.is_empty());
    assert_eq!(
        bb.vote_expiry_timestamp,
        ctx.activation_timestamp + VOTE_DURATION
    );
    let expected_voter_list: Vec<_> = ctx
        .operators
        .iter()
        .map(|k| to_pubkey(&k.pubkey()))
        .collect();
    assert_eq!(bb.voter_list, expected_voter_list);
    assert!(!bb.tie_breaker_consensus);

    // Regression for PR #109: voting before the snapshot slot has passed is
    // rejected (the snapshot cannot exist yet).
    let ballot1 = ballot(1, 2);
    assert_ncn_err(
        try_cast(&mut h, &mut ctx, 0, &ballot1),
        NcnError::SnapshotSlotNotReached,
    );
    advance_past_snapshot_slot(&mut h);
    let vote_slot = h.svm.get_sysvar::<Clock>().slot;

    // A zeroed merkle root is not a valid ballot.
    assert_ncn_err(
        try_cast(&mut h, &mut ctx, 0, &ballot(0, 2)),
        NcnError::InvalidBallot,
    );

    // Operator 0 casts the first vote.
    cast(&mut h, &mut ctx, 0, &ballot1);
    let mut expected_votes = vec![OperatorVote {
        operator: to_pubkey(&ctx.operators[0].pubkey()),
        slot_voted: vote_slot,
        ballot_index: 0,
    }];
    let mut expected_tallies = vec![BallotTally {
        index: 0,
        ballot: ballot1.clone(),
        tally: 1,
    }];
    let bb = fetch_ballot_box(&h, &ctx);
    assert_eq!(bb.slot_consensus_reached, 0);
    assert_eq!(bb.operator_votes, expected_votes);
    assert_eq!(bb.ballot_tallies, expected_tallies);

    // Non-whitelisted signers are rejected.
    let outsider = Keypair::new();
    h.svm.airdrop(&outsider.pubkey(), LAMPORTS_PER_SOL).unwrap();
    let ix = ncn_cast_vote_ix(&outsider.pubkey(), ctx.ballot_box, &ballot1);
    assert_ncn_err(
        try_send(&mut h, &outsider, &mut ctx.nonce, &[ix]),
        NcnError::OperatorNotWhitelisted,
    );

    // Operator 1 opens a second tally.
    let ballot2 = ballot(2, 3);
    cast(&mut h, &mut ctx, 1, &ballot2);
    expected_votes.push(OperatorVote {
        operator: to_pubkey(&ctx.operators[1].pubkey()),
        slot_voted: vote_slot,
        ballot_index: 1,
    });
    expected_tallies.push(BallotTally {
        index: 1,
        ballot: ballot2.clone(),
        tally: 1,
    });

    // Operators 2-6 pile onto a third ballot: 5/8 votes = 6250 bps, still
    // under the 6666 bps threshold.
    let ballot3 = ballot(3, 4);
    for i in 2..7 {
        cast(&mut h, &mut ctx, i, &ballot3);
        expected_votes.push(OperatorVote {
            operator: to_pubkey(&ctx.operators[i].pubkey()),
            slot_voted: vote_slot,
            ballot_index: 2,
        });
    }
    expected_tallies.push(BallotTally {
        index: 2,
        ballot: ballot3.clone(),
        tally: 5,
    });
    let bb = fetch_ballot_box(&h, &ctx);
    assert_eq!(bb.slot_consensus_reached, 0);
    assert_eq!(bb.winning_ballot, Ballot::default());
    assert_eq!(bb.operator_votes, expected_votes);
    assert_eq!(bb.ballot_tallies, expected_tallies);

    // Operator 1 withdraws; the empty tally slot is kept to preserve indices.
    let op1 = ctx.operators[1].insecure_clone();
    let ix = ncn_remove_vote_ix(&op1.pubkey(), ctx.ballot_box);
    send(&mut h, &op1, &mut ctx.nonce, &[ix]);
    expected_votes.remove(1);
    expected_tallies[1].tally = 0;
    let bb = fetch_ballot_box(&h, &ctx);
    assert_eq!(bb.operator_votes, expected_votes);
    assert_eq!(bb.ballot_tallies, expected_tallies);

    // Removing again fails: the operator no longer has a vote.
    let ix = ncn_remove_vote_ix(&op1.pubkey(), ctx.ballot_box);
    assert_ncn_err(
        try_send(&mut h, &op1, &mut ctx.nonce, &[ix]),
        NcnError::OperatorHasNotVoted,
    );

    // Finalizing before consensus fails.
    let admin = ctx.admin.insecure_clone();
    let ix = finalize_ballot_ix(&admin.pubkey(), ctx.ballot_box, ctx.consensus_result);
    assert_ncn_err(
        try_send(&mut h, &admin, &mut ctx.nonce, &[ix]),
        NcnError::ConsensusNotReached,
    );

    // Operator 1 switches to ballot 3: 6/8 = 7500 bps crosses the threshold.
    cast(&mut h, &mut ctx, 1, &ballot3);
    expected_votes.push(OperatorVote {
        operator: to_pubkey(&ctx.operators[1].pubkey()),
        slot_voted: vote_slot,
        ballot_index: 2,
    });
    expected_tallies[2].tally += 1;
    let bb = fetch_ballot_box(&h, &ctx);
    assert_eq!(bb.slot_consensus_reached, vote_slot);
    assert_eq!(bb.winning_ballot, ballot3);
    assert_eq!(bb.operator_votes, expected_votes);
    assert_eq!(bb.ballot_tallies, expected_tallies);

    // Late votes are still recorded but cannot change the outcome.
    cast(&mut h, &mut ctx, 7, &ballot3);
    expected_votes.push(OperatorVote {
        operator: to_pubkey(&ctx.operators[7].pubkey()),
        slot_voted: vote_slot,
        ballot_index: 2,
    });
    expected_tallies[2].tally += 1;
    let bb = fetch_ballot_box(&h, &ctx);
    assert_eq!(bb.slot_consensus_reached, vote_slot);
    assert_eq!(bb.winning_ballot, ballot3);
    assert_eq!(bb.operator_votes, expected_votes);
    assert_eq!(bb.ballot_tallies, expected_tallies);

    // One vote per operator.
    assert_ncn_err(
        try_cast(&mut h, &mut ctx, 7, &ballot1),
        NcnError::OperatorHasVoted,
    );

    // No withdrawals once consensus is locked.
    let ix = ncn_remove_vote_ix(&op1.pubkey(), ctx.ballot_box);
    assert_ncn_err(
        try_send(&mut h, &op1, &mut ctx.nonce, &[ix]),
        NcnError::ConsensusReached,
    );

    // Finalize into the ConsensusResult the svmgov proposal is bound to.
    let ix = finalize_ballot_ix(&admin.pubkey(), ctx.ballot_box, ctx.consensus_result);
    send(&mut h, &admin, &mut ctx.nonce, &[ix]);
    let result: ConsensusResult = fetch_ncn_account(&h.svm, &ctx.consensus_result);
    assert_eq!(result.snapshot_slot, ctx.snapshot_slot);
    assert_eq!(result.ballot, ballot3);
    assert!(!result.tie_breaker_consensus);
}

// ---------------------------------------------------------------------------
// 3. Tie-breaker resolution after expiry (port of test_tie_breaker).
// ---------------------------------------------------------------------------

#[test]
fn tie_breaker_decides_expired_vote() {
    const VOTE_DURATION: i64 = 1_000;
    let (mut h, mut ctx) = setup_flow(8, VOTE_DURATION);
    advance_past_snapshot_slot(&mut h);

    // Split vote: 2 + 4 of 8 — the best tally is 5000 bps, no consensus.
    let ballot1 = ballot(1, 3);
    let ballot2 = ballot(2, 4);
    for i in 0..2 {
        cast(&mut h, &mut ctx, i, &ballot1);
    }
    for i in 2..6 {
        cast(&mut h, &mut ctx, i, &ballot2);
    }
    let bb = fetch_ballot_box(&h, &ctx);
    assert_eq!(bb.slot_consensus_reached, 0);
    assert_eq!(bb.winning_ballot, Ballot::default());

    // Tie breaking is only allowed after expiry, and only by the admin.
    let admin = ctx.admin.insecure_clone();
    let winning = ballot(222, 222);
    let ix = set_tie_breaker_ix(&admin.pubkey(), ctx.ballot_box, &winning);
    assert_ncn_err(
        try_send(&mut h, &admin, &mut ctx.nonce, &[ix]),
        NcnError::VotingNotExpired,
    );
    let outsider = Keypair::new();
    h.svm.airdrop(&outsider.pubkey(), LAMPORTS_PER_SOL).unwrap();
    let ix = set_tie_breaker_ix(&outsider.pubkey(), ctx.ballot_box, &winning);
    assert_anchor_err(
        try_send(&mut h, &outsider, &mut ctx.nonce, &[ix]),
        anchor_lang::error::ErrorCode::ConstraintHasOne,
    );

    // Expire the vote; the admin may then decide with ANY ballot.
    set_clock_timestamp(&mut h.svm, ctx.activation_timestamp + VOTE_DURATION + 1);
    let consensus_slot = h.svm.get_sysvar::<Clock>().slot;
    let ix = set_tie_breaker_ix(&admin.pubkey(), ctx.ballot_box, &winning);
    send(&mut h, &admin, &mut ctx.nonce, &[ix]);
    let bb = fetch_ballot_box(&h, &ctx);
    assert_eq!(bb.slot_consensus_reached, consensus_slot);
    assert_eq!(bb.winning_ballot, winning);
    assert!(bb.tie_breaker_consensus);

    // Voting after expiry fails.
    assert_ncn_err(
        try_cast(&mut h, &mut ctx, 6, &ballot1),
        NcnError::VotingExpired,
    );

    // Finalization records the tie-breaker provenance.
    let ix = finalize_ballot_ix(&admin.pubkey(), ctx.ballot_box, ctx.consensus_result);
    send(&mut h, &admin, &mut ctx.nonce, &[ix]);
    let result: ConsensusResult = fetch_ncn_account(&h.svm, &ctx.consensus_result);
    assert_eq!(result.snapshot_slot, ctx.snapshot_slot);
    assert_eq!(result.ballot, winning);
    assert!(result.tie_breaker_consensus);

    // The decision is final.
    let ix = set_tie_breaker_ix(&admin.pubkey(), ctx.ballot_box, &ballot(223, 223));
    assert_ncn_err(
        try_send(&mut h, &admin, &mut ctx.nonce, &[ix]),
        NcnError::ConsensusReached,
    );
}

// ---------------------------------------------------------------------------
// 4. Ballot-box reset once tallies are full (port of test_reset_ballot_box).
// ---------------------------------------------------------------------------

#[test]
fn reset_ballot_box_clears_full_box() {
    const VOTE_DURATION: i64 = 100_000;
    let (mut h, mut ctx) = setup_flow(8, VOTE_DURATION);
    advance_past_snapshot_slot(&mut h);

    let admin = ctx.admin.insecure_clone();
    let op0 = ctx.operators[0].insecure_clone();

    // One operator fills MAX-1 tally slots by casting and immediately
    // withdrawing: empty tallies are retained to preserve indices.
    for i in 0..(MAX_BALLOT_TALLIES - 1) {
        let b = ballot((i as u8).wrapping_add(100), (i as u8).wrapping_add(200));
        let cast_ix = ncn_cast_vote_ix(&op0.pubkey(), ctx.ballot_box, &b);
        let remove_ix = ncn_remove_vote_ix(&op0.pubkey(), ctx.ballot_box);
        send(&mut h, &op0, &mut ctx.nonce, &[cast_ix, remove_ix]);
    }
    let bb = fetch_ballot_box(&h, &ctx);
    assert_eq!(bb.ballot_tallies.len(), MAX_BALLOT_TALLIES - 1);
    assert!(bb.operator_votes.is_empty());
    assert!(bb.ballot_tallies.iter().all(|t| t.tally == 0));

    // Reset requires the box to be completely full.
    let ix = reset_ballot_box_ix(&admin.pubkey(), ctx.ballot_box);
    assert_ncn_err(
        try_send(&mut h, &admin, &mut ctx.nonce, &[ix]),
        NcnError::BallotTalliesNotMaxLength,
    );

    // The final vote fills the last slot (1/8 votes, far from consensus).
    cast(&mut h, &mut ctx, 0, &ballot(222, 222));
    let bb = fetch_ballot_box(&h, &ctx);
    assert_eq!(bb.ballot_tallies.len(), MAX_BALLOT_TALLIES);
    assert_eq!(bb.operator_votes.len(), 1);
    assert_eq!(bb.slot_consensus_reached, 0);

    // Only the tie-breaker admin may reset.
    let outsider = Keypair::new();
    h.svm.airdrop(&outsider.pubkey(), LAMPORTS_PER_SOL).unwrap();
    let ix = reset_ballot_box_ix(&outsider.pubkey(), ctx.ballot_box);
    assert_anchor_err(
        try_send(&mut h, &outsider, &mut ctx.nonce, &[ix]),
        anchor_lang::error::ErrorCode::ConstraintHasOne,
    );

    // Reset clears votes and tallies but keeps the box parameters.
    let ix = reset_ballot_box_ix(&admin.pubkey(), ctx.ballot_box);
    send(&mut h, &admin, &mut ctx.nonce, &[ix]);
    let bb = fetch_ballot_box(&h, &ctx);
    assert!(bb.operator_votes.is_empty());
    assert!(bb.ballot_tallies.is_empty());
    assert_eq!(bb.snapshot_slot, ctx.snapshot_slot);
    assert_eq!(bb.slot_consensus_reached, 0);
    assert!(!bb.tie_breaker_consensus);
    assert_eq!(
        bb.vote_expiry_timestamp,
        ctx.activation_timestamp + VOTE_DURATION
    );
}

// ---------------------------------------------------------------------------
// 5. MetaMerkleProof lifecycle (port of test_merkle_proofs +
//    test_invalid_merkle_proofs), on a fabricated two-bundle snapshot.
// ---------------------------------------------------------------------------

struct Bundle {
    meta_leaf: MetaMerkleLeaf,
    meta_proof: Vec<[u8; 32]>,
    stake_leaves: Vec<StakeMerkleLeaf>,
    stake_proofs: Vec<Vec<[u8; 32]>>,
}

/// Builds `bundle_count` validator bundles of two delegators each plus the
/// meta tree over them, and writes a ConsensusResult carrying the meta root.
fn setup_snapshot(h: &mut Harness, snapshot_slot: u64, bundle_count: usize) -> Vec<Bundle> {
    let mut bundles: Vec<Bundle> = (0..bundle_count)
        .map(|i| {
            let stake_leaves: Vec<StakeMerkleLeaf> = (0..2)
                .map(|d| StakeMerkleLeaf {
                    voting_wallet: to_pubkey(&Address::new_unique()),
                    stake_account: to_pubkey(&Address::new_unique()),
                    active_stake: (100 + 100 * i as u64) + d,
                })
                .collect();
            let contents: Vec<[u8; 32]> =
                stake_leaves.iter().map(|l| l.hash().to_bytes()).collect();
            let (stake_root, stake_proofs) = build_merkle(&contents);
            Bundle {
                meta_leaf: MetaMerkleLeaf {
                    voting_wallet: to_pubkey(&Address::new_unique()),
                    vote_account: to_pubkey(&Address::new_unique()),
                    stake_merkle_root: stake_root,
                    active_stake: 1_000 * (i as u64 + 1),
                },
                meta_proof: Vec::new(),
                stake_leaves,
                stake_proofs,
            }
        })
        .collect();

    let meta_contents: Vec<[u8; 32]> = bundles
        .iter()
        .map(|b| b.meta_leaf.hash().to_bytes())
        .collect();
    let (meta_root, meta_proofs) = build_merkle(&meta_contents);
    for (bundle, proof) in bundles.iter_mut().zip(meta_proofs) {
        bundle.meta_proof = proof;
    }

    // The ConsensusResult carrying this root: fabricated directly — its
    // creation through finalize_ballot is covered by the balloting tests.
    write_ncn_account(
        &mut h.svm,
        consensus_result_pda(snapshot_slot),
        <ConsensusResult as anchor_lang::Discriminator>::DISCRIMINATOR,
        &ConsensusResult {
            snapshot_slot,
            ballot: Ballot {
                meta_merkle_root: meta_root,
                snapshot_hash: [0; 32],
            },
            tie_breaker_consensus: false,
        },
    );
    bundles
}

#[test]
fn meta_merkle_proof_lifecycle() {
    const SNAPSHOT_SLOT: u64 = 777;
    let mut h = setup_harness(CREATION_EPOCH, 1, 1);
    let mut nonce = 0u32;
    let bundles = setup_snapshot(&mut h, SNAPSHOT_SLOT, 2);
    let consensus_result = consensus_result_pda(SNAPSHOT_SLOT);

    let payer = h.validators[0].identity.insecure_clone();
    let proof_account = meta_merkle_proof_pda(
        &consensus_result,
        &Address::new_from_array(bundles[0].meta_leaf.vote_account.to_bytes()),
    );

    // Init with a proof from the wrong bundle is rejected (init verifies).
    let ix = init_meta_merkle_proof_ix(
        &payer.pubkey(),
        consensus_result,
        &bundles[0].meta_leaf,
        &bundles[1].meta_proof,
        1,
    );
    assert_ncn_err(
        try_send(&mut h, &payer, &mut nonce, &[ix]),
        NcnError::InvalidMerkleProof,
    );

    // Valid init persists the leaf and proof.
    let ix = init_meta_merkle_proof_ix(
        &payer.pubkey(),
        consensus_result,
        &bundles[0].meta_leaf,
        &bundles[0].meta_proof,
        1,
    );
    send(&mut h, &payer, &mut nonce, &[ix]);
    let stored: MetaMerkleProof = fetch_ncn_account(&h.svm, &proof_account);
    assert_eq!(stored.payer, to_pubkey(&payer.pubkey()));
    assert_eq!(stored.consensus_result, to_pubkey(&consensus_result));
    assert_eq!(stored.meta_merkle_leaf, bundles[0].meta_leaf);
    assert_eq!(stored.meta_merkle_proof, bundles[0].meta_proof);
    assert_eq!(stored.close_timestamp, 1);

    // Meta-level verification, then both stake leaves under the bundle.
    let ix = ncn_verify_merkle_proof_ix(proof_account, consensus_result, None, None);
    send(&mut h, &payer, &mut nonce, &[ix]);
    for (leaf, proof) in bundles[0].stake_leaves.iter().zip(&bundles[0].stake_proofs) {
        let ix =
            ncn_verify_merkle_proof_ix(proof_account, consensus_result, Some(proof), Some(leaf));
        send(&mut h, &payer, &mut nonce, &[ix]);
    }

    // A stake leaf/proof from the other bundle fails against this root.
    let ix = ncn_verify_merkle_proof_ix(
        proof_account,
        consensus_result,
        Some(&bundles[1].stake_proofs[0]),
        Some(&bundles[1].stake_leaves[0]),
    );
    assert_ncn_err(
        try_send(&mut h, &payer, &mut nonce, &[ix]),
        NcnError::InvalidMerkleProof,
    );

    // Providing only one of (proof, leaf) is rejected.
    let ix = ncn_verify_merkle_proof_ix(
        proof_account,
        consensus_result,
        Some(&bundles[0].stake_proofs[0]),
        None,
    );
    assert_ncn_err(
        try_send(&mut h, &payer, &mut nonce, &[ix]),
        NcnError::InvalidMerkleInputs,
    );
    let ix = ncn_verify_merkle_proof_ix(
        proof_account,
        consensus_result,
        None,
        Some(&bundles[0].stake_leaves[0]),
    );
    assert_ncn_err(
        try_send(&mut h, &payer, &mut nonce, &[ix]),
        NcnError::InvalidMerkleInputs,
    );

    // The payer may close at any time by signing.
    let ix = close_meta_merkle_proof_ix(&payer.pubkey(), true, proof_account);
    send(&mut h, &payer, &mut nonce, &[ix]);
    assert!(
        h.svm
            .get_account(&proof_account)
            .map_or(true, |a| a.lamports == 0 && a.data.is_empty()),
        "proof account must be closed"
    );

    // Re-init with a future close timestamp: anyone may close it, but only
    // once that timestamp has elapsed (rent still returns to the payer).
    let now = h.svm.get_sysvar::<Clock>().unix_timestamp;
    let ix = init_meta_merkle_proof_ix(
        &payer.pubkey(),
        consensus_result,
        &bundles[0].meta_leaf,
        &bundles[0].meta_proof,
        now + 1_000,
    );
    send(&mut h, &payer, &mut nonce, &[ix]);

    let outsider = Keypair::new();
    h.svm.airdrop(&outsider.pubkey(), LAMPORTS_PER_SOL).unwrap();
    let ix = close_meta_merkle_proof_ix(&payer.pubkey(), false, proof_account);
    assert_anchor_err(
        try_send(&mut h, &outsider, &mut nonce, &[ix]),
        anchor_lang::error::ErrorCode::RequireGteViolated,
    );

    set_clock_timestamp(&mut h.svm, now + 1_001);
    let payer_balance_before = h.svm.get_account(&payer.pubkey()).unwrap().lamports;
    let ix = close_meta_merkle_proof_ix(&payer.pubkey(), false, proof_account);
    send(&mut h, &outsider, &mut nonce, &[ix]);
    assert!(
        h.svm
            .get_account(&proof_account)
            .map_or(true, |a| a.lamports == 0 && a.data.is_empty()),
        "proof account must be closed"
    );
    assert!(
        h.svm.get_account(&payer.pubkey()).unwrap().lamports > payer_balance_before,
        "rent must return to the recorded payer"
    );
}

// ---------------------------------------------------------------------------
// 6. The init_ballot_box PDA gate (not covered by the old suite: it ran a
//    `skip-pda-check` build precisely because it could not pass this check).
// ---------------------------------------------------------------------------

/// A direct `init_ballot_box` whose `proposal` signer is a plain keypair (not
/// the svmgov Proposal PDA) must fail the seeds constraint. Also guards
/// against a gate-disabled program artifact ever ending up in target/deploy.
#[test]
fn direct_init_ballot_box_without_proposal_pda_rejected() {
    let (mut h, mut ctx) = setup_flow(1, 1_000);
    let admin = ctx.admin.insecure_clone();

    // A fresh snapshot slot in the future, so only the gate can fail.
    let snapshot_slot = h.svm.get_sysvar::<Clock>().slot + 1_000;
    let mut data = anchor_discriminator("global", "init_ballot_box").to_vec();
    data.extend(snapshot_slot.to_le_bytes());
    data.extend(7u64.to_le_bytes()); // proposal_seed
    data.extend(h.validators[0].vote.pubkey().to_bytes()); // spl_vote_account
    let ix = Instruction {
        program_id: NCN_SNAPSHOT_PROGRAM_ID,
        accounts: vec![
            solana_instruction::AccountMeta::new(admin.pubkey(), true),
            solana_instruction::AccountMeta::new_readonly(admin.pubkey(), true),
            solana_instruction::AccountMeta::new(ballot_box_pda(snapshot_slot), false),
            solana_instruction::AccountMeta::new_readonly(ncn_program_config_pda(), false),
            solana_instruction::AccountMeta::new_readonly(
                solana_sdk_ids::system_program::ID,
                false,
            ),
        ],
        data,
    };
    assert_anchor_err(
        try_send(&mut h, &admin, &mut ctx.nonce, &[ix]),
        anchor_lang::error::ErrorCode::ConstraintSeeds,
    );
}
