//! Shared LiteSVM harness for the support/retally integration suites.
//!
//! The harness is parameterized by validator and supporter counts so suites
//! at different scales (150-of-1000, 1500-of-2000, ...) reuse the same setup:
//! `setup_harness` derives the cluster support threshold (in bps) from the
//! two counts, so crossing the threshold always lands exactly on the
//! `supporter_count`-th support.
#![allow(dead_code)]

use {
    borsh::{BorshDeserialize, BorshSerialize},
    litesvm::LiteSVM,
    sha2::{Digest, Sha256},
    solana_account::Account,
    solana_address::Address,
    solana_clock::Clock,
    solana_compute_budget_interface::ComputeBudgetInstruction,
    solana_instruction::{AccountMeta, Instruction},
    solana_instruction_error::InstructionError,
    solana_keypair::Keypair,
    solana_message::Message,
    solana_native_token::LAMPORTS_PER_SOL,
    solana_sdk_ids::{native_loader, system_program, vote},
    solana_signer::Signer,
    solana_transaction::Transaction,
    solana_vote_interface_host::state::{VoteInit, VoteStateV3, VoteStateVersions},
    std::{collections::HashMap, path::PathBuf},
    svmgov_program::GovernanceError,
};

pub const SVMGOV_PROGRAM_ID: Address =
    Address::from_str_const("govYkyQ3ePtGULAtY6V75qjWE8UH4vCUVQ1W4HdCAZU");
pub const NCN_SNAPSHOT_PROGRAM_ID: Address =
    Address::from_str_const("ncnwF8AgynRcdEnGLcprSQNaKvgSMTgk3yPRc8cf9Zf");

pub const STAKE_PER_VALIDATOR: u64 = LAMPORTS_PER_SOL;
pub const SLOTS_PER_EPOCH: u64 = 432_000;
pub const DISCUSSION_EPOCHS: u64 = 1;
pub const MAX_SUPPORT_EPOCHS: u64 = 10;
pub const VOTING_EPOCHS: u64 = 3;
/// Mirrors `constants::MAX_SUPPORTERS_LIMIT` (not exported from the crate).
pub const MAX_SUPPORTERS: u32 = 2_000;
/// Anchor `#[error_code]` offset (`anchor_lang::error::ERROR_CODE_OFFSET`).
pub const ANCHOR_ERROR_CODE_OFFSET: u32 = 6000;

pub fn anchor_custom_error(err: GovernanceError) -> InstructionError {
    InstructionError::Custom(ANCHOR_ERROR_CODE_OFFSET + err as u32)
}

pub fn pk_bytes(a: &Address) -> [u8; 32] {
    a.to_bytes()
}

pub fn anchor_discriminator(namespace: &str, name: &str) -> [u8; 8] {
    let preimage = format!("{namespace}:{name}");
    let hash = Sha256::digest(preimage.as_bytes());
    let mut out = [0u8; 8];
    out.copy_from_slice(&hash[..8]);
    out
}

pub fn read_program() -> Vec<u8> {
    // CARGO_MANIFEST_DIR = programs/svmgov_program
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../target/deploy/svmgov_program.so");
    std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "failed to read {}: {e}. Build with: cargo-build-sbf -p svmgov_program",
            path.display()
        )
    })
}

#[derive(BorshSerialize)]
pub struct GlobalConfigAccount {
    pub admin: [u8; 32],
    pub pending_admin: Option<[u8; 32]>,
    pub max_title_length: u16,
    pub max_description_length: u16,
    pub max_support_epochs: u64,
    pub min_proposal_stake_lamports: u64,
    pub cluster_support_pct_min_bps: u64,
    pub discussion_epochs: u64,
    pub voting_epochs: u64,
    pub snapshot_epoch_extension: u64,
    pub snapshot_slot_offset: i64,
    pub bump: u8,
    pub max_supporters: u32,
}

#[derive(BorshSerialize)]
pub struct ProposalIndexAccount {
    pub current_index: u32,
    pub bump: u8,
}

