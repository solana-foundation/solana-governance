//! Migration of a pre-`max_supporters` `GlobalConfig` to the current layout.
//!
//! Accounts initialized before the `max_supporters` field existed are 4 bytes
//! smaller than `GlobalConfig::INIT_SPACE + 8`. `update_config` detects this,
//! reallocs the account (admin tops up rent) and requires a valid
//! `max_supporters` value to be provided in the same call. `initialize_config`
//! now always writes the new layout, so the legacy account is seeded directly
//! via `set_account`.

use {
    borsh::{BorshDeserialize, BorshSerialize},
    litesvm::LiteSVM,
    sha2::{Digest, Sha256},
    solana_account::Account,
    solana_address::Address,
    solana_instruction::{AccountMeta, Instruction},
    solana_instruction_error::InstructionError,
    solana_keypair::Keypair,
    solana_message::Message,
    solana_native_token::LAMPORTS_PER_SOL,
    solana_sdk_ids::system_program,
    solana_signer::Signer,
    solana_transaction::Transaction,
    solana_transaction_error::TransactionError,
    std::path::PathBuf,
    svmgov_program::GovernanceError,
};

const SVMGOV_PROGRAM_ID: Address =
    Address::from_str_const("govYkyQ3ePtGULAtY6V75qjWE8UH4vCUVQ1W4HdCAZU");

/// Mirrors `constants::MAX_SUPPORTERS_LIMIT` (not exported from the crate).
const MAX_SUPPORTERS_LIMIT: u32 = 2_000;
/// Anchor `#[error_code]` offset (`anchor_lang::error::ERROR_CODE_OFFSET`).
const ANCHOR_ERROR_CODE_OFFSET: u32 = 6000;

/// Discriminator + legacy `INIT_SPACE` (no `max_supporters`). Anchor allocates
/// `INIT_SPACE`, which reserves the full 33 bytes for `pending_admin`, so a
/// real legacy account is padded past its borsh payload when the option is `None`.
const LEGACY_SPACE: usize = 8 + 126;
/// Discriminator + `GlobalConfig::INIT_SPACE` (adds the `max_supporters: u32`).
const MIGRATED_SPACE: usize = LEGACY_SPACE + 4;

const MAX_TITLE_LENGTH: u16 = 200;
const MAX_DESCRIPTION_LENGTH: u16 = 500;
const MAX_SUPPORT_EPOCHS: u64 = 10;
const MIN_PROPOSAL_STAKE_LAMPORTS: u64 = LAMPORTS_PER_SOL;
const CLUSTER_SUPPORT_PCT_MIN_BPS: u64 = 1_500;
const DISCUSSION_EPOCHS: u64 = 1;
const VOTING_EPOCHS: u64 = 3;
const SNAPSHOT_EPOCH_EXTENSION: u64 = 0;
const SNAPSHOT_SLOT_OFFSET: i64 = 0;

fn anchor_custom_error(err: GovernanceError) -> InstructionError {
    InstructionError::Custom(ANCHOR_ERROR_CODE_OFFSET + err as u32)
}

fn anchor_discriminator(namespace: &str, name: &str) -> [u8; 8] {
    let preimage = format!("{namespace}:{name}");
    let hash = Sha256::digest(preimage.as_bytes());
    let mut out = [0u8; 8];
    out.copy_from_slice(&hash[..8]);
    out
}

fn read_program() -> Vec<u8> {
    // CARGO_MANIFEST_DIR = programs/svmgov_program
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../../../target/deploy/svmgov_program.so");
    std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "failed to read {}: {e}. Build with: cargo-build-sbf -p svmgov_program",
            path.display()
        )
    })
}

/// `GlobalConfig` as serialized before the `max_supporters` field was added.
#[derive(BorshSerialize)]
struct LegacyGlobalConfig {
    admin: [u8; 32],
    pending_admin: Option<[u8; 32]>,
    max_title_length: u16,
    max_description_length: u16,
    max_support_epochs: u64,
    min_proposal_stake_lamports: u64,
    cluster_support_pct_min_bps: u64,
    discussion_epochs: u64,
    voting_epochs: u64,
    snapshot_epoch_extension: u64,
    snapshot_slot_offset: i64,
    bump: u8,
}

