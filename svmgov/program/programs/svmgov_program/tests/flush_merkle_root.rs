//! End-to-end coverage for the admin snapshot recovery path.

mod common;

use {
    common::*,
    ncn_snapshot::{BallotBox, ProgramConfig},
    solana_address::Address,
    solana_instruction::{AccountMeta, Instruction},
    solana_sdk_ids::system_program,
    solana_signer::Signer,
};

const CREATION_EPOCH: u64 = 1;
const SNAPSHOT_EPOCH_EXTENSION: u64 = 1;

fn flush_merkle_root_ix(
    signer: &Address,
    proposal: Address,
    vote_account: Address,
    ballot_box: Address,
    program_config: Address,
    global_config: Address,
) -> Instruction {
    Instruction {
        program_id: SVMGOV_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*signer, true),
            AccountMeta::new(proposal, false),
            AccountMeta::new_readonly(vote_account, false),
            AccountMeta::new(ballot_box, false),
            AccountMeta::new_readonly(NCN_SNAPSHOT_PROGRAM_ID, false),
            AccountMeta::new_readonly(program_config, false),
            AccountMeta::new_readonly(global_config, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: anchor_discriminator("global", "flush_merkle_root").to_vec(),
    }
}

#[test]
fn flush_recomputes_proposal_lineage_and_ballot_expiry() {
    let mut harness = setup_harness(CREATION_EPOCH, 3, 3);

    let config_admin = harness.config_admin.insecure_clone();
    send_ix(
        &mut harness.svm,
        &config_admin,
        &[update_global_config_ix(
            &config_admin.pubkey(),
            harness.global_config,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(SNAPSHOT_EPOCH_EXTENSION),
            None,
            None,
            Some(true),
        )],
    );

    clear_account(&mut harness.svm, ncn_program_config_pda());
    send_ix(
        &mut harness.svm,
        &config_admin,
        &[init_ncn_program_config_ix(
            &config_admin.pubkey(),
            &config_admin.pubkey(),
            &SVMGOV_PROGRAM_ID,
        )],
    );
    send_ix(
        &mut harness.svm,
        &config_admin,
        &[update_ncn_program_config_ix(
            &config_admin.pubkey(),
            None,
            Some(10_000),
            Some(&config_admin.pubkey()),
            None,
        )],
    );
    let config: ProgramConfig = fetch_ncn_account(&harness.svm, &ncn_program_config_pda());
    assert_eq!(config.min_consensus_threshold_bps, 10_000);

    let proposal = create_proposal(&mut harness, 77, "flush recovery");
    let initial_snapshot_epoch = CREATION_EPOCH + DISCUSSION_EPOCHS + SNAPSHOT_EPOCH_EXTENSION;
    let initial_ballot_box = ballot_box_pda(initial_snapshot_epoch * SLOTS_PER_EPOCH);
    for supporter in 0..harness.supporter_count {
        support_one(&mut harness, proposal, supporter, initial_ballot_box);
    }
    let before = fetch_proposal(&harness.svm, &proposal);
    assert!(before.voting);

    let current_epoch = harness.svm.get_sysvar::<solana_clock::Clock>().epoch;
    let expected_snapshot_epoch = current_epoch + SNAPSHOT_EPOCH_EXTENSION;
    let expected_snapshot_slot = expected_snapshot_epoch * SLOTS_PER_EPOCH;
    let expected_start_epoch = expected_snapshot_epoch + 1;
    let expected_end_epoch = expected_start_epoch + VOTING_EPOCHS;
    let expected_consensus_result = consensus_result_pda(expected_snapshot_slot);
    let recovered_ballot_box = ballot_box_pda(expected_snapshot_slot);
    let vote_account = harness.validators[0].vote.pubkey();

    send_ix(
        &mut harness.svm,
        &config_admin,
        &[flush_merkle_root_ix(
            &config_admin.pubkey(),
            proposal,
            vote_account,
            recovered_ballot_box,
            ncn_program_config_pda(),
            harness.global_config,
        )],
    );

    let recovered = fetch_proposal(&harness.svm, &proposal);
    assert_eq!(recovered.snapshot_slot, expected_snapshot_slot);
    assert_eq!(recovered.start_epoch, expected_start_epoch);
    assert_eq!(recovered.end_epoch, expected_end_epoch);
    assert_eq!(
        recovered.consensus_result,
        Some(expected_consensus_result.to_bytes())
    );

    let ballot_box: BallotBox = fetch_ncn_account(&harness.svm, &recovered_ballot_box);
    assert_eq!(ballot_box.snapshot_slot, expected_snapshot_slot);
    assert_eq!(
        ballot_box.vote_expiry_slot,
        expected_start_epoch * SLOTS_PER_EPOCH
    );
    assert!(
        ballot_box
            .vote_expiry_slot
            .saturating_sub(ballot_box.snapshot_slot)
            >= ncn_snapshot::MIN_VOTE_EXPIRY_SLOTS
    );
}
