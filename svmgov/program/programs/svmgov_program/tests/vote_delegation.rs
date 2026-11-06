//! Vote delegation suite (issue #215): the `delegate_vote` /
//! `accept_vote_delegation` / `remove_vote_delegation` authority rules, and how
//! an accepted delegation redirects `cast_vote`, `modify_vote`,
//! `support_proposal`, `create_proposal` and `update_proposal_description`
//! from the validator identity to the delegate while keeping every on-chain
//! record attributed to the identity.
mod common;

use {
    common::*,
    litesvm::types::{FailedTransactionMetadata, TransactionMetadata},
    solana_address::Address,
    solana_keypair::Keypair,
    solana_native_token::LAMPORTS_PER_SOL,
    solana_signer::Signer,
    solana_transaction_error::TransactionError,
    svmgov_program::GovernanceError,
};

/// Meta-leaf active stake per snapshot validator (1 SOL).
const V_ACTIVE: u64 = 1_000_000_000;
/// Stake-leaf stake of the single delegator under each validator.
const D0: u64 = 300_000_000;

const VALID_LINK: &str = "https://github.com/solana-foundation/solana-governance-proposals/blob/27bca51e5c0fc34ddbea6904faf86f5098225316/proposals/title.md";
const UPDATED_LINK: &str = "https://github.com/solana-foundation/solana-governance-proposals/blob/0123456789abcdef0123456789abcdef01234567/proposals/renamed.md";

type TxResult = Result<TransactionMetadata, FailedTransactionMetadata>;

/// Mirrors `calculate_vote_lamports!`: stake * bp / 10_000, floored.
fn lam(stake: u64, bp: u64) -> u64 {
    ((stake as u128) * (bp as u128) / 10_000) as u64
}

/// Asserts the transaction failed with `expected`, whichever instruction raised
/// it (support transactions carry a leading compute-budget instruction).
fn assert_rejected(result: TxResult, expected: GovernanceError) {
    let err = result.expect_err("transaction must be rejected");
    match err.err {
        TransactionError::InstructionError(_, ref ix_err) => {
            assert_eq!(
                *ix_err,
                anchor_custom_error(expected),
                "logs: {:#?}",
                err.meta.logs
            )
        }
        other => panic!(
            "expected an instruction error, got {other:?}\nlogs: {:#?}",
            err.meta.logs
        ),
    }
}

/// Sends `ix` signed by `signer` under a fresh blockhash, so repeating an
/// identical instruction is not rejected as an already-processed transaction.
fn send(h: &mut Harness, signer: &Keypair, ix: solana_instruction::Instruction) -> TxResult {
    h.svm.expire_blockhash();
    try_send_ix(&mut h.svm, signer, &[ix])
}

fn assert_ok(result: TxResult) -> TransactionMetadata {
    result.unwrap_or_else(|e| panic!("tx failed: {:#?}\nlogs: {:#?}", e.err, e.meta.logs))
}

fn funded_keypair(h: &mut Harness) -> Keypair {
    let keypair = Keypair::new();
    h.svm
        .airdrop(&keypair.pubkey(), 10 * LAMPORTS_PER_SOL)
        .unwrap();
    keypair
}

fn vote_account(h: &Harness, idx: usize) -> Address {
    h.validators[idx].vote.pubkey()
}

fn identity(h: &Harness, idx: usize) -> Keypair {
    h.validators[idx].identity.insecure_clone()
}

fn lamports(h: &Harness, address: &Address) -> u64 {
    h.svm.get_account(address).map(|a| a.lamports).unwrap_or(0)
}

fn try_delegate(h: &mut Harness, idx: usize, signer: &Keypair, delegate: &Address) -> TxResult {
    let vote = vote_account(h, idx);
    send(
        h,
        signer,
        delegate_vote_ix(&signer.pubkey(), vote, delegate),
    )
}

fn try_accept(h: &mut Harness, idx: usize, signer: &Keypair) -> TxResult {
    let vote = vote_account(h, idx);
    send(h, signer, accept_vote_delegation_ix(&signer.pubkey(), vote))
}

fn try_remove(h: &mut Harness, idx: usize, signer: &Keypair, creator: Address) -> TxResult {
    let vote = vote_account(h, idx);
    send(
        h,
        signer,
        remove_vote_delegation_ix(&signer.pubkey(), vote, creator),
    )
}

/// Delegates validator `idx`'s vote to a fresh funded key that accepts.
fn delegate_and_accept(h: &mut Harness, idx: usize) -> Keypair {
    let delegate = funded_keypair(h);
    let id = identity(h, idx);
    assert_ok(try_delegate(h, idx, &id, &delegate.pubkey()));
    assert_ok(try_accept(h, idx, &delegate));
    delegate
}

