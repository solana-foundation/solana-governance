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

const VALID_LINK: &str = "https://github.com/solana-foundation/solana-governance-proposals/blob/27bca51e5c0fc34ddbea6904faf86f5098225316/proposals/title.md";
const UPDATED_LINK: &str = "https://github.com/solana-foundation/solana-governance-proposals/blob/0123456789abcdef0123456789abcdef01234567/proposals/renamed.md";

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
    let description = VALID_LINK;
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
    const PREFIX: &str = "https://github.com/solana-foundation/solana-governance-proposals/blob/27bca51e5c0fc34ddbea6904faf86f5098225316/";
    let max_description = format!(
        "{PREFIX}{}.md",
        "a".repeat(MAX_DESCRIPTION_LEN - PREFIX.len() - 3)
    );
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
fn author_can_update_description_during_support() {
    let mut h = setup();
    let proposal = proposal_pda(&h, 1);
    try_create(&mut h, 1, "title", VALID_LINK).expect("proposal creation must succeed");

    let author = h.validators[0].identity.insecure_clone();
    send_ix(
        &mut h.svm,
        &author,
        &[update_proposal_description_ix(
            &author.pubkey(),
            proposal,
            h.global_config,
            UPDATED_LINK,
        )],
    );

    assert_eq!(fetch_proposal(&h.svm, &proposal).description, UPDATED_LINK);
}

#[test]
fn only_author_can_update_description() {
    let mut h = setup();
    let proposal = proposal_pda(&h, 1);
    try_create(&mut h, 1, "title", VALID_LINK).expect("proposal creation must succeed");

    let err = try_send_ix(
        &mut h.svm,
        &h.config_admin,
        &[update_proposal_description_ix(
            &h.config_admin.pubkey(),
            proposal,
            h.global_config,
            UPDATED_LINK,
        )],
    )
    .expect_err("non-author must not update proposal description");
    assert_eq!(
        err.err,
        TransactionError::InstructionError(
            0,
            anchor_custom_error(GovernanceError::UnauthorizedProposalUpdate)
        )
    );
}

#[test]
fn description_update_closes_when_voting_starts_and_preserves_supporters() {
    let mut h = setup();
    let proposal = proposal_pda(&h, 1);
    try_create(&mut h, 1, "title", VALID_LINK).expect("proposal creation must succeed");

    let creation_epoch = h.svm.get_sysvar::<solana_clock::Clock>().epoch;
    let ballot_box = seed_ballot_box(&mut h.svm, expected_snapshot_slot(creation_epoch));
    support_one(&mut h, proposal, 0, ballot_box);
    let activated = fetch_proposal(&h.svm, &proposal);
    assert!(activated.voting);
    let account_len = h.svm.get_account(&proposal).unwrap().data.len();

    set_clock(&mut h.svm, activated.start_epoch - 1);
    let author = h.validators[0].identity.insecure_clone();
    send_ix(
        &mut h.svm,
        &author,
        &[update_proposal_description_ix(
            &author.pubkey(),
            proposal,
            h.global_config,
            UPDATED_LINK,
        )],
    );
    let updated = fetch_proposal(&h.svm, &proposal);
    assert_eq!(updated.description, UPDATED_LINK);
    assert_eq!(updated.supporters, activated.supporters);
    assert_eq!(
        h.svm.get_account(&proposal).unwrap().data.len(),
        account_len
    );

    set_clock(&mut h.svm, activated.start_epoch);
    let err = try_send_ix(
        &mut h.svm,
        &author,
        &[update_proposal_description_ix(
            &author.pubkey(),
            proposal,
            h.global_config,
            VALID_LINK,
        )],
    )
    .expect_err("description must freeze once voting starts");
    assert_eq!(
        err.err,
        TransactionError::InstructionError(
            0,
            anchor_custom_error(GovernanceError::CannotModifyAfterStart)
        )
    );
}

#[test]
fn description_update_reuses_canonical_url_validation_and_support_window() {
    let mut h = setup();
    let proposal = proposal_pda(&h, 1);
    try_create(&mut h, 1, "title", VALID_LINK).expect("proposal creation must succeed");
    let author = h.validators[0].identity.insecure_clone();

    let err = try_send_ix(
        &mut h.svm,
        &author,
        &[update_proposal_description_ix(
            &author.pubkey(),
            proposal,
            h.global_config,
            "https://github.com/solana-foundation/solana-governance-proposals/blob/main/proposals/sgp-0001.md",
        )],
    )
    .expect_err("branch refs must be rejected");
    assert_eq!(
        err.err,
        TransactionError::InstructionError(
            0,
            anchor_custom_error(GovernanceError::DescriptionInvalid)
        )
    );

    let creation_epoch = fetch_proposal(&h.svm, &proposal).creation_epoch;
    set_clock(&mut h.svm, creation_epoch + MAX_SUPPORT_EPOCHS + 1);
    let err = try_send_ix(
        &mut h.svm,
        &author,
        &[update_proposal_description_ix(
            &author.pubkey(),
            proposal,
            h.global_config,
            UPDATED_LINK,
        )],
    )
    .expect_err("unsupported proposals must freeze after support closes");
    assert_eq!(
        err.err,
        TransactionError::InstructionError(
            0,
            anchor_custom_error(GovernanceError::SupportPeriodExpired)
        )
    );
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
            new_proposals_allowed: true,
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

#[test]
fn new_proposals_rejected_if_new_proposals_allowed_is_false() {
    let mut h = setup_harness(1, 2, 2);

    let global_config = h.global_config;
    // Rewrite the global config with new_proposals_allowed set to false.
    {
        let config = fetch_global_config(&h.svm, &global_config);
        assert!(
            config.new_proposals_allowed,
            "new proposals should be allowed by default"
        );
        let ix = set_new_proposals_allowed_ix(&h.config_admin.pubkey(), global_config, false);
        let _ = try_send_ix(&mut h.svm, &h.config_admin, &[ix])
            .expect("failed to set new_proposals_allowed to false");
        let config = fetch_global_config(&h.svm, &global_config);
        assert!(
            !config.new_proposals_allowed,
            "new proposals should now be disallowed"
        );
    }

    // Valid proposal creation should now be rejected.
    {
        let title = "t".repeat(MAX_TITLE_LEN);
        let description = VALID_LINK;
        let err = try_create(&mut h, 1, &title, description)
            .expect_err("proposal should be rejected when new_proposals_allowed is false");
        assert_eq!(
            err.err,
            TransactionError::InstructionError(
                0,
                anchor_custom_error(GovernanceError::NewProposalsNotAllowed)
            ),
            "logs: {:#?}",
            err.meta.logs
        );
    }

    // reenable new proposals
    {
        let ix = set_new_proposals_allowed_ix(&h.config_admin.pubkey(), global_config, true);
        let _ = try_send_ix(&mut h.svm, &h.config_admin, &[ix])
            .expect("failed to set new_proposals_allowed to true");
        let config = fetch_global_config(&h.svm, &global_config);
        assert!(
            config.new_proposals_allowed,
            "new proposals should now be allowed"
        );
    }

    // Valid proposal creation should now succeed.
    {
        let title = "x".repeat(MAX_TITLE_LEN);
        let description = VALID_LINK;
        let _ = try_create(&mut h, 1, &title, description)
            .expect("proposal should succeed when new_proposals_allowed is true");
    }
}
