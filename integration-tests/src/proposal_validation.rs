//! `create_proposal` input-validation failures.
//!
//! Each test stands up a ready environment via [`harness::setup_scenario`] (one
//! validator with stake), then submits a `create_proposal` with one bad field and
//! asserts the specific on-chain error. These validations run at the top of the
//! handler (before the stake math), but account validation still requires a real
//! vote account — which the scenario configures.

use surfpool_sdk::{Keypair, Signer};

use crate::harness::*;

const GOOD_TITLE: &str = "Valid Title";
const GOOD_DESCRIPTION: &str = "https://github.com/solana-foundation/governance";

/// One funded validator with a self-stake, svmgov/ncn deployed + configured.
fn validation_scenario() -> Scenario {
    let vote_account = Keypair::new().pubkey();
    let self_stake = Keypair::new().pubkey();
    let identity = Keypair::new();
    let identity_pk = identity.pubkey();
    Scenario {
        admin: Keypair::new(),
        operators: vec![],
        validators: vec![ValidatorSpec {
            identity,
            vote_account,
            stakes: vec![StakeSpec { stake_account: self_stake, voting_wallet: identity_pk, amount: 100 * SOL }],
        }],
        config: GovConfigArgs {
            max_title_length: 200,
            max_description_length: 500,
            max_support_epochs: 0,
            min_proposal_stake_lamports: 10 * SOL,
            cluster_support_pct_min_bps: 7500,
            discussion_epochs: 1,
            voting_epochs: 2,
            snapshot_epoch_extension: 0,
            snapshot_slot_offset: 0,
        },
        fund_lamports: 100_000 * SOL,
    }
}

/// Sets up the scenario, advances to the proposal epoch, and attempts a
/// `create_proposal` with the given title/description — returning the error.
async fn create_proposal_error(title: &str, description: &str) -> String {
    let scenario = validation_scenario();
    let surfnet = setup_scenario(&scenario).await;
    let rpc = surfnet.rpc_client();
    let cheats = surfnet.cheatcodes();
    cheats.time_travel_to_epoch(2).expect("time travel");

    let v = &scenario.validators[0];
    try_send(
        &rpc,
        &[ix_create_proposal(&v.identity.pubkey(), &v.vote_account, 1, title, description)],
        &[&v.identity],
    )
    .expect_err("create_proposal should fail")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_proposal_rejects_long_title() {
    // 201 chars > max_title_length (200).
    let long_title = "a".repeat(201);
    let err = create_proposal_error(&long_title, GOOD_DESCRIPTION).await;
    assert!(err.contains("TitleTooLong"), "expected TitleTooLong, got: {err}");
    println!("✅ create_proposal rejects over-long title (TitleTooLong)");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_proposal_rejects_long_description() {
    // A valid github link that is > max_description_length (500), so it fails the
    // length check before the link check.
    let long_description = format!("https://github.com/org/{}", "a".repeat(500));
    let err = create_proposal_error(GOOD_TITLE, &long_description).await;
    assert!(err.contains("DescriptionTooLong"), "expected DescriptionTooLong, got: {err}");
    println!("✅ create_proposal rejects over-long description (DescriptionTooLong)");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_proposal_rejects_non_github_description() {
    // Non-empty, within the length limit, but not a github link.
    let err = create_proposal_error(GOOD_TITLE, "this is not a github link").await;
    assert!(err.contains("DescriptionInvalid"), "expected DescriptionInvalid, got: {err}");
    println!("✅ create_proposal rejects non-github description (DescriptionInvalid)");
}