fn voting_fixture() -> (Harness, VotingScenario) {
    // 3 validators, all must support: threshold = 10_000 bps.
    let mut h = setup_harness(1, 3, 3);
    let s = setup_voting_scenario(
        &mut h,
        42,
        "Delegation",
        &[
            (0, V_ACTIVE, &[D0]),
            (1, V_ACTIVE, &[D0]),
            (2, V_ACTIVE, &[D0]),
        ],
    );
    (h, s)
}

// ---------------------------------------------------------------------------
// Creating, accepting and removing a delegation.
// ---------------------------------------------------------------------------

#[test]
fn only_the_identity_or_withdrawer_can_delegate() {
    let mut h = setup_harness(1, 1, 1);
    let withdrawer = funded_keypair(&mut h);
    set_vote_account_withdrawer(&mut h, 0, &withdrawer.pubkey());
    let id = identity(&h, 0);
    let vote = vote_account(&h, 0);
    let delegate = Keypair::new().pubkey();

    let stranger = funded_keypair(&mut h);
    assert_rejected(
        try_delegate(&mut h, 0, &stranger, &delegate),
        GovernanceError::UnauthorizedVoteDelegation,
    );
    assert_rejected(
        try_delegate(&mut h, 0, &id, &Address::default()),
        GovernanceError::InvalidVoteDelegate,
    );

    assert_ok(try_delegate(&mut h, 0, &id, &delegate));
    let state = fetch_vote_delegation(&h.svm, &vote).expect("delegation must exist");
    assert_eq!(state.delegate, delegate.to_bytes());
    assert_eq!(state.vote_account, vote.to_bytes());
    assert_eq!(state.creator, id.pubkey().to_bytes());
    assert!(!state.delegation_accepted);

    // One delegation per vote account.
    assert!(try_delegate(&mut h, 0, &id, &delegate).is_err());

    // The withdrawer may create one too (after the pending one is withdrawn).
    assert_ok(try_remove(&mut h, 0, &id, id.pubkey()));
    assert!(fetch_vote_delegation(&h.svm, &vote).is_none());
    assert_ok(try_delegate(&mut h, 0, &withdrawer, &delegate));
    let state = fetch_vote_delegation(&h.svm, &vote).expect("delegation must exist");
    assert_eq!(state.creator, withdrawer.pubkey().to_bytes());
}

#[test]
fn only_the_named_delegate_accepts_and_only_once() {
    let mut h = setup_harness(1, 1, 1);
    let id = identity(&h, 0);
    let delegate = funded_keypair(&mut h);
    let stranger = funded_keypair(&mut h);
    assert_ok(try_delegate(&mut h, 0, &id, &delegate.pubkey()));

    assert_rejected(
        try_accept(&mut h, 0, &stranger),
        GovernanceError::NotVoteDelegate,
    );
    assert_rejected(try_accept(&mut h, 0, &id), GovernanceError::NotVoteDelegate);
    assert_ok(try_accept(&mut h, 0, &delegate));
    assert!(
        fetch_vote_delegation(&h.svm, &vote_account(&h, 0))
            .unwrap()
            .delegation_accepted
    );
    assert_rejected(
        try_accept(&mut h, 0, &delegate),
        GovernanceError::VoteDelegationAlreadyAccepted,
    );
}

#[test]
fn removal_authority_follows_acceptance_with_the_withdrawer_as_escape_hatch() {
    let (mut h, s) = voting_fixture();
    let id = identity(&h, 0);
    let withdrawer = funded_keypair(&mut h);
    set_vote_account_withdrawer(&mut h, 0, &withdrawer.pubkey());
    let stranger = funded_keypair(&mut h);
    let delegate = funded_keypair(&mut h);
    let vote = vote_account(&h, 0);

    // Pending: the identity may withdraw it, strangers may not, and the rent
    // must go back to the recorded creator.
    assert_ok(try_delegate(&mut h, 0, &id, &delegate.pubkey()));
    assert_rejected(
        try_remove(&mut h, 0, &stranger, id.pubkey()),
        GovernanceError::UnauthorizedVoteDelegationRemoval,
    );
    assert_rejected(
        try_remove(&mut h, 0, &id, stranger.pubkey()),
        GovernanceError::InvalidVoteDelegationCreator,
    );
    assert_ok(try_remove(&mut h, 0, &id, id.pubkey()));
    assert!(fetch_vote_delegation(&h.svm, &vote).is_none());

    // Accepted: the hot identity key is locked out, the delegate may resign,
    // and the creator receives the rent.
    assert_ok(try_delegate(&mut h, 0, &id, &delegate.pubkey()));
    assert_ok(try_accept(&mut h, 0, &delegate));
    assert_rejected(
        try_remove(&mut h, 0, &id, id.pubkey()),
        GovernanceError::UnauthorizedVoteDelegationRemoval,
    );
    let rent = lamports(&h, &vote_delegation_pda(&vote));
    let before = lamports(&h, &id.pubkey());
    assert_ok(try_remove(&mut h, 0, &delegate, id.pubkey()));
    assert_eq!(lamports(&h, &id.pubkey()), before + rent);

    // Accepted, delegate key lost: the authorized withdrawer revokes, after
    // which the identity votes again.
    assert_ok(try_delegate(&mut h, 0, &id, &delegate.pubkey()));
    assert_ok(try_accept(&mut h, 0, &delegate));
    assert_rejected(
        try_cast_vote_as(&mut h, &s, 0, &id, 10_000, 0, 0),
        GovernanceError::NotVoteDelegate,
    );
    assert_ok(try_remove(&mut h, 0, &withdrawer, id.pubkey()));
    assert_ok(try_cast_vote_as(&mut h, &s, 0, &id, 10_000, 0, 0));
}

