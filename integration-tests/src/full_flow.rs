//! End-to-end governance happy path across BOTH programs on one ephemeral surfnet.
//!
//! Setup (deploy + fund + configure + whitelist + vote/stake accounts) is handled
//! by [`harness::setup_scenario`]; this test drives the flow from there:
//!
//!  5. create a proposal (validator 1)
//!  6. garner support past the threshold (validators 1 then 2) -> CPI init_ballot_box
//!  7. ncn operators upload a fake merkle snapshot -> consensus -> ConsensusResult
//!  8. publish per-validator meta-merkle proofs; advance to the voting epoch
//!  9. validators cast_vote + a staker cast_vote_override (stake sourced from the snapshot)
//! 10. advance past the voting window and finalize the proposal
//!
//! The on-chain epoch stake (used by create/support) and the fake snapshot (used by
//! voting) are built from the SAME amounts, so the test proves the snapshot is the
//! stake source for voting.

use surfpool_sdk::{Keypair, Signer};

use crate::harness::*;

const TITLE: &str = "Integration Test Proposal";
const DESCRIPTION: &str = "https://github.com/solana-foundation/governance";
const SEED: u64 = 42;

// Stake split (lamports). Cluster total = 200 SOL.
const V1_SELF: u64 = 60 * SOL;
const V1_DELEGATED: u64 = 40 * SOL; // the staker who will override
const V2_SELF: u64 = 100 * SOL;

/// Whole-SOL view of a lamport amount, for logging.
fn sol(lamports: u64) -> u64 {
    lamports / SOL
}

