//! `migrate_vote_records`: the one-time, permissionless growth of `Vote` and
//! `VoteOverride` accounts recorded before `delegate_signer` existed.
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

/// Borsh size of the appended `delegate_signer: Option<Pubkey>`.
const DELEGATE_SIGNER_LEN: usize = 1 + 32;
/// Account sizes (discriminator included) before the field was appended. The
/// CLI's `LEGACY_VOTE_ACCOUNT_LEN` and `LEGACY_VOTE_OVERRIDE_ACCOUNT_LEN`
/// filters depend on these exact values.
const LEGACY_VOTE_LEN: usize = 145;
const LEGACY_VOTE_OVERRIDE_LEN: usize = 233;

type TxResult = Result<TransactionMetadata, FailedTransactionMetadata>;

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

fn lamports(h: &Harness, address: &Address) -> u64 {
    h.svm.get_account(address).map(|a| a.lamports).unwrap_or(0)
}

fn data_len(h: &Harness, address: &Address) -> usize {
    h.svm
        .get_account(address)
        .map(|a| a.data.len())
        .unwrap_or(0)
}

fn try_migrate(h: &mut Harness, payer: &Keypair, records: &[Address]) -> TxResult {
    h.svm.expire_blockhash();
    try_send_ix(
        &mut h.svm,
        payer,
        &[migrate_vote_records_ix(&payer.pubkey(), records)],
    )
}

/// Rewrites `address` as it would have been recorded before `delegate_signer`
/// existed: `len` bytes of data at that size's rent minimum.
fn downgrade(h: &mut Harness, address: Address, len: usize) {
    let mut account = h.svm.get_account(&address).expect("record must exist");
    assert_eq!(account.data.len(), len + DELEGATE_SIGNER_LEN);
    account.data.truncate(len);
    account.lamports = h.svm.minimum_balance_for_rent_exemption(len);
    h.svm.set_account(address, account).unwrap();
}

/// A proposal in voting with validator 0's vote and its delegator's override
/// recorded, both signed by the voters themselves.
fn fixture_with_records() -> (Harness, VotingScenario, Address, Address) {
    let mut h = setup_harness(1, 3, 3);
    let s = setup_voting_scenario(
        &mut h,
        42,
        "Migration",
        &[
            (0, V_ACTIVE, &[D0]),
            (1, V_ACTIVE, &[D0]),
            (2, V_ACTIVE, &[D0]),
        ],
    );
    let id = h.validators[s.voters[0].idx].identity.insecure_clone();
    let staker = s.voters[0].delegators[0].wallet.insecure_clone();
    assert_ok(try_cast_vote_as(&mut h, &s, 0, &id, 7_000, 3_000, 0));
    assert_ok(try_delegator_override_as(
        "cast_vote_override",
        &mut h,
        &s,
        0,
        0,
        &staker,
        0,
        10_000,
        0,
    ));

    let vote_account = h.validators[s.voters[0].idx].vote.pubkey();
    let vote = vote_pda(&s.proposal, &vote_account);
    let vote_override =
        vote_override_pda(&s.proposal, &s.voters[0].delegators[0].stake_account, &vote);
    (h, s, vote, vote_override)
}

#[test]
fn legacy_records_grow_to_the_current_layout_at_the_payers_expense() {
    let (mut h, s, vote, vote_override) = fixture_with_records();
    let before_vote = fetch_vote(&h, &s, 0).expect("vote must exist");
    let before_override = fetch_vote_override(&h, &s, 0, 0).expect("override must exist");
    assert_eq!(before_vote.delegate_signer, None);
    assert_eq!(before_override.delegate_signer, None);

    downgrade(&mut h, vote, LEGACY_VOTE_LEN);
    downgrade(&mut h, vote_override, LEGACY_VOTE_OVERRIDE_LEN);
    let legacy_rent = lamports(&h, &vote) + lamports(&h, &vote_override);

    let payer = funded_keypair(&mut h);
    let payer_before = lamports(&h, &payer.pubkey());
    assert_ok(try_migrate(&mut h, &payer, &[vote, vote_override]));

    // Both records are back at the current size, rent-exempt, and decode to
    // exactly what was recorded with `delegate_signer` reading as `None`.
    assert_eq!(data_len(&h, &vote), LEGACY_VOTE_LEN + DELEGATE_SIGNER_LEN);
    assert_eq!(
        data_len(&h, &vote_override),
        LEGACY_VOTE_OVERRIDE_LEN + DELEGATE_SIGNER_LEN
    );
    let expected_rent = h
        .svm
        .minimum_balance_for_rent_exemption(LEGACY_VOTE_LEN + DELEGATE_SIGNER_LEN)
        + h.svm
            .minimum_balance_for_rent_exemption(LEGACY_VOTE_OVERRIDE_LEN + DELEGATE_SIGNER_LEN);
    assert_eq!(
        lamports(&h, &vote) + lamports(&h, &vote_override),
        expected_rent
    );
    assert!(lamports(&h, &payer.pubkey()) <= payer_before - (expected_rent - legacy_rent));
    assert_eq!(fetch_vote(&h, &s, 0), Some(before_vote));
    assert_eq!(fetch_vote_override(&h, &s, 0, 0), Some(before_override));

    // A migrated vote is a live vote again: the identity can still modify it.
    let id = h.validators[s.voters[0].idx].identity.insecure_clone();
    assert_ok(try_modify_vote_as(&mut h, &s, 0, &id, 10_000, 0, 0));
    let vote_state = fetch_vote(&h, &s, 0).expect("vote must exist");
    assert_eq!(vote_state.for_votes_bp, 10_000);
    assert_eq!(vote_state.delegate_signer, None);
}

#[test]
fn migration_skips_current_records_and_rejects_anything_else() {
    let (mut h, s, vote, vote_override) = fixture_with_records();
    let payer = funded_keypair(&mut h);

    // Already-current records are a no-op, so a batch can be retried.
    let rent_before = lamports(&h, &vote) + lamports(&h, &vote_override);
    assert_ok(try_migrate(&mut h, &payer, &[vote, vote_override]));
    assert_eq!(
        lamports(&h, &vote) + lamports(&h, &vote_override),
        rent_before
    );
    assert!(fetch_vote(&h, &s, 0).is_some());

    // Other program accounts and records at an unexpected size are refused.
    assert_rejected(
        try_migrate(&mut h, &payer, &[s.proposal]),
        GovernanceError::InvalidVoteRecord,
    );
    downgrade(&mut h, vote, LEGACY_VOTE_LEN);
    let mut account = h.svm.get_account(&vote).unwrap();
    account.data.truncate(LEGACY_VOTE_LEN - 1);
    h.svm.set_account(vote, account).unwrap();
    assert_rejected(
        try_migrate(&mut h, &payer, &[vote]),
        GovernanceError::InvalidVoteRecord,
    );
}