#[test]
fn creator_reclaims_rent_once_the_vote_account_is_gone() {
    let mut h = setup_harness(1, 1, 1);
    let id = identity(&h, 0);
    let vote = vote_account(&h, 0);
    let _delegate = delegate_and_accept(&mut h, 0);
    let stranger = funded_keypair(&mut h);

    clear_account(&mut h.svm, vote);
    assert_rejected(
        try_remove(&mut h, 0, &stranger, id.pubkey()),
        GovernanceError::UnauthorizedVoteDelegationRemoval,
    );
    assert_ok(try_remove(&mut h, 0, &id, id.pubkey()));
    assert!(fetch_vote_delegation(&h.svm, &vote).is_none());
}

// ---------------------------------------------------------------------------
// Effect on voting.
// ---------------------------------------------------------------------------

#[test]
fn delegation_is_inert_until_accepted() {
    let (mut h, s) = voting_fixture();
    let id = identity(&h, 0);
    let delegate = funded_keypair(&mut h);
    let stranger = funded_keypair(&mut h);
    assert_ok(try_delegate(&mut h, 0, &id, &delegate.pubkey()));

    // The pending delegate is told what is missing; anyone else is simply not
    // the voting wallet.
    assert_rejected(
        try_cast_vote_as(&mut h, &s, 0, &delegate, 10_000, 0, 0),
        GovernanceError::VoteDelegationNotAccepted,
    );
    assert_rejected(
        try_cast_vote_as(&mut h, &s, 0, &stranger, 10_000, 0, 0),
        GovernanceError::UnauthorizedVoteSigner,
    );
    assert_ok(try_cast_vote_as(&mut h, &s, 0, &id, 10_000, 0, 0));
    let vote = fetch_vote(&h, &s, 0).expect("vote must exist");
    assert_eq!(vote.validator, id.pubkey().to_bytes());
    assert_rejected(
        try_modify_vote_as(&mut h, &s, 0, &delegate, 0, 10_000, 0),
        GovernanceError::VoteDelegationNotAccepted,
    );
}

#[test]
fn accepted_delegate_votes_with_the_validators_stake_and_locks_out_the_identity() {
    let (mut h, s) = voting_fixture();
    let id = identity(&h, 0);
    let delegate = delegate_and_accept(&mut h, 0);

    assert_rejected(
        try_cast_vote_as(&mut h, &s, 0, &id, 10_000, 0, 0),
        GovernanceError::NotVoteDelegate,
    );

    // The delegate votes with the full validator stake; the Vote still records
    // the identity, so per-validator attribution is unchanged for clients.
    assert_ok(try_cast_vote_as(&mut h, &s, 0, &delegate, 7_000, 3_000, 0));
    let vote = fetch_vote(&h, &s, 0).expect("vote must exist");
    assert_eq!(vote.validator, id.pubkey().to_bytes());
    assert_eq!(vote.stake, V_ACTIVE);
    assert_eq!(
        (vote.for_votes_lamports, vote.against_votes_lamports),
        (lam(V_ACTIVE, 7_000), lam(V_ACTIVE, 3_000))
    );

    assert_rejected(
        try_modify_vote_as(&mut h, &s, 0, &id, 0, 10_000, 0),
        GovernanceError::NotVoteDelegate,
    );
    assert_ok(try_modify_vote_as(&mut h, &s, 0, &delegate, 0, 10_000, 0));

    // Undelegated validators are unaffected.
    cast_validator_vote(&mut h, &s, 1, 10_000, 0, 0);
    cast_validator_vote(&mut h, &s, 2, 0, 0, 10_000);

    let state = finalize_proposal(&mut h, &s);
    assert_eq!(
        (
            state.for_votes_lamports,
            state.against_votes_lamports,
            state.abstain_votes_lamports
        ),
        (V_ACTIVE, V_ACTIVE, V_ACTIVE)
    );
    assert_eq!(state.vote_count, 3);
}

