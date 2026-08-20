//! `create_proposal` input-validation suite: title and description length
//! limits, the GitHub-link format required of descriptions, the proposer
//! stake floor, and the signer-operates-the-vote-account check.
//!
//! Limits come from the harness global config (`common::setup_harness`):
//! titles cap at 200 characters, descriptions at 500.

mod common;

use {
    common::*, solana_address::Address, solana_signer::Signer,
    solana_transaction_error::TransactionError, svmgov_program::GovernanceError,
};

/// Mirror of the `GlobalConfigAccount` limits written by `setup_harness`.
const MAX_TITLE_LEN: usize = 200;
const MAX_DESCRIPTION_LEN: usize = 500;

const VALID_LINK: &str = "https://github.com/solana-foundation/solana-governance-proposals/blob/commit-sha/proposals/title.md";

/// One funded validator is all these tests need.
fn setup() -> Harness {
    setup_harness(1, 1, 1)
}

fn proposal_pda(h: &Harness, seed: u64) -> Address {
    Address::find_program_address(
        &[
            b"proposal",
            &seed.to_le_bytes(),
            h.validators[0].vote.pubkey().as_ref(),
        ],
        &SVMGOV_PROGRAM_ID,
    )
    .0
}

/// Attempts `create_proposal` as validator 0 with the given title/description.
fn try_create(
    h: &mut Harness,
    seed: u64,
    title: &str,
    description: &str,
) -> Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata> {
    let author = h.validators[0].identity.insecure_clone();
    let ix = create_proposal_ix(
        &author.pubkey(),
        proposal_pda(h, seed),
        h.proposal_index,
        h.validators[0].vote.pubkey(),
        h.global_config,
        seed,
        title,
        description,
    );
    try_send_ix(&mut h.svm, &author, &[ix])
}

fn assert_rejected(
    result: Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata>,
    expected: GovernanceError,
) {
    let err = result.expect_err("create_proposal must fail");
    assert_eq!(
        err.err,
        TransactionError::InstructionError(0, anchor_custom_error(expected)),
        "logs: {:#?}",
        err.meta.logs
    );
}

#[test]
fn title_empty_rejected() {
    let mut h = setup();
    assert_rejected(
        try_create(&mut h, 1, "", VALID_LINK),
        GovernanceError::TitleEmpty,
    );
}

#[test]
fn title_too_long_rejected() {
    let mut h = setup();
    let title = "t".repeat(MAX_TITLE_LEN + 1);
    assert_rejected(
        try_create(&mut h, 1, &title, VALID_LINK),
        GovernanceError::TitleTooLong,
    );
}

#[test]
fn description_empty_rejected() {
    let mut h = setup();
    assert_rejected(
        try_create(&mut h, 1, "title", ""),
        GovernanceError::DescriptionEmpty,
    );
}

#[test]
fn description_too_long_rejected() {
    let mut h = setup();
    // Length is checked before link shape: even a well-formed GitHub link is
    // rejected once it exceeds the configured maximum.
    const PREFIX: &str = "https://github.com/solana-foundation/solana-governance-proposals/";
    let description = format!(
        "{PREFIX}{}",
        "a".repeat(MAX_DESCRIPTION_LEN + 1 - PREFIX.len())
    );
    assert_eq!(description.len(), MAX_DESCRIPTION_LEN + 1);
    assert_rejected(
        try_create(&mut h, 1, "title", &description),
        GovernanceError::DescriptionTooLong,
    );
}

#[test]
fn description_must_be_github_link() {
    let mut h = setup();
    let bad_links = [
        "not a link at all",
        "https://example.com/org/repo",             // wrong host
        "http://github.com/org/repo",               // not https
        "https://github.com/",                      // empty path
        "https://github.com/org",                   // single segment
        "https://github.com/org//repo",             // empty segment
        "https://github.com/org/repo?tab=readme",   // query string
        "https://github.com/org/repo#anchor",       // fragment
        "https://github.com/org/repo name",         // whitespace
        "https://github.com/a/b/c/d/e/f/g/h/i/j/k", // 11 segments, max is 10
        "https://github.com/attacker/repo/blob/ref/0022-x.md", // unapproved repository
        "https://github.com/solana-foundation/solana-improvement-documents/blob/ref/0022-x.md", // legacy SIMD repository
        "https://github.com/solana-foundation/solana-governance-proposals/../../attacker/repo/blob/ref/0022-x.md", // path traversal
    ];
    for (i, link) in bad_links.iter().enumerate() {
        assert_rejected(
            try_create(&mut h, 100 + i as u64, "title", link),
            GovernanceError::DescriptionInvalid,
        );
    }
}

