//! Migration of `GlobalConfig` from the deployed `max_supporters` layout to
//! the layout that appends `new_proposals_allowed`.

use {
    borsh::{BorshDeserialize, BorshSerialize},
    litesvm::LiteSVM,
    sha2::{Digest, Sha256},
    solana_account::Account,
    solana_address::Address,
    solana_instruction::{AccountMeta, Instruction},
    solana_keypair::Keypair,
    solana_message::Message,
    solana_native_token::LAMPORTS_PER_SOL,
    solana_sdk_ids::system_program,
    solana_signer::Signer,
    solana_transaction::Transaction,
    std::path::PathBuf,
};

const SVMGOV_PROGRAM_ID: Address =
    Address::from_str_const("govYkyQ3ePtGULAtY6V75qjWE8UH4vCUVQ1W4HdCAZU");

/// Discriminator plus the deployed layout, which does not have
/// `new_proposals_allowed`.
const PREVIOUS_SPACE: usize = 8 + 130;
/// Discriminator plus the current layout, which appends one boolean.
const CURRENT_SPACE: usize = PREVIOUS_SPACE + 1;

const MAX_TITLE_LENGTH: u16 = 200;
const MAX_DESCRIPTION_LENGTH: u16 = 500;
const MAX_SUPPORT_EPOCHS: u64 = 10;
const MIN_PROPOSAL_STAKE_LAMPORTS: u64 = LAMPORTS_PER_SOL;
const CLUSTER_SUPPORT_PCT_MIN_BPS: u64 = 1_500;
const DISCUSSION_EPOCHS: u64 = 1;
const VOTING_EPOCHS: u64 = 3;
const SNAPSHOT_EPOCH_EXTENSION: u64 = 0;
const SNAPSHOT_SLOT_OFFSET: i64 = 0;
const MAX_SUPPORTERS: u32 = 500;

fn anchor_discriminator(namespace: &str, name: &str) -> [u8; 8] {
    let preimage = format!("{namespace}:{name}");
    let hash = Sha256::digest(preimage.as_bytes());
    let mut out = [0u8; 8];
    out.copy_from_slice(&hash[..8]);
    out
}

fn read_program() -> Vec<u8> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../../../target/deploy/svmgov_program.so");
    std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "failed to read {}: {e}. Build with: cargo-build-sbf -p svmgov_program",
            path.display()
        )
    })
}

/// `GlobalConfig` as serialized before the `new_proposals_allowed` field was added.
#[derive(BorshSerialize)]
struct PreviousGlobalConfig {
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

#[derive(BorshDeserialize)]
struct CurrentGlobalConfig {
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
    new_proposals_allowed: bool,
}

/// Borsh-encoded `update_config` arguments, in instruction declaration order.
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
    new_proposals_allowed: Option<bool>,
}

struct Harness {
    svm: LiteSVM,
    admin: Keypair,
    global_config: Address,
}

fn setup_harness() -> Harness {
    let mut svm = LiteSVM::new();
    svm.add_program(SVMGOV_PROGRAM_ID, &read_program()).unwrap();

    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
    let (global_config, bump) =
        Address::find_program_address(&[b"global_config"], &SVMGOV_PROGRAM_ID);

    let previous_config = PreviousGlobalConfig {
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
        max_supporters: MAX_SUPPORTERS,
    };
    let mut data = anchor_discriminator("account", "GlobalConfig").to_vec();
    data.extend(borsh::to_vec(&previous_config).expect("serialize prior config"));
    data.resize(PREVIOUS_SPACE, 0);

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

fn update(h: &mut Harness, args: &UpdateConfigArgs) {
    let mut data = anchor_discriminator("global", "update_config").to_vec();
    data.extend(borsh::to_vec(args).expect("serialize update arguments"));
    let instruction = Instruction {
        program_id: SVMGOV_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(h.admin.pubkey(), true),
            AccountMeta::new(h.global_config, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    };
    let transaction = Transaction::new(
        &[&h.admin],
        Message::new(&[instruction], Some(&h.admin.pubkey())),
        h.svm.latest_blockhash(),
    );
    h.svm.send_transaction(transaction).unwrap();
}

fn fetch_config(h: &Harness) -> CurrentGlobalConfig {
    let account = h.svm.get_account(&h.global_config).unwrap();
    assert_eq!(account.data.len(), CURRENT_SPACE);
    assert_eq!(
        account.data[..8],
        anchor_discriminator("account", "GlobalConfig")
    );
    CurrentGlobalConfig::deserialize(&mut &account.data[8..]).unwrap()
}

#[test_log::test]
fn migration_preserves_max_supporters_and_applies_pause_flag() {
    let mut h = setup_harness();
    let admin_before = h.svm.get_balance(&h.admin.pubkey()).unwrap();

    update(
        &mut h,
        &UpdateConfigArgs {
            new_proposals_allowed: Some(false),
            ..Default::default()
        },
    );

    let config = fetch_config(&h);
    assert_eq!(config.admin, h.admin.pubkey().to_bytes());
    assert_eq!(config.pending_admin, None);
    let (_, bump) = Address::find_program_address(&[b"global_config"], &SVMGOV_PROGRAM_ID);
    assert_eq!(config.bump, bump);
    assert_eq!(config.max_supporters, MAX_SUPPORTERS);
    assert!(!config.new_proposals_allowed);
    assert_eq!(config.max_title_length, MAX_TITLE_LENGTH);
    assert_eq!(config.max_description_length, MAX_DESCRIPTION_LENGTH);
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
    assert!(h.svm.get_balance(&h.admin.pubkey()).unwrap() < admin_before);
}

#[test_log::test]
fn migration_defaults_to_allowing_proposals() {
    let mut h = setup_harness();

    update(&mut h, &UpdateConfigArgs::default());

    let config = fetch_config(&h);
    assert_eq!(config.max_supporters, MAX_SUPPORTERS);
    assert!(config.new_proposals_allowed);
}