/// Current on-chain `GlobalConfig` layout.
#[derive(Debug, BorshDeserialize, PartialEq)]
struct GlobalConfigAccount {
    admin: [u8; 32],
    pending_admin: Option<[u8; 32]>,
    max_title_length: u16,
    max_description_length: u16,
    max_support_epochs: u64,
    min_proposal_stake_lamports: u64,
    cluster_support_pct_min_bps: u64,
    discussion_epochs: u64,
    voting_epochs: u64,
    snapshot_epoch_extension: u64,
    snapshot_slot_offset: i64,
    bump: u8,
    max_supporters: u32,
}

/// Borsh-encoded `update_config` args, in declaration order.
#[derive(Default, BorshSerialize)]
struct UpdateConfigArgs {
    max_title_length: Option<u16>,
    max_description_length: Option<u16>,
    max_support_epochs: Option<u64>,
    min_proposal_stake_lamports: Option<u64>,
    cluster_support_pct_min_bps: Option<u64>,
    discussion_epochs: Option<u64>,
    voting_epochs: Option<u64>,
    snapshot_epoch_extension: Option<u64>,
    snapshot_slot_offset: Option<i64>,
    max_supporters: Option<u32>,
}

struct Harness {
    svm: LiteSVM,
    admin: Keypair,
    global_config: Address,
}

fn update_config_ix(admin: &Address, global_config: Address, args: &UpdateConfigArgs) -> Instruction {
    let mut data = anchor_discriminator("global", "update_config").to_vec();
    data.extend(borsh::to_vec(args).expect("borsh serialize args"));
    Instruction {
        program_id: SVMGOV_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*admin, true),
            AccountMeta::new(global_config, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    }
}

fn try_update(
    h: &mut Harness,
    signer: &Keypair,
    args: &UpdateConfigArgs,
) -> Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata> {
    let ix = update_config_ix(&signer.pubkey(), h.global_config, args);
    let blockhash = h.svm.latest_blockhash();
    let msg = Message::new(&[ix], Some(&signer.pubkey()));
    let tx = Transaction::new(&[signer], msg, blockhash);
    h.svm.send_transaction(tx)
}

fn update(h: &mut Harness, args: &UpdateConfigArgs) {
    let admin = h.admin.insecure_clone();
    try_update(h, &admin, args).unwrap_or_else(|e| {
        panic!("update_config failed: {:#?}\nlogs: {:#?}", e.err, e.meta.logs);
    });
}

fn assert_update_fails_with(h: &mut Harness, args: &UpdateConfigArgs, expected: GovernanceError) {
    let admin = h.admin.insecure_clone();
    let err = try_update(h, &admin, args).expect_err("update_config should fail");
    assert_eq!(
        err.err,
        TransactionError::InstructionError(0, anchor_custom_error(expected)),
        "logs: {:#?}",
        err.meta.logs
    );
}

fn fetch_config(h: &Harness) -> GlobalConfigAccount {
    let account = raw_config(h);
    assert_eq!(
        account.data[..8],
        anchor_discriminator("account", "GlobalConfig"),
        "discriminator must survive updates"
    );
    let mut data: &[u8] = &account.data[8..];
    GlobalConfigAccount::deserialize(&mut data).expect("deserialize GlobalConfig")
}

fn raw_config(h: &Harness) -> Account {
    h.svm.get_account(&h.global_config).expect("global_config account")
}