#[derive(Debug, BorshDeserialize)]
pub struct ProposalAccount {
    pub author: [u8; 32],
    pub title: String,
    pub description: String,
    pub creation_epoch: u64,
    pub start_epoch: u64,
    pub end_epoch: u64,
    pub proposer_stake_weight_bp: u64,
    pub cluster_support_lamports: u64,
    pub for_votes_lamports: u64,
    pub against_votes_lamports: u64,
    pub abstain_votes_lamports: u64,
    pub voting: bool,
    pub finalized: bool,
    pub proposal_bump: u8,
    pub creation_timestamp: i64,
    pub vote_count: u32,
    pub index: u32,
    pub consensus_result: Option<[u8; 32]>,
    pub snapshot_slot: u64,
    pub proposal_seed: u64,
    pub vote_account_pubkey: [u8; 32],
    pub num_supporters: u32,
    /// Not part of the Borsh payload: the entries live at the fixed offset
    /// `Proposal::SUPPORTERS_OFFSET` (= 8 + INIT_SPACE, past any slack left
    /// by shorter-than-max strings / a None consensus_result);
    /// `fetch_proposal` fills this in.
    #[borsh(skip)]
    pub supporters: Vec<[u8; 32]>,
}

pub struct Validator {
    pub identity: Keypair,
    pub vote: Keypair,
}

pub struct Harness {
    pub svm: LiteSVM,
    pub validators: Vec<Validator>,
    /// Total validators in the cluster (each staked `STAKE_PER_VALIDATOR`).
    pub validator_count: usize,
    /// Validators with funded identities + vote accounts, i.e. the number of
    /// supports needed to cross the threshold exactly.
    pub supporter_count: usize,
    /// Threshold written to the global config, derived from the two counts:
    /// `supporter_count / validator_count` in basis points.
    pub cluster_support_pct_min_bps: u64,
    pub global_config: Address,
    pub proposal_index: Address,
    pub program_config: Address,
}

pub fn make_vote_account_data(node: &Address) -> Vec<u8> {
    let vote_init = VoteInit {
        node_pubkey: *node,
        authorized_voter: *node,
        authorized_withdrawer: *node,
        commission: 0,
    };
    let state = VoteStateV3::new(&vote_init, &Clock::default());
    let versioned = VoteStateVersions::V3(Box::new(state));
    let mut data = vec![0u8; VoteStateV3::size_of()];
    VoteStateV3::serialize(&versioned, &mut data).expect("serialize VoteStateV3");
    data
}