#[test]
fn delegate_takes_over_a_vote_the_identity_cast_before_accepting() {
    let (mut h, s) = voting_fixture();
    let id = identity(&h, 0);
    cast_validator_vote(&mut h, &s, 0, 10_000, 0, 0);

    let delegate = delegate_and_accept(&mut h, 0);
    assert_rejected(
        try_modify_vote_as(&mut h, &s, 0, &id, 0, 10_000, 0),
        GovernanceError::NotVoteDelegate,
    );
    assert_ok(try_modify_vote_as(&mut h, &s, 0, &delegate, 0, 10_000, 0));

    let vote = fetch_vote(&h, &s, 0).expect("vote must exist");
    assert_eq!(vote.validator, id.pubkey().to_bytes());
    let state = fetch_proposal(&h.svm, &s.proposal);
    assert_eq!(
        (state.for_votes_lamports, state.against_votes_lamports),
        (0, V_ACTIVE)
    );
}

// ---------------------------------------------------------------------------
// Effect on proposing and supporting.
// ---------------------------------------------------------------------------

#[test]
fn accepted_delegate_creates_supports_and_edits_proposals_for_the_identity() {
    const CREATION_EPOCH: u64 = 1;
    let mut h = setup_harness(CREATION_EPOCH, 3, 3);
    let id0 = identity(&h, 0);
    let vote0 = vote_account(&h, 0);
    let delegate = delegate_and_accept(&mut h, 0);
    let stranger = funded_keypair(&mut h);

    let seed = 7u64;
    let proposal = Address::find_program_address(
        &[b"proposal", &seed.to_le_bytes(), vote0.as_ref()],
        &SVMGOV_PROGRAM_ID,
    )
    .0;

    // The identity is locked out of proposing from its vote account; the
    // delegate proposes and the proposal is attributed to the identity.
    let ix = create_proposal_ix(
        &id0.pubkey(),
        proposal,
        h.proposal_index,
        vote0,
        h.global_config,
        seed,
        "Delegated",
        VALID_LINK,
    );
    assert_rejected(send(&mut h, &id0, ix), GovernanceError::NotVoteDelegate);
    let ix = create_proposal_ix(
        &delegate.pubkey(),
        proposal,
        h.proposal_index,
        vote0,
        h.global_config,
        seed,
        "Delegated",
        VALID_LINK,
    );
    assert_ok(send(&mut h, &delegate, ix));
    assert_eq!(
        fetch_proposal(&h.svm, &proposal).author,
        id0.pubkey().to_bytes()
    );

    // Support is recorded against the identity; undelegated validators are
    // unaffected.
    let ballot_box = seed_ballot_box(&mut h.svm, expected_snapshot_slot(CREATION_EPOCH));
    assert_rejected(
        try_support_as(&mut h, proposal, 0, &id0, ballot_box),
        GovernanceError::NotVoteDelegate,
    );
    assert_ok(try_support_as(&mut h, proposal, 0, &delegate, ballot_box));
    let support = fetch_support(&h.svm, &proposal, &vote0).expect("support must exist");
    assert_eq!(support.validator, id0.pubkey().to_bytes());
    support_one(&mut h, proposal, 1, ballot_box);

    // Description updates follow the same authority as creation.
    let ix = update_proposal_description_ix(
        &id0.pubkey(),
        proposal,
        vote0,
        h.global_config,
        UPDATED_LINK,
    );
    assert_rejected(send(&mut h, &id0, ix), GovernanceError::NotVoteDelegate);
    let ix = update_proposal_description_ix(
        &stranger.pubkey(),
        proposal,
        vote0,
        h.global_config,
        UPDATED_LINK,
    );
    // With a delegation in effect nobody but the delegate qualifies, so even a
    // stranger is told so (UnauthorizedProposalUpdate is the undelegated case).
    assert_rejected(
        send(&mut h, &stranger, ix),
        GovernanceError::NotVoteDelegate,
    );
    let ix = update_proposal_description_ix(
        &delegate.pubkey(),
        proposal,
        vote0,
        h.global_config,
        UPDATED_LINK,
    );
    assert_ok(send(&mut h, &delegate, ix));
    assert_eq!(fetch_proposal(&h.svm, &proposal).description, UPDATED_LINK);
}