/// Seeds a legacy-layout (pre-`max_supporters`) `GlobalConfig`, rent-exempt for
/// its own size only, so migration must both realloc and top up rent.
fn setup_harness() -> Harness {
    let mut svm = LiteSVM::new();
    svm.add_program(SVMGOV_PROGRAM_ID, &read_program()).unwrap();

    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

    let (global_config, bump) =
        Address::find_program_address(&[b"global_config"], &SVMGOV_PROGRAM_ID);

    let legacy = LegacyGlobalConfig {
        admin: admin.pubkey().to_bytes(),
        pending_admin: None,
        max_title_length: MAX_TITLE_LENGTH,
        max_description_length: MAX_DESCRIPTION_LENGTH,
        max_support_epochs: MAX_SUPPORT_EPOCHS,
        min_proposal_stake_lamports: MIN_PROPOSAL_STAKE_LAMPORTS,
        cluster_support_pct_min_bps: CLUSTER_SUPPORT_PCT_MIN_BPS,
        discussion_epochs: DISCUSSION_EPOCHS,
        voting_epochs: VOTING_EPOCHS,
        snapshot_epoch_extension: SNAPSHOT_EPOCH_EXTENSION,
        snapshot_slot_offset: SNAPSHOT_SLOT_OFFSET,
        bump,
    };
    let mut data = anchor_discriminator("account", "GlobalConfig").to_vec();
    data.extend(borsh::to_vec(&legacy).expect("borsh serialize legacy config"));
    assert!(data.len() <= LEGACY_SPACE, "legacy layout drifted");
    // Pad to the allocation Anchor's old `init` made (INIT_SPACE + 8).
    data.resize(LEGACY_SPACE, 0);

    let lamports = svm.minimum_balance_for_rent_exemption(data.len());
    svm.set_account(
        global_config,
        Account {
            lamports,
            data,
            owner: SVMGOV_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    Harness {
        svm,
        admin,
        global_config,
    }
}

/// Asserts every field the update did not touch still holds its seeded value.
fn assert_untouched_fields(config: &GlobalConfigAccount, h: &Harness, bump_expected: bool) {
    assert_eq!(config.admin, h.admin.pubkey().to_bytes());
    assert_eq!(config.pending_admin, None);
    assert_eq!(config.max_support_epochs, MAX_SUPPORT_EPOCHS);
    assert_eq!(
        config.min_proposal_stake_lamports,
        MIN_PROPOSAL_STAKE_LAMPORTS
    );
    assert_eq!(
        config.cluster_support_pct_min_bps,
        CLUSTER_SUPPORT_PCT_MIN_BPS
    );
    assert_eq!(config.discussion_epochs, DISCUSSION_EPOCHS);
    assert_eq!(config.voting_epochs, VOTING_EPOCHS);
    assert_eq!(config.snapshot_epoch_extension, SNAPSHOT_EPOCH_EXTENSION);
    assert_eq!(config.snapshot_slot_offset, SNAPSHOT_SLOT_OFFSET);
    let (_, bump) = Address::find_program_address(&[b"global_config"], &SVMGOV_PROGRAM_ID);
    if bump_expected {
        assert_eq!(config.bump, bump);
    }
}

/// A legacy account is migrated in place: resized to the new layout, rent
/// topped up by the admin, `max_supporters` set, every other field preserved.
#[test_log::test]
fn migrate_legacy_config_succeeds_with_valid_max_supporters() {
    let mut h = setup_harness();
    assert_eq!(raw_config(&h).data.len(), LEGACY_SPACE);
    let admin_before = h.svm.get_balance(&h.admin.pubkey()).unwrap();

    update(
        &mut h,
        &UpdateConfigArgs {
            max_supporters: Some(MAX_SUPPORTERS_LIMIT),
            ..Default::default()
        },
    );

    let account = raw_config(&h);
    assert_eq!(account.data.len(), MIGRATED_SPACE);
    let min_rent = h.svm.minimum_balance_for_rent_exemption(MIGRATED_SPACE);
    assert!(
        account.lamports >= min_rent,
        "migrated account must stay rent-exempt"
    );
    assert!(
        h.svm.get_balance(&h.admin.pubkey()).unwrap() < admin_before,
        "admin pays the rent top-up (and fee)"
    );

    let config = fetch_config(&h);
    assert_eq!(config.max_supporters, MAX_SUPPORTERS_LIMIT);
    assert_eq!(config.max_title_length, MAX_TITLE_LENGTH);
    assert_eq!(config.max_description_length, MAX_DESCRIPTION_LENGTH);
    assert_untouched_fields(&config, &h, true);
}

/// Migration can bundle other field updates in the same instruction.
#[test_log::test]
fn migrate_legacy_config_applies_other_updates_in_same_call() {
    let mut h = setup_harness();

    update(
        &mut h,
        &UpdateConfigArgs {
            max_title_length: Some(120),
            voting_epochs: Some(5),
            max_supporters: Some(1),
            ..Default::default()
        },
    );

    assert_eq!(raw_config(&h).data.len(), MIGRATED_SPACE);
    let config = fetch_config(&h);
    assert_eq!(config.max_supporters, 1);
    assert_eq!(config.max_title_length, 120);
    assert_eq!(config.voting_epochs, 5);
    assert_eq!(config.max_description_length, MAX_DESCRIPTION_LENGTH);
}

/// Migrating without providing `max_supporters` must fail: the resized account
/// would otherwise hold a zeroed (invalid) value.
#[test_log::test]
fn migrate_legacy_config_rejects_missing_max_supporters() {
    let mut h = setup_harness();

    assert_update_fails_with(
        &mut h,
        &UpdateConfigArgs::default(),
        GovernanceError::InvalidMaxSupporters,
    );

    // Failed tx rolls back the realloc: still the legacy layout.
    assert_eq!(raw_config(&h).data.len(), LEGACY_SPACE);
}

/// Migration must reject `max_supporters` values outside `1..=MAX_SUPPORTERS_LIMIT`.
#[test_log::test]
fn migrate_legacy_config_rejects_invalid_max_supporters_values() {
    for invalid in [0, MAX_SUPPORTERS_LIMIT + 1, u32::MAX] {
        let mut h = setup_harness();

        assert_update_fails_with(
            &mut h,
            &UpdateConfigArgs {
                max_supporters: Some(invalid),
                ..Default::default()
            },
            GovernanceError::InvalidMaxSupporters,
        );

        assert_eq!(
            raw_config(&h).data.len(),
            LEGACY_SPACE,
            "max_supporters={invalid}: failed migration must not resize"
        );
    }
}

/// Migration bundled with another invalid field fails atomically — the account
/// must not end up resized or partially updated.
#[test_log::test]
fn migrate_legacy_config_rolls_back_when_other_field_invalid() {
    let mut h = setup_harness();

    assert_update_fails_with(
        &mut h,
        &UpdateConfigArgs {
            // Above MAX_TITLE_ACCOUNT_SIZE (200).
            max_title_length: Some(201),
            max_supporters: Some(100),
            ..Default::default()
        },
        GovernanceError::InvalidMaxTitleLength,
    );

    assert_eq!(raw_config(&h).data.len(), LEGACY_SPACE);
}

/// Only the stored admin may migrate; the realloc attempt by a stranger rolls back.
#[test_log::test]
fn migrate_legacy_config_rejects_non_admin() {
    let mut h = setup_harness();
    let stranger = Keypair::new();
    h.svm.airdrop(&stranger.pubkey(), LAMPORTS_PER_SOL).unwrap();

    let err = try_update(
        &mut h,
        &stranger,
        &UpdateConfigArgs {
            max_supporters: Some(100),
            ..Default::default()
        },
    )
    .expect_err("non-admin migration should fail");
    assert_eq!(
        err.err,
        TransactionError::InstructionError(
            0,
            anchor_custom_error(GovernanceError::UnauthorizedAdmin)
        )
    );

    assert_eq!(raw_config(&h).data.len(), LEGACY_SPACE);
}

/// Updates after migration are plain updates: no further resize, no extra rent,
/// values (including `max_supporters`) editable, invalid values still rejected.
#[test_log::test]
fn updates_after_migration_keep_account_size_stable() {
    let mut h = setup_harness();

    update(
        &mut h,
        &UpdateConfigArgs {
            max_supporters: Some(500),
            ..Default::default()
        },
    );
    let migrated = raw_config(&h);
    assert_eq!(migrated.data.len(), MIGRATED_SPACE);

    // No-op update: nothing changes, size included.
    update(&mut h, &UpdateConfigArgs::default());
    assert_eq!(fetch_config(&h).max_supporters, 500);

    // Regular field update.
    update(
        &mut h,
        &UpdateConfigArgs {
            max_description_length: Some(300),
            ..Default::default()
        },
    );

    // max_supporters itself stays updatable post-migration.
    update(
        &mut h,
        &UpdateConfigArgs {
            max_supporters: Some(750),
            ..Default::default()
        },
    );

    // Invalid max_supporters on an already-migrated account still fails.
    assert_update_fails_with(
        &mut h,
        &UpdateConfigArgs {
            max_supporters: Some(MAX_SUPPORTERS_LIMIT + 1),
            ..Default::default()
        },
        GovernanceError::InvalidMaxSupporters,
    );

    let account = raw_config(&h);
    assert_eq!(
        account.data.len(),
        MIGRATED_SPACE,
        "further updates must not resize the account"
    );
    assert_eq!(
        account.lamports, migrated.lamports,
        "further updates must not move lamports"
    );

    let config = fetch_config(&h);
    assert_eq!(config.max_supporters, 750);
    assert_eq!(config.max_description_length, 300);
    assert_eq!(config.max_title_length, MAX_TITLE_LENGTH);
    assert_untouched_fields(&config, &h, true);
}
