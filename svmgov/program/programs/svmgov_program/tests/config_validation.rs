//! Public-instruction coverage for snapshot-slot offset validation.

mod common;

use {
    anchor_lang::{
        solana_program::bpf_loader_upgradeable::UpgradeableLoaderState, AnchorSerialize,
    },
    common::*,
    litesvm::LiteSVM,
    solana_address::Address,
    solana_instruction::{AccountMeta, Instruction},
    solana_keypair::Keypair,
    solana_native_token::LAMPORTS_PER_SOL,
    solana_sdk_ids::{bpf_loader_upgradeable, system_program},
    solana_signer::Signer,
    solana_transaction_error::TransactionError,
    svmgov_program::GovernanceError,
};

fn invalid_snapshot_offset() -> i64 {
    (SLOTS_PER_EPOCH - ncn_snapshot::MIN_VOTE_EXPIRY_SLOTS + 1) as i64
}

fn assert_invalid_offset(
    result: Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata>,
) {
    let error = result.expect_err("invalid snapshot offset must be rejected");
    assert_eq!(
        error.err,
        TransactionError::InstructionError(
            0,
            anchor_custom_error(GovernanceError::InvalidSnapshotSlotOffset)
        ),
        "logs: {:#?}",
        error.meta.logs
    );
}

fn initialize_config_ix(
    admin: &Address,
    global_config: Address,
    program_data: Address,
    snapshot_slot_offset: i64,
) -> Instruction {
    let mut data = anchor_discriminator("global", "initialize_config").to_vec();
    200u16.serialize(&mut data).unwrap();
    500u16.serialize(&mut data).unwrap();
    10u64.serialize(&mut data).unwrap();
    0u64.serialize(&mut data).unwrap();
    5_000u64.serialize(&mut data).unwrap();
    1u64.serialize(&mut data).unwrap();
    3u64.serialize(&mut data).unwrap();
    0u64.serialize(&mut data).unwrap();
    snapshot_slot_offset.serialize(&mut data).unwrap();
    MAX_SUPPORTERS.serialize(&mut data).unwrap();
    Some(true).serialize(&mut data).unwrap();

    Instruction {
        program_id: SVMGOV_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*admin, true),
            AccountMeta::new(global_config, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(SVMGOV_PROGRAM_ID, false),
            AccountMeta::new_readonly(program_data, false),
        ],
        data,
    }
}

fn set_upgrade_authority(svm: &mut LiteSVM, authority: &Address) -> Address {
    let program_data =
        Address::find_program_address(&[SVMGOV_PROGRAM_ID.as_ref()], &bpf_loader_upgradeable::ID).0;
    let mut account = svm
        .get_account(&program_data)
        .expect("LiteSVM must create upgradeable program data");

    let state = UpgradeableLoaderState::ProgramData {
        slot: 0,
        upgrade_authority_address: Some(to_pubkey(authority)),
    };
    let metadata = bincode::serialize(&state).expect("serialize upgradeable loader state");
    account.data[..metadata.len()].copy_from_slice(&metadata);
    svm.set_account(program_data, account).unwrap();
    program_data
}

#[test]
fn initialize_config_rejects_invalid_snapshot_offset() {
    let mut svm = LiteSVM::new();
    svm.add_program(SVMGOV_PROGRAM_ID, &read_program()).unwrap();
    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
    let program_data = set_upgrade_authority(&mut svm, &admin.pubkey());
    let global_config = Address::find_program_address(&[b"global_config"], &SVMGOV_PROGRAM_ID).0;

    assert_invalid_offset(try_send_ix(
        &mut svm,
        &admin,
        &[initialize_config_ix(
            &admin.pubkey(),
            global_config,
            program_data,
            invalid_snapshot_offset(),
        )],
    ));
    assert!(
        svm.get_account(&global_config)
            .is_none_or(|account| account.lamports == 0),
        "failed initialization must not persist the config account"
    );
}

#[test]
fn update_config_rejects_invalid_snapshot_offset_without_mutation() {
    let mut harness = setup_harness(1, 1, 1);
    let before = harness
        .svm
        .get_account(&harness.global_config)
        .unwrap()
        .data;
    let instruction = update_global_config_ix(
        &harness.config_admin.pubkey(),
        harness.global_config,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(invalid_snapshot_offset()),
        None,
        None,
    );
    let admin = harness.config_admin.insecure_clone();

    assert_invalid_offset(try_send_ix(&mut harness.svm, &admin, &[instruction]));
    let after = harness
        .svm
        .get_account(&harness.global_config)
        .unwrap()
        .data;
    assert_eq!(after, before, "rejected update must leave config unchanged");
}