#[test]
fn valid_proposal_accepted() {
    let mut h = setup();

    // Boundary: title exactly at the cap; valid link to a proposal document.
    let title = "t".repeat(MAX_TITLE_LEN);
    let description = "https://github.com/solana-foundation/solana-governance-proposals/blob/main/proposals/sgp-0001-title.md";
    try_create(&mut h, 1, &title, description).unwrap_or_else(|e| {
        panic!(
            "create_proposal failed: {:#?}\nlogs: {:#?}",
            e.err, e.meta.logs
        )
    });

    let state = fetch_proposal(&h.svm, &proposal_pda(&h, 1));
    assert_eq!(state.title, title);
    assert_eq!(state.description, description);
    assert_eq!(state.author, pk_bytes(&h.validators[0].identity.pubkey()));
    assert_eq!(state.index, 1);
    assert!(!state.voting);

    // Boundary: description exactly at the cap.
    const PREFIX: &str = "https://github.com/solana-foundation/solana-governance-proposals/";
    let max_description = format!("{PREFIX}{}", "a".repeat(MAX_DESCRIPTION_LEN - PREFIX.len()));
    assert_eq!(max_description.len(), MAX_DESCRIPTION_LEN);
    try_create(&mut h, 2, "second", &max_description).unwrap_or_else(|e| {
        panic!(
            "create_proposal failed: {:#?}\nlogs: {:#?}",
            e.err, e.meta.logs
        )
    });
    assert_eq!(fetch_proposal(&h.svm, &proposal_pda(&h, 2)).index, 2);
}

#[test]
fn insufficient_stake_rejected() {
    let mut h = setup();

    // Rewrite the global config with a stake floor above the validator's
    // 1-SOL epoch stake (fields otherwise mirror `setup_harness`).
    let (global_config, bump) =
        Address::find_program_address(&[b"global_config"], &SVMGOV_PROGRAM_ID);
    write_anchor_account(
        &mut h.svm,
        global_config,
        SVMGOV_PROGRAM_ID,
        &anchor_discriminator("account", "GlobalConfig"),
        &GlobalConfigAccount {
            admin: pk_bytes(&Address::new_unique()),
            pending_admin: None,
            max_title_length: MAX_TITLE_LEN as u16,
            max_description_length: MAX_DESCRIPTION_LEN as u16,
            max_support_epochs: MAX_SUPPORT_EPOCHS,
            min_proposal_stake_lamports: STAKE_PER_VALIDATOR + 1,
            cluster_support_pct_min_bps: h.cluster_support_pct_min_bps,
            discussion_epochs: DISCUSSION_EPOCHS,
            voting_epochs: VOTING_EPOCHS,
            snapshot_epoch_extension: 0,
            snapshot_slot_offset: 0,
            bump,
            max_supporters: MAX_SUPPORTERS,
        },
    );

    assert_rejected(
        try_create(&mut h, 1, "title", VALID_LINK),
        GovernanceError::NotEnoughStake,
    );
}

/// The signer must be the node pubkey of the vote account it proposes with:
/// another funded identity signing against validator 0's vote account is
/// rejected.
#[test]
fn signer_must_operate_the_vote_account() {
    let mut h = setup_harness(1, 2, 2);
    let author = h.validators[1].identity.insecure_clone();
    let vote_account = h.validators[0].vote.pubkey();
    let seed = 1u64;
    let (proposal, _) = Address::find_program_address(
        &[b"proposal", &seed.to_le_bytes(), vote_account.as_ref()],
        &SVMGOV_PROGRAM_ID,
    );
    let ix = create_proposal_ix(
        &author.pubkey(),
        proposal,
        h.proposal_index,
        vote_account,
        h.global_config,
        seed,
        "title",
        VALID_LINK,
    );
    let err =
        try_send_ix(&mut h.svm, &author, &[ix]).expect_err("mismatched node pubkey must fail");
    assert_eq!(
        err.err,
        TransactionError::InstructionError(
            0,
            anchor_custom_error(GovernanceError::InvalidVoteAccount)
        ),
        "logs: {:#?}",
        err.meta.logs
    );
}