pub fn write_anchor_account<T: BorshSerialize>(
    svm: &mut LiteSVM,
    address: Address,
    owner: Address,
    discriminator: [u8; 8],
    data: &T,
) {
    let mut bytes = discriminator.to_vec();
    bytes.extend(borsh::to_vec(data).expect("borsh serialize"));
    let lamports = svm.minimum_balance_for_rent_exemption(bytes.len());
    svm.set_account(
        address,
        Account {
            lamports,
            data: bytes,
            owner,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();
}

pub fn try_send_ix(
    svm: &mut LiteSVM,
    payer: &Keypair,
    ixs: &[Instruction],
) -> Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata> {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new(ixs, Some(&payer.pubkey()));
    let tx = Transaction::new(&[payer], msg, blockhash);
    svm.send_transaction(tx)
}

pub fn send_ix(
    svm: &mut LiteSVM,
    payer: &Keypair,
    ixs: &[Instruction],
) -> litesvm::types::TransactionMetadata {
    try_send_ix(svm, payer, ixs).unwrap_or_else(|e| {
        panic!("tx failed: {:#?}\nlogs: {:#?}", e.err, e.meta.logs);
    })
}

pub fn create_proposal_ix(
    author: &Address,
    proposal: Address,
    proposal_index: Address,
    vote_account: Address,
    global_config: Address,
    seed: u64,
    title: &str,
    description: &str,
) -> Instruction {
    let mut data = anchor_discriminator("global", "create_proposal").to_vec();
    data.extend(seed.to_le_bytes());
    data.extend((title.len() as u32).to_le_bytes());
    data.extend(title.as_bytes());
    data.extend((description.len() as u32).to_le_bytes());
    data.extend(description.as_bytes());

    Instruction {
        program_id: SVMGOV_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*author, true),
            AccountMeta::new(proposal, false),
            AccountMeta::new(proposal_index, false),
            AccountMeta::new_readonly(vote_account, false),
            AccountMeta::new_readonly(global_config, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    }
}

pub fn support_proposal_ix(
    supporter: &Address,
    proposal: Address,
    support: Address,
    vote_account: Address,
    ballot_box: Address,
    program_config: Address,
    global_config: Address,
) -> Instruction {
    Instruction {
        program_id: SVMGOV_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*supporter, true),
            AccountMeta::new(proposal, false),
            AccountMeta::new(support, false),
            AccountMeta::new_readonly(vote_account, false),
            AccountMeta::new(ballot_box, false),
            AccountMeta::new_readonly(NCN_SNAPSHOT_PROGRAM_ID, false),
            AccountMeta::new_readonly(program_config, false),
            AccountMeta::new_readonly(global_config, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: anchor_discriminator("global", "support_proposal").to_vec(),
    }
}

pub fn retally_support_ix(
    caller: &Address,
    proposal: Address,
    ballot_box: Address,
    program_config: Address,
    global_config: Address,
) -> Instruction {
    Instruction {
        program_id: SVMGOV_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*caller, true),
            AccountMeta::new(proposal, false),
            AccountMeta::new(ballot_box, false),
            AccountMeta::new_readonly(NCN_SNAPSHOT_PROGRAM_ID, false),
            AccountMeta::new_readonly(program_config, false),
            AccountMeta::new_readonly(global_config, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: anchor_discriminator("global", "retally_support").to_vec(),
    }
}

pub fn fetch_proposal(svm: &LiteSVM, proposal: &Address) -> ProposalAccount {
    let account = svm.get_account(proposal).expect("proposal account");
    assert!(account.data.len() > 8, "proposal too small");
    // The Borsh payload ends at or before SUPPORTERS_OFFSET; ignore the slack.
    let mut data: &[u8] = &account.data[8..];
    let mut state = ProposalAccount::deserialize(&mut data).expect("deserialize proposal");
    // Supporter entries are not part of the Borsh payload: the account is
    // always sized 8 + INIT_SPACE + 32 * num_supporters, with the entries
    // pinned at the fixed capacity boundary.
    let offset = svmgov_program::Proposal::SUPPORTERS_OFFSET;
    assert_eq!(
        account.data.len(),
        offset + 32 * state.num_supporters as usize,
        "proposal account must be sized 8 + INIT_SPACE + 32 * num_supporters"
    );
    state.supporters = (0..state.num_supporters as usize)
        .map(|i| {
            account.data[offset + 32 * i..offset + 32 * (i + 1)]
                .try_into()
                .unwrap()
        })
        .collect();
    state
}

pub fn expected_snapshot_slot(crossing_epoch: u64) -> u64 {
    // discussion_epochs=1, snapshot_epoch_extension=0, snapshot_slot_offset=0
    (crossing_epoch + DISCUSSION_EPOCHS) * SLOTS_PER_EPOCH
}

pub fn set_clock(svm: &mut LiteSVM, epoch: u64) {
    let mut clock = svm.get_sysvar::<Clock>();
    clock.epoch = epoch;
    // Stay inside the epoch so snapshot_slot (= next epoch start) remains in the future.
    clock.slot = epoch * SLOTS_PER_EPOCH + 1;
    svm.set_sysvar(&clock);
}

pub fn seed_ballot_box(svm: &mut LiteSVM, snapshot_slot: u64) -> Address {
    let (ballot_box, _) = Address::find_program_address(
        &[b"BallotBox", &snapshot_slot.to_le_bytes()],
        &NCN_SNAPSHOT_PROGRAM_ID,
    );
    if svm.get_account(&ballot_box).is_none() {
        svm.set_account(
            ballot_box,
            Account {
                lamports: svm.minimum_balance_for_rent_exemption(1),
                data: vec![1],
                owner: NCN_SNAPSHOT_PROGRAM_ID,
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();
    }
    ballot_box
}

/// Builds a cluster of `validator_count` equally staked validators where the
/// first `supporter_count` have funded identities and real vote accounts, and
/// writes a global config whose support threshold is crossed exactly when all
/// of them support (threshold bps = supporter_count / validator_count).
pub fn setup_harness(
    creation_epoch: u64,
    validator_count: usize,
    supporter_count: usize,
) -> Harness {
    assert!(supporter_count <= validator_count);
    assert!(supporter_count as u32 <= MAX_SUPPORTERS);
    // The threshold must be exactly representable in basis points, otherwise
    // the crossing would not land precisely on the supporter_count-th support.
    assert_eq!(
        supporter_count * 10_000 % validator_count,
        0,
        "supporter_count/validator_count must be an exact bps fraction"
    );
    let cluster_support_pct_min_bps = (supporter_count * 10_000 / validator_count) as u64;

    let mut svm = LiteSVM::new();
    svm.add_program(SVMGOV_PROGRAM_ID, &read_program()).unwrap();
    set_clock(&mut svm, creation_epoch);

    let (global_config, global_bump) =
        Address::find_program_address(&[b"global_config"], &SVMGOV_PROGRAM_ID);
    let (proposal_index, index_bump) =
        Address::find_program_address(&[b"index"], &SVMGOV_PROGRAM_ID);
    let (program_config, _) =
        Address::find_program_address(&[b"ProgramConfig"], &NCN_SNAPSHOT_PROGRAM_ID);

    write_anchor_account(
        &mut svm,
        global_config,
        SVMGOV_PROGRAM_ID,
        anchor_discriminator("account", "GlobalConfig"),
        &GlobalConfigAccount {
            admin: pk_bytes(&Address::new_unique()),
            pending_admin: None,
            max_title_length: 200,
            max_description_length: 500,
            max_support_epochs: MAX_SUPPORT_EPOCHS,
            min_proposal_stake_lamports: 0,
            cluster_support_pct_min_bps,
            discussion_epochs: DISCUSSION_EPOCHS,
            voting_epochs: VOTING_EPOCHS,
            snapshot_epoch_extension: 0,
            snapshot_slot_offset: 0,
            bump: global_bump,
            max_supporters: MAX_SUPPORTERS,
        },
    );
    write_anchor_account(
        &mut svm,
        proposal_index,
        SVMGOV_PROGRAM_ID,
        anchor_discriminator("account", "ProposalIndex"),
        &ProposalIndexAccount {
            current_index: 0,
            bump: index_bump,
        },
    );

    // Stand-ins so constraints pass; non-empty ballot box skips ncn_snapshot CPI.
    svm.set_account(
        NCN_SNAPSHOT_PROGRAM_ID,
        Account {
            lamports: 1,
            data: vec![0],
            owner: native_loader::ID,
            executable: true,
            rent_epoch: 0,
        },
    )
    .unwrap();
    svm.set_account(
        program_config,
        Account {
            lamports: svm.minimum_balance_for_rent_exemption(1),
            data: vec![0],
            owner: NCN_SNAPSHOT_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    let validators: Vec<Validator> = (0..validator_count)
        .map(|_| Validator {
            identity: Keypair::new(),
            vote: Keypair::new(),
        })
        .collect();

    let mut stakes = HashMap::with_capacity(validator_count);
    for v in &validators {
        stakes.insert(v.vote.pubkey(), STAKE_PER_VALIDATOR);
    }
    svm.set_epoch_stakes(stakes).unwrap();

    for v in validators.iter().take(supporter_count) {
        let data = make_vote_account_data(&v.identity.pubkey());
        let lamports = svm.minimum_balance_for_rent_exemption(data.len());
        svm.set_account(
            v.vote.pubkey(),
            Account {
                lamports,
                data,
                owner: vote::ID,
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();
        // Enough lamports for many proposals / reallocs across sub-tests.
        svm.airdrop(&v.identity.pubkey(), 100 * LAMPORTS_PER_SOL)
            .unwrap();
    }

    Harness {
        svm,
        validators,
        validator_count,
        supporter_count,
        cluster_support_pct_min_bps,
        global_config,
        proposal_index,
        program_config,
    }
}

pub fn create_proposal(h: &mut Harness, seed: u64, title: &str) -> Address {
    let author = h.validators[0].identity.insecure_clone();
    let author_vote = h.validators[0].vote.pubkey();
    let (proposal, _) = Address::find_program_address(
        &[b"proposal", &seed.to_le_bytes(), author_vote.as_ref()],
        &SVMGOV_PROGRAM_ID,
    );
    send_ix(
        &mut h.svm,
        &author,
        &[create_proposal_ix(
            &author.pubkey(),
            proposal,
            h.proposal_index,
            author_vote,
            h.global_config,
            seed,
            title,
            "https://github.com/solana-foundation/solana-governance",
        )],
    );
    proposal
}

pub fn try_support_one(
    h: &mut Harness,
    proposal: Address,
    validator_idx: usize,
    ballot_box: Address,
) -> Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata> {
    let identity = h.validators[validator_idx].identity.insecure_clone();
    let vote = h.validators[validator_idx].vote.pubkey();
    let (support, _) = Address::find_program_address(
        &[b"support", proposal.as_ref(), vote.as_ref()],
        &SVMGOV_PROGRAM_ID,
    );
    try_send_ix(
        &mut h.svm,
        &identity,
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
            support_proposal_ix(
                &identity.pubkey(),
                proposal,
                support,
                vote,
                ballot_box,
                h.program_config,
                h.global_config,
            ),
        ],
    )
}

pub fn support_one(h: &mut Harness, proposal: Address, validator_idx: usize, ballot_box: Address) {
    try_support_one(h, proposal, validator_idx, ballot_box).unwrap_or_else(|e| {
        panic!("support failed: {:#?}\nlogs: {:#?}", e.err, e.meta.logs);
    });
}

pub fn try_retally_one(
    h: &mut Harness,
    proposal: Address,
    caller_idx: usize,
    ballot_box: Address,
) -> Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata> {
    let caller = h.validators[caller_idx].identity.insecure_clone();
    try_send_ix(
        &mut h.svm,
        &caller,
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
            retally_support_ix(
                &caller.pubkey(),
                proposal,
                ballot_box,
                h.program_config,
                h.global_config,
            ),
        ],
    )
}

pub fn retally_one(h: &mut Harness, proposal: Address, caller_idx: usize, ballot_box: Address) {
    try_retally_one(h, proposal, caller_idx, ballot_box).unwrap_or_else(|e| {
        panic!("retally failed: {:#?}\nlogs: {:#?}", e.err, e.meta.logs);
    });
}

/// Asserts the proposal crossed the threshold with exactly
/// `h.supporter_count` supporters at `crossing_epoch`.
pub fn assert_threshold_reached(h: &Harness, proposal: &ProposalAccount, crossing_epoch: u64) {
    let expected_support = STAKE_PER_VALIDATOR * h.supporter_count as u64;
    let cluster = STAKE_PER_VALIDATOR * h.validator_count as u64;
    assert_eq!(proposal.cluster_support_lamports, expected_support);
    assert_eq!(proposal.supporters.len(), h.supporter_count);
    assert!(
        proposal.voting,
        "expected voting activated at {} bps support",
        h.cluster_support_pct_min_bps
    );
    assert_eq!(
        expected_support * 10_000 / cluster,
        h.cluster_support_pct_min_bps
    );
    assert_eq!(
        proposal.snapshot_slot,
        expected_snapshot_slot(crossing_epoch)
    );
}