/// Short hex prefix of a 32-byte value, for logging.
fn short(bytes: &[u8; 32]) -> String {
    bytes[..4].iter().map(|b| format!("{b:02x}")).collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn full_governance_flow() {
    println!("\n══════════ full governance flow ══════════");

    // Pubkeys (Copy) reused in the scenario + the flow below.
    let v1_vote = Keypair::new().pubkey();
    let v2_vote = Keypair::new().pubkey();
    let v1_self_stake = Keypair::new().pubkey();
    let v1_delegated_stake = Keypair::new().pubkey();
    let v2_self_stake = Keypair::new().pubkey();
    // The delegated staker signs its own override, so the test owns this keypair
    // (the scenario only needs its pubkey as a stake voting wallet).
    let staker = Keypair::new();

    // Validator identity keypairs are owned by the scenario; capture pubkeys first
    // so the self-stakes can reference them.
    let v1_identity = Keypair::new();
    let v2_identity = Keypair::new();
    let v1_id = v1_identity.pubkey();
    let v2_id = v2_identity.pubkey();

    let scenario = Scenario {
        admin: Keypair::new(),
        operators: (0..3).map(|_| Keypair::new()).collect(),
        validators: vec![
            ValidatorSpec {
                identity: v1_identity,
                vote_account: v1_vote,
                stakes: vec![
                    StakeSpec { stake_account: v1_self_stake, voting_wallet: v1_id, amount: V1_SELF },
                    StakeSpec { stake_account: v1_delegated_stake, voting_wallet: staker.pubkey(), amount: V1_DELEGATED },
                ],
            },
            ValidatorSpec {
                identity: v2_identity,
                vote_account: v2_vote,
                stakes: vec![StakeSpec { stake_account: v2_self_stake, voting_wallet: v2_id, amount: V2_SELF }],
            },
        ],
        config: GovConfigArgs {
            max_title_length: 200,
            max_description_length: 500,
            max_support_epochs: 0, // support must land in the creation epoch
            min_proposal_stake_lamports: 10 * SOL,
            cluster_support_pct_min_bps: 7500, // 150 SOL: needs both validators to support
            discussion_epochs: 1,
            voting_epochs: 2,
            snapshot_epoch_extension: 0,
            snapshot_slot_offset: 0,
        },
        fund_lamports: 100_000 * SOL,
    };

    let surfnet = setup_scenario(&scenario).await;
    let rpc = surfnet.rpc_client();
    let cheats = surfnet.cheatcodes();
    let admin = &scenario.admin;
    let v1_identity = &scenario.validators[0].identity;
    let v2_identity = &scenario.validators[1].identity;
    println!(
        "[1-4] setup complete: both programs deployed + configured, {} operators whitelisted, validators v1={} SOL ({}+{} delegated) / v2={} SOL, cluster={} SOL",
        scenario.operators.len(),
        sol(V1_SELF + V1_DELEGATED),
        sol(V1_SELF),
        sol(V1_DELEGATED),
        sol(V2_SELF),
        sol(V1_SELF + V1_DELEGATED + V2_SELF)
    );

    // svmgov is linked in the ncn ProgramConfig (required for the support CPI).
    let ncn_cfg: NcnProgramConfigAcct = fetch(&rpc, &ncn_program_config_pda());
    assert_eq!(ncn_cfg.svmgov_program_pubkey, SVMGOV_ID.to_bytes());

    cheats.time_travel_to_epoch(2).expect("time travel to epoch 2");
    println!("    time-traveled to epoch 2 (stake active; on-chain epoch-stake syscall non-zero)");

    // --- 5. create proposal (validator 1) ------------------------------------
    send(
        &rpc,
        &[ix_create_proposal(&v1_identity.pubkey(), &v1_vote, SEED, TITLE, DESCRIPTION)],
        &[v1_identity],
    );
    let proposal = proposal_pda(SEED, &v1_vote);
    let p: ProposalAcct = fetch(&rpc, &proposal);
    assert_eq!(p.author, v1_identity.pubkey().to_bytes());
    assert_eq!(p.vote_account_pubkey, v1_vote.to_bytes());
    assert!(!p.voting, "voting must not start at creation");
    println!("[5] proposal created: {proposal} (author=validator1, voting=false)");

    // --- 6. garner support past threshold (CPI opens the ballot box) ---------
    let snapshot_slot = expected_snapshot_slot(
        2,
        scenario.config.discussion_epochs,
        scenario.config.snapshot_epoch_extension,
        scenario.config.snapshot_slot_offset,
    );
    send(
        &rpc,
        &[ix_support_proposal(&v1_identity.pubkey(), &proposal, &v1_vote, snapshot_slot)],
        &[v1_identity],
    );
    let p: ProposalAcct = fetch(&rpc, &proposal);
    assert!(!p.voting, "one supporter should not cross the threshold");
    println!(
        "[6] support: validator1 pledged {} SOL (cluster_support={} SOL) — below threshold",
        sol(V1_SELF + V1_DELEGATED),
        sol(p.cluster_support_lamports)
    );
    send(
        &rpc,
        &[ix_support_proposal(&v2_identity.pubkey(), &proposal, &v2_vote, snapshot_slot)],
        &[v2_identity],
    );
    let p: ProposalAcct = fetch(&rpc, &proposal);
    assert!(p.voting, "second supporter should activate voting");
    assert_eq!(p.snapshot_slot, snapshot_slot);
    assert_eq!(p.consensus_result, Some(consensus_result_pda(snapshot_slot).to_bytes()));
    println!(
        "    validator2 pledged {} SOL (cluster_support={} SOL) — voting ACTIVATED, ballot box opened @ slot {snapshot_slot}",
        sol(V2_SELF),
        sol(p.cluster_support_lamports)
    );

    // --- 7. build + upload the fake merkle snapshot --------------------------
    let snapshot = build_fake_snapshot(&scenario.snapshot_validators());
    let ballot = Ballot { meta_merkle_root: snapshot.root, snapshot_hash: [0u8; 32] };
    println!(
        "[7] built fake merkle snapshot (root={}…) over {} validators",
        short(&snapshot.root),
        snapshot.validators.len()
    );
    for (i, op) in scenario.operators.iter().enumerate() {
        send(&rpc, &[ix_ncn_cast_vote(&op.pubkey(), snapshot_slot, ballot.clone())], &[admin, op]);
        println!("    operator {} cast ballot for root {}…", i + 1, short(&snapshot.root));
    }
    send(&rpc, &[ix_finalize_ballot(&admin.pubkey(), snapshot_slot)], &[admin]);
    let cr: ConsensusResultAcct = fetch(&rpc, &consensus_result_pda(snapshot_slot));
    assert_eq!(cr.ballot.meta_merkle_root, snapshot.root, "consensus must lock in the snapshot root");
    println!("    consensus reached -> ConsensusResult locked in root {}…", short(&cr.ballot.meta_merkle_root));

    // --- 8. publish per-validator meta-merkle proofs (verified on-chain) -----
    let far_future = 32_503_680_000i64;
    for vb in &snapshot.validators {
        send(
            &rpc,
            &[ix_init_meta_merkle_proof(&admin.pubkey(), snapshot_slot, vb.meta_leaf.clone(), vb.meta_proof.clone(), far_future)],
            &[admin],
        );
    }
    println!("[8] published {} meta-merkle proofs (verified on-chain against the ConsensusResult root)", snapshot.validators.len());

    cheats.time_travel_to_epoch(4).expect("time travel to start_epoch");
    println!("    time-traveled to epoch 4 — voting window [start={}, end={}) open", p.start_epoch, p.end_epoch);

    // --- 9. votes (stake sourced from the snapshot) --------------------------
    println!("[9] voting (stake sourced from the merkle snapshot):");
    send(
        &rpc,
        &[ix_cast_vote(&v1_identity.pubkey(), &proposal, &v1_vote, snapshot_slot, 10_000, 0, 0)],
        &[v1_identity],
    );
    println!("    validator1 cast_vote FOR ({} SOL)", sol(V1_SELF + V1_DELEGATED));
    send(
        &rpc,
        &[ix_cast_vote(&v2_identity.pubkey(), &proposal, &v2_vote, snapshot_slot, 0, 10_000, 0)],
        &[v2_identity],
    );
    println!("    validator2 cast_vote AGAINST ({} SOL)", sol(V2_SELF));
    // The staker (40 SOL delegated to V1) overrides to ABSTAIN, pulling 40 SOL out
    // of V1's FOR via the two-tier stake-merkle proof.
    let staker_bundle = snapshot.validators[0]
        .stakes
        .iter()
        .find(|s| s.leaf.stake_account == v1_delegated_stake.to_bytes())
        .expect("staker stake bundle");
    send(
        &rpc,
        &[ix_cast_vote_override(
            &staker.pubkey(),
            &proposal,
            &v1_vote,
            &v1_delegated_stake,
            snapshot_slot,
            0,
            0,
            10_000,
            staker_bundle.proof.clone(),
            staker_bundle.leaf.clone(),
        )],
        &[&staker],
    );
    println!(
        "    staker cast_vote_override ABSTAIN ({} SOL) — two-tier stake-merkle proof pulls it out of validator1's FOR",
        sol(V1_DELEGATED)
    );

    let p: ProposalAcct = fetch(&rpc, &proposal);
    assert_eq!(p.for_votes_lamports, V1_SELF, "FOR = V1 (100) - override (40) = 60 SOL");
    assert_eq!(p.against_votes_lamports, V2_SELF, "AGAINST = V2 = 100 SOL");
    assert_eq!(p.abstain_votes_lamports, V1_DELEGATED, "ABSTAIN = staker override = 40 SOL");
    println!(
        "    tally: FOR={} SOL  AGAINST={} SOL  ABSTAIN={} SOL",
        sol(p.for_votes_lamports),
        sol(p.against_votes_lamports),
        sol(p.abstain_votes_lamports)
    );

    // --- 10. finalize past the voting window ---------------------------------
    cheats.time_travel_to_epoch(6).expect("time travel to end_epoch");
    send(&rpc, &[ix_finalize_proposal(&admin.pubkey(), &proposal)], &[admin]);
    let p: ProposalAcct = fetch(&rpc, &proposal);
    assert!(p.finalized, "proposal should be finalized");
    println!("[10] time-traveled past end_epoch and finalized the proposal");

    println!(
        "✅ full governance flow ok — proposal {proposal} finalized: FOR={} SOL  AGAINST={} SOL  ABSTAIN={} SOL\n",
        sol(p.for_votes_lamports),
        sol(p.against_votes_lamports),
        sol(p.abstain_votes_lamports)
    );
}
