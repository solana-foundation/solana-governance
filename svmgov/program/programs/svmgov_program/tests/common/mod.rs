//! Shared LiteSVM harness for the support/retally integration suites.
//!
//! The harness is parameterized by validator and supporter counts so suites
//! at different scales (150-of-1000, 1500-of-2000, ...) reuse the same setup:
//! `setup_harness` derives the cluster support threshold (in bps) from the
//! two counts, so crossing the threshold always lands exactly on the
//! `supporter_count`-th support.
#![allow(dead_code)]

use {
    anchor_lang::{prelude::Pubkey as AnchorPubkey, AnchorSerialize, Discriminator},
    borsh::{BorshDeserialize, BorshSerialize},
    litesvm::LiteSVM,
    ncn_merkle_tree::{get_proof, MerkleTree},
    ncn_snapshot::{Ballot, ConsensusResult, MetaMerkleLeaf, MetaMerkleProof, StakeMerkleLeaf},
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
    solana_sdk_ids::{system_program, vote},
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

/// Converts the test-side `Address` (solana-address) into the anchor-side
/// `Pubkey` used by the ncn-snapshot types. Same 32 bytes, different crates.
pub fn to_pubkey(a: &Address) -> AnchorPubkey {
    AnchorPubkey::new_from_array(a.to_bytes())
}

pub fn anchor_discriminator(namespace: &str, name: &str) -> [u8; 8] {
    let preimage = format!("{namespace}:{name}");
    let hash = Sha256::digest(preimage.as_bytes());
    let mut out = [0u8; 8];
    out.copy_from_slice(&hash[..8]);
    out
}

pub fn read_program() -> Vec<u8> {
    // CARGO_MANIFEST_DIR = svmgov/program/programs/svmgov_program; the
    // program builds through the repo-root workspace, so its artifact lives
    // in the root target dir.
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../../../target/deploy/svmgov_program.so");
    std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "failed to read {}: {e}. Build it from the repo root with: \
             cargo-build-sbf --manifest-path \
             svmgov/program/programs/svmgov_program/Cargo.toml -- --locked",
            path.display()
        )
    })
}

pub fn read_ncn_program() -> Vec<u8> {
    // CARGO_MANIFEST_DIR = svmgov/program/programs/svmgov_program; the ncn
    // program builds through the repo-root workspace, so its artifact lives
    // in the root target dir.
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../../../target/deploy/ncn_snapshot.so");
    std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "failed to read {}: {e}. Build it from the repo root with: \
             cargo-build-sbf --manifest-path \
             ncn/programs/ncn-snapshot/Cargo.toml -- --locked",
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
    pub support_threshold: u64,
    pub last_support_epoch: u64,
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

pub fn write_account_bytes(svm: &mut LiteSVM, address: Address, owner: Address, bytes: Vec<u8>) {
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

pub fn write_anchor_account<T: BorshSerialize>(
    svm: &mut LiteSVM,
    address: Address,
    owner: Address,
    discriminator: &[u8],
    data: &T,
) {
    let mut bytes = discriminator.to_vec();
    bytes.extend(borsh::to_vec(data).expect("borsh serialize"));
    write_account_bytes(svm, address, owner, bytes);
}

/// Writes an ncn-snapshot account. The ncn types serialize through anchor's
/// own borsh re-export (0.10), a different crate instance from the harness's
/// borsh 1.x dev-dependency, so they need their own writer.
pub fn write_ncn_account<T: AnchorSerialize>(
    svm: &mut LiteSVM,
    address: Address,
    discriminator: &[u8],
    data: &T,
) {
    let mut bytes = discriminator.to_vec();
    bytes.extend(data.try_to_vec().expect("anchor serialize"));
    write_account_bytes(svm, address, NCN_SNAPSHOT_PROGRAM_ID, bytes);
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
        &anchor_discriminator("account", "GlobalConfig"),
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
        &anchor_discriminator("account", "ProposalIndex"),
        &ProposalIndexAccount {
            current_index: 0,
            bump: index_bump,
        },
    );

    // Load the real ncn-snapshot program: the voting suites CPI into its
    // verify_merkle_proof instruction. The support suites never invoke it (a
    // pre-seeded non-empty ballot box skips the init_ballot_box CPI).
    svm.add_program(NCN_SNAPSHOT_PROGRAM_ID, &read_ncn_program())
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

// ---------------------------------------------------------------------------
// Voting scaffolding: merkle snapshot fabrication + vote instruction builders.
//
// The voting suites need a ConsensusResult and per-validator MetaMerkleProof
// accounts that the real ncn-snapshot program will accept. Leaf hashing and
// account layouts come straight from the `ncn_snapshot` crate (already a
// dependency of the program), and the trees are built with the production
// `ncn-merkle-tree` builder — the same one the ncn CLI uses to generate
// snapshots.
// ---------------------------------------------------------------------------

/// Builds the merkle tree over pre-hashed leaf contents with the production
/// builder and returns `(root, proof-per-leaf)`. Each proof is additionally
/// cross-checked against the on-chain `ncn_snapshot::merkle_helper`, so a
/// builder/verifier disagreement fails here rather than deep inside a CPI.
pub fn build_merkle(leaf_contents: &[[u8; 32]]) -> ([u8; 32], Vec<Vec<[u8; 32]>>) {
    let tree = MerkleTree::new(leaf_contents, true);
    let root = tree.get_root().expect("merkle root").to_bytes();
    let proofs: Vec<Vec<[u8; 32]>> = (0..leaf_contents.len())
        .map(|index| get_proof(&tree, index))
        .collect();
    for (content, proof) in leaf_contents.iter().zip(&proofs) {
        ncn_snapshot::merkle_helper::verify_helper(content, proof, root.into())
            .expect("built merkle proof must verify with the on-chain helper");
    }
    (root, proofs)
}

// Borsh mirrors of the svmgov accounts the voting suites assert on.

#[derive(Debug, BorshDeserialize)]
pub struct VoteAccountState {
    pub validator: [u8; 32],
    pub proposal: [u8; 32],
    pub for_votes_bp: u64,
    pub against_votes_bp: u64,
    pub abstain_votes_bp: u64,
    pub for_votes_lamports: u64,
    pub against_votes_lamports: u64,
    pub abstain_votes_lamports: u64,
    pub stake: u64,
    pub override_lamports: u64,
    pub vote_timestamp: i64,
    pub bump: u8,
}

#[derive(Debug, BorshDeserialize)]
pub struct VoteOverrideCacheState {
    pub validator: [u8; 32],
    pub proposal: [u8; 32],
    pub vote_account_validator: [u8; 32],
    pub for_votes_bp: u64,
    pub against_votes_bp: u64,
    pub abstain_votes_bp: u64,
    pub for_votes_lamports: u64,
    pub against_votes_lamports: u64,
    pub abstain_votes_lamports: u64,
    pub total_stake: u64,
    pub bump: u8,
}

pub fn vote_pda(proposal: &Address, spl_vote_account: &Address) -> Address {
    Address::find_program_address(
        &[b"vote", proposal.as_ref(), spl_vote_account.as_ref()],
        &SVMGOV_PROGRAM_ID,
    )
    .0
}

pub fn vote_override_cache_pda(proposal: &Address, validator_vote: &Address) -> Address {
    Address::find_program_address(
        &[
            b"vote_override_cache",
            proposal.as_ref(),
            validator_vote.as_ref(),
        ],
        &SVMGOV_PROGRAM_ID,
    )
    .0
}

pub fn vote_override_pda(
    proposal: &Address,
    stake_account: &Address,
    validator_vote: &Address,
) -> Address {
    Address::find_program_address(
        &[
            b"vote_override",
            proposal.as_ref(),
            stake_account.as_ref(),
            validator_vote.as_ref(),
        ],
        &SVMGOV_PROGRAM_ID,
    )
    .0
}

pub fn consensus_result_pda(snapshot_slot: u64) -> Address {
    Address::find_program_address(
        &[b"ConsensusResult", &snapshot_slot.to_le_bytes()],
        &NCN_SNAPSHOT_PROGRAM_ID,
    )
    .0
}

pub fn meta_merkle_proof_pda(consensus_result: &Address, vote_account: &Address) -> Address {
    Address::find_program_address(
        &[
            b"MetaMerkleProof",
            consensus_result.as_ref(),
            vote_account.as_ref(),
        ],
        &NCN_SNAPSHOT_PROGRAM_ID,
    )
    .0
}

/// One delegator (stake account) under a snapshot validator.
pub struct DelegatorCtx {
    pub wallet: Keypair,
    pub stake_account: Address,
    pub stake: u64,
    pub stake_proof: Vec<[u8; 32]>,
}

/// One validator present in the fabricated snapshot.
pub struct SnapshotValidator {
    /// Index into `h.validators`.
    pub idx: usize,
    pub active_stake: u64,
    pub delegators: Vec<DelegatorCtx>,
    pub meta_merkle_proof_account: Address,
}

/// A proposal that reached the voting phase, bound to a fabricated snapshot.
pub struct VotingScenario {
    pub proposal: Address,
    pub snapshot_slot: u64,
    pub start_epoch: u64,
    pub end_epoch: u64,
    pub consensus_result: Address,
    pub voters: Vec<SnapshotValidator>,
}

/// Drives a fresh proposal through support to activation, fabricates a
/// two-tier merkle snapshot for `specs` (`(validator_idx, active_stake,
/// delegator_stakes)`), writes the ConsensusResult + MetaMerkleProof accounts
/// the ncn-snapshot program expects, and advances the clock into the voting
/// window.
pub fn setup_voting_scenario(
    h: &mut Harness,
    seed: u64,
    title: &str,
    specs: &[(usize, u64, &[u64])],
) -> VotingScenario {
    let creation_epoch = h.svm.get_sysvar::<Clock>().epoch;
    let proposal = create_proposal(h, seed, title);
    let ballot_box = seed_ballot_box(&mut h.svm, expected_snapshot_slot(creation_epoch));
    for i in 0..h.supporter_count {
        support_one(h, proposal, i, ballot_box);
    }
    let state = fetch_proposal(&h.svm, &proposal);
    assert!(state.voting, "proposal must reach the voting phase");
    let snapshot_slot = state.snapshot_slot;
    let consensus_result = consensus_result_pda(snapshot_slot);
    assert_eq!(
        state.consensus_result,
        Some(consensus_result.to_bytes()),
        "proposal must bind the ConsensusResult PDA the harness fabricates"
    );

    // Build each validator's stake tree, then the meta tree over all leaves.
    let mut voters = Vec::with_capacity(specs.len());
    let mut meta_contents = Vec::with_capacity(specs.len());
    for (idx, active_stake, delegator_stakes) in specs {
        let delegators: Vec<DelegatorCtx> = delegator_stakes
            .iter()
            .map(|stake| {
                let wallet = Keypair::new();
                h.svm
                    .airdrop(&wallet.pubkey(), 10 * LAMPORTS_PER_SOL)
                    .unwrap();
                let stake_account = Address::new_unique();
                let data = vec![0u8; 8];
                let lamports = h.svm.minimum_balance_for_rent_exemption(data.len());
                h.svm
                    .set_account(
                        stake_account,
                        Account {
                            lamports,
                            data,
                            owner: solana_sdk_ids::stake::ID,
                            executable: false,
                            rent_epoch: 0,
                        },
                    )
                    .unwrap();
                DelegatorCtx {
                    wallet,
                    stake_account,
                    stake: *stake,
                    stake_proof: Vec::new(),
                }
            })
            .collect();

        let stake_contents: Vec<[u8; 32]> = delegators
            .iter()
            .map(|d| {
                StakeMerkleLeaf {
                    voting_wallet: to_pubkey(&d.wallet.pubkey()),
                    stake_account: to_pubkey(&d.stake_account),
                    active_stake: d.stake,
                }
                .hash()
                .to_bytes()
            })
            .collect();
        let (stake_root, stake_proofs) = build_merkle(&stake_contents);

        let mut delegators = delegators;
        for (d, proof) in delegators.iter_mut().zip(stake_proofs) {
            d.stake_proof = proof;
        }

        let meta_leaf = MetaMerkleLeaf {
            voting_wallet: to_pubkey(&h.validators[*idx].identity.pubkey()),
            vote_account: to_pubkey(&h.validators[*idx].vote.pubkey()),
            stake_merkle_root: stake_root,
            active_stake: *active_stake,
        };
        meta_contents.push(meta_leaf.hash().to_bytes());
        voters.push((idx, active_stake, delegators, meta_leaf));
    }
    let (meta_root, meta_proofs) = build_merkle(&meta_contents);

    write_ncn_account(
        &mut h.svm,
        consensus_result,
        ConsensusResult::DISCRIMINATOR,
        &ConsensusResult {
            snapshot_slot,
            ballot: Ballot {
                meta_merkle_root: meta_root,
                snapshot_hash: [0; 32],
            },
            tie_breaker_consensus: false,
        },
    );

    let voters = voters
        .into_iter()
        .zip(meta_proofs)
        .map(|((idx, active_stake, delegators, meta_leaf), meta_proof)| {
            let identity = h.validators[*idx].identity.pubkey();
            let vote_account = h.validators[*idx].vote.pubkey();
            let proof_account = meta_merkle_proof_pda(&consensus_result, &vote_account);
            write_ncn_account(
                &mut h.svm,
                proof_account,
                MetaMerkleProof::DISCRIMINATOR,
                &MetaMerkleProof {
                    payer: to_pubkey(&identity),
                    consensus_result: to_pubkey(&consensus_result),
                    meta_merkle_leaf: meta_leaf,
                    meta_merkle_proof: meta_proof,
                    close_timestamp: 0,
                },
            );
            SnapshotValidator {
                idx: *idx,
                active_stake: *active_stake,
                delegators,
                meta_merkle_proof_account: proof_account,
            }
        })
        .collect();

    set_clock(&mut h.svm, state.start_epoch);
    VotingScenario {
        proposal,
        snapshot_slot,
        start_epoch: state.start_epoch,
        end_epoch: state.end_epoch,
        consensus_result,
        voters,
    }
}

pub fn cast_vote_ix(
    signer: &Address,
    proposal: Address,
    spl_vote_account: Address,
    consensus_result: Address,
    meta_merkle_proof: Address,
    for_bp: u64,
    against_bp: u64,
    abstain_bp: u64,
) -> Instruction {
    let vote = vote_pda(&proposal, &spl_vote_account);
    let mut data = anchor_discriminator("global", "cast_vote").to_vec();
    data.extend(for_bp.to_le_bytes());
    data.extend(against_bp.to_le_bytes());
    data.extend(abstain_bp.to_le_bytes());
    Instruction {
        program_id: SVMGOV_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*signer, true),
            AccountMeta::new(proposal, false),
            AccountMeta::new(vote, false),
            AccountMeta::new_readonly(spl_vote_account, false),
            AccountMeta::new(vote_override_cache_pda(&proposal, &vote), false),
            AccountMeta::new_readonly(NCN_SNAPSHOT_PROGRAM_ID, false),
            AccountMeta::new_readonly(consensus_result, false),
            AccountMeta::new_readonly(meta_merkle_proof, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    }
}

/// `cast_vote_override` and `modify_vote_override` share the same account
/// list and argument encoding; only the instruction discriminator differs.
#[allow(clippy::too_many_arguments)]
fn vote_override_ix(
    ix_name: &str,
    signer: &Address,
    proposal: Address,
    spl_vote_account: Address,
    spl_stake_account: Address,
    consensus_result: Address,
    meta_merkle_proof: Address,
    stake_merkle_proof: &[[u8; 32]],
    stake_leaf_stake: u64,
    for_bp: u64,
    against_bp: u64,
    abstain_bp: u64,
) -> Instruction {
    let validator_vote = vote_pda(&proposal, &spl_vote_account);
    let mut data = anchor_discriminator("global", ix_name).to_vec();
    data.extend(for_bp.to_le_bytes());
    data.extend(against_bp.to_le_bytes());
    data.extend(abstain_bp.to_le_bytes());
    data.extend((stake_merkle_proof.len() as u32).to_le_bytes());
    for node in stake_merkle_proof {
        data.extend(node);
    }
    let stake_leaf = StakeMerkleLeaf {
        voting_wallet: to_pubkey(signer),
        stake_account: to_pubkey(&spl_stake_account),
        active_stake: stake_leaf_stake,
    };
    data.extend(stake_leaf.try_to_vec().expect("serialize StakeMerkleLeaf"));
    Instruction {
        program_id: SVMGOV_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*signer, true),
            AccountMeta::new(proposal, false),
            AccountMeta::new(validator_vote, false),
            AccountMeta::new_readonly(spl_vote_account, false),
            AccountMeta::new(
                vote_override_pda(&proposal, &spl_stake_account, &validator_vote),
                false,
            ),
            AccountMeta::new(vote_override_cache_pda(&proposal, &validator_vote), false),
            AccountMeta::new_readonly(spl_stake_account, false),
            AccountMeta::new_readonly(NCN_SNAPSHOT_PROGRAM_ID, false),
            AccountMeta::new_readonly(consensus_result, false),
            AccountMeta::new_readonly(meta_merkle_proof, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn cast_vote_override_ix(
    signer: &Address,
    proposal: Address,
    spl_vote_account: Address,
    spl_stake_account: Address,
    consensus_result: Address,
    meta_merkle_proof: Address,
    stake_merkle_proof: &[[u8; 32]],
    stake_leaf_stake: u64,
    for_bp: u64,
    against_bp: u64,
    abstain_bp: u64,
) -> Instruction {
    vote_override_ix(
        "cast_vote_override",
        signer,
        proposal,
        spl_vote_account,
        spl_stake_account,
        consensus_result,
        meta_merkle_proof,
        stake_merkle_proof,
        stake_leaf_stake,
        for_bp,
        against_bp,
        abstain_bp,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn modify_vote_override_ix(
    signer: &Address,
    proposal: Address,
    spl_vote_account: Address,
    spl_stake_account: Address,
    consensus_result: Address,
    meta_merkle_proof: Address,
    stake_merkle_proof: &[[u8; 32]],
    stake_leaf_stake: u64,
    for_bp: u64,
    against_bp: u64,
    abstain_bp: u64,
) -> Instruction {
    vote_override_ix(
        "modify_vote_override",
        signer,
        proposal,
        spl_vote_account,
        spl_stake_account,
        consensus_result,
        meta_merkle_proof,
        stake_merkle_proof,
        stake_leaf_stake,
        for_bp,
        against_bp,
        abstain_bp,
    )
}

pub fn modify_vote_ix(
    signer: &Address,
    proposal: Address,
    spl_vote_account: Address,
    consensus_result: Address,
    meta_merkle_proof: Address,
    for_bp: u64,
    against_bp: u64,
    abstain_bp: u64,
) -> Instruction {
    let vote = vote_pda(&proposal, &spl_vote_account);
    let mut data = anchor_discriminator("global", "modify_vote").to_vec();
    data.extend(for_bp.to_le_bytes());
    data.extend(against_bp.to_le_bytes());
    data.extend(abstain_bp.to_le_bytes());
    Instruction {
        program_id: SVMGOV_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*signer, true),
            AccountMeta::new(proposal, false),
            AccountMeta::new(vote, false),
            AccountMeta::new_readonly(spl_vote_account, false),
            AccountMeta::new_readonly(NCN_SNAPSHOT_PROGRAM_ID, false),
            AccountMeta::new_readonly(consensus_result, false),
            AccountMeta::new_readonly(meta_merkle_proof, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    }
}

pub fn finalize_proposal_ix(signer: &Address, proposal: Address) -> Instruction {
    Instruction {
        program_id: SVMGOV_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*signer, true),
            AccountMeta::new(proposal, false),
        ],
        data: anchor_discriminator("global", "finalize_proposal").to_vec(),
    }
}

/// Casts the scenario validator's own vote.
pub fn cast_validator_vote(
    h: &mut Harness,
    s: &VotingScenario,
    voter: usize,
    for_bp: u64,
    against_bp: u64,
    abstain_bp: u64,
) {
    let identity = h.validators[s.voters[voter].idx].identity.insecure_clone();
    let vote_account = h.validators[s.voters[voter].idx].vote.pubkey();
    send_ix(
        &mut h.svm,
        &identity,
        &[cast_vote_ix(
            &identity.pubkey(),
            s.proposal,
            vote_account,
            s.consensus_result,
            s.voters[voter].meta_merkle_proof_account,
            for_bp,
            against_bp,
            abstain_bp,
        )],
    );
}

/// Casts a delegator override against the scenario validator's vote account.
pub fn cast_delegator_override(
    h: &mut Harness,
    s: &VotingScenario,
    voter: usize,
    delegator: usize,
    for_bp: u64,
    against_bp: u64,
    abstain_bp: u64,
) {
    let vote_account = h.validators[s.voters[voter].idx].vote.pubkey();
    let d = &s.voters[voter].delegators[delegator];
    let wallet = d.wallet.insecure_clone();
    let ix = cast_vote_override_ix(
        &wallet.pubkey(),
        s.proposal,
        vote_account,
        d.stake_account,
        s.consensus_result,
        s.voters[voter].meta_merkle_proof_account,
        &d.stake_proof,
        d.stake,
        for_bp,
        against_bp,
        abstain_bp,
    );
    send_ix(&mut h.svm, &wallet, &[ix]);
}

/// Modifies the scenario validator's existing vote distribution.
pub fn modify_validator_vote(
    h: &mut Harness,
    s: &VotingScenario,
    voter: usize,
    for_bp: u64,
    against_bp: u64,
    abstain_bp: u64,
) {
    let identity = h.validators[s.voters[voter].idx].identity.insecure_clone();
    let vote_account = h.validators[s.voters[voter].idx].vote.pubkey();
    send_ix(
        &mut h.svm,
        &identity,
        &[modify_vote_ix(
            &identity.pubkey(),
            s.proposal,
            vote_account,
            s.consensus_result,
            s.voters[voter].meta_merkle_proof_account,
            for_bp,
            against_bp,
            abstain_bp,
        )],
    );
}

/// Modifies a delegator's existing override distribution.
pub fn modify_delegator_override(
    h: &mut Harness,
    s: &VotingScenario,
    voter: usize,
    delegator: usize,
    for_bp: u64,
    against_bp: u64,
    abstain_bp: u64,
) {
    let vote_account = h.validators[s.voters[voter].idx].vote.pubkey();
    let d = &s.voters[voter].delegators[delegator];
    let wallet = d.wallet.insecure_clone();
    let ix = modify_vote_override_ix(
        &wallet.pubkey(),
        s.proposal,
        vote_account,
        d.stake_account,
        s.consensus_result,
        s.voters[voter].meta_merkle_proof_account,
        &d.stake_proof,
        d.stake,
        for_bp,
        against_bp,
        abstain_bp,
    );
    send_ix(&mut h.svm, &wallet, &[ix]);
}

/// Advances past the voting window and finalizes the proposal.
pub fn finalize_proposal(h: &mut Harness, s: &VotingScenario) -> ProposalAccount {
    set_clock(&mut h.svm, s.end_epoch);
    let signer = h.validators[0].identity.insecure_clone();
    send_ix(
        &mut h.svm,
        &signer,
        &[finalize_proposal_ix(&signer.pubkey(), s.proposal)],
    );
    let state = fetch_proposal(&h.svm, &s.proposal);
    assert!(state.finalized, "proposal must be finalized");
    state
}

pub fn fetch_vote(h: &Harness, s: &VotingScenario, voter: usize) -> Option<VoteAccountState> {
    let vote = vote_pda(
        &s.proposal,
        &h.validators[s.voters[voter].idx].vote.pubkey(),
    );
    let account = h.svm.get_account(&vote)?;
    if account.data.len() <= 8 {
        return None;
    }
    Some(VoteAccountState::deserialize(&mut &account.data[8..]).expect("deserialize Vote"))
}

pub fn fetch_override_cache(
    h: &Harness,
    s: &VotingScenario,
    voter: usize,
) -> Option<VoteOverrideCacheState> {
    let vote = vote_pda(
        &s.proposal,
        &h.validators[s.voters[voter].idx].vote.pubkey(),
    );
    let cache = vote_override_cache_pda(&s.proposal, &vote);
    let account = h.svm.get_account(&cache)?;
    if account.data.len() <= 8 {
        return None;
    }
    Some(
        VoteOverrideCacheState::deserialize(&mut &account.data[8..])
            .expect("deserialize VoteOverrideCache"),
    )
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

// ---------------------------------------------------------------------------
// NCN program-flow scaffolding: direct ncn-snapshot instruction builders used
// by the `ncn_flow` suite (the LiteSVM port of the old `ncn/tests` anchor
// suite). Argument encodings are borsh, matching anchor's instruction
// serialization; account orders mirror the program's `#[derive(Accounts)]`
// structs.
// ---------------------------------------------------------------------------

pub fn ncn_program_config_pda() -> Address {
    Address::find_program_address(&[b"ProgramConfig"], &NCN_SNAPSHOT_PROGRAM_ID).0
}

pub fn ballot_box_pda(snapshot_slot: u64) -> Address {
    Address::find_program_address(
        &[b"BallotBox", &snapshot_slot.to_le_bytes()],
        &NCN_SNAPSHOT_PROGRAM_ID,
    )
    .0
}

/// Replaces whatever the harness seeded at `address` with a non-existent
/// account, so a real `init` instruction can create it.
pub fn clear_account(svm: &mut LiteSVM, address: Address) {
    svm.set_account(
        address,
        Account {
            lamports: 0,
            data: vec![],
            owner: system_program::ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();
}

/// Moves only the clock's unix timestamp (epoch/slot untouched), for
/// vote-expiry scenarios.
pub fn set_clock_timestamp(svm: &mut LiteSVM, unix_timestamp: i64) {
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp = unix_timestamp;
    svm.set_sysvar(&clock);
}

/// Deserializes an anchor account of the ncn-snapshot program (discriminator
/// checked by `AccountDeserialize`).
pub fn fetch_ncn_account<T: anchor_lang::AccountDeserialize>(
    svm: &LiteSVM,
    address: &Address,
) -> T {
    let account = svm.get_account(address).expect("ncn account must exist");
    T::try_deserialize(&mut account.data.as_slice()).expect("deserialize ncn account")
}

pub fn ncn_custom_error(err: ncn_snapshot::error::ErrorCode) -> InstructionError {
    InstructionError::Custom(ANCHOR_ERROR_CODE_OFFSET + err as u32)
}

fn encode_opt_pubkey(data: &mut Vec<u8>, value: Option<&Address>) {
    match value {
        None => data.push(0),
        Some(a) => {
            data.push(1);
            data.extend(a.to_bytes());
        }
    }
}

fn encode_opt_pubkey_vec(data: &mut Vec<u8>, value: Option<&[Address]>) {
    match value {
        None => data.push(0),
        Some(keys) => {
            data.push(1);
            data.extend((keys.len() as u32).to_le_bytes());
            for key in keys {
                data.extend(key.to_bytes());
            }
        }
    }
}

pub fn init_ncn_program_config_ix(
    payer: &Address,
    authority: &Address,
    svmgov_program: &Address,
) -> Instruction {
    let mut data = anchor_discriminator("global", "init_program_config").to_vec();
    data.extend(svmgov_program.to_bytes());
    Instruction {
        program_id: NCN_SNAPSHOT_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new(ncn_program_config_pda(), false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    }
}

pub fn update_operator_whitelist_ix(
    authority: &Address,
    operators_to_add: Option<&[Address]>,
    operators_to_remove: Option<&[Address]>,
) -> Instruction {
    let mut data = anchor_discriminator("global", "update_operator_whitelist").to_vec();
    encode_opt_pubkey_vec(&mut data, operators_to_add);
    encode_opt_pubkey_vec(&mut data, operators_to_remove);
    Instruction {
        program_id: NCN_SNAPSHOT_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new(ncn_program_config_pda(), false),
        ],
        data,
    }
}

pub fn update_ncn_program_config_ix(
    authority: &Address,
    proposed_authority: Option<&Address>,
    min_consensus_threshold_bps: Option<u16>,
    tie_breaker_admin: Option<&Address>,
    vote_duration: Option<i64>,
    svmgov_program: Option<&Address>,
) -> Instruction {
    let mut data = anchor_discriminator("global", "update_program_config").to_vec();
    encode_opt_pubkey(&mut data, proposed_authority);
    match min_consensus_threshold_bps {
        None => data.push(0),
        Some(bps) => {
            data.push(1);
            data.extend(bps.to_le_bytes());
        }
    }
    encode_opt_pubkey(&mut data, tie_breaker_admin);
    match vote_duration {
        None => data.push(0),
        Some(duration) => {
            data.push(1);
            data.extend(duration.to_le_bytes());
        }
    }
    encode_opt_pubkey(&mut data, svmgov_program);
    Instruction {
        program_id: NCN_SNAPSHOT_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new(ncn_program_config_pda(), false),
        ],
        data,
    }
}

pub fn finalize_proposed_authority_ix(authority: &Address) -> Instruction {
    Instruction {
        program_id: NCN_SNAPSHOT_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new(ncn_program_config_pda(), false),
        ],
        data: anchor_discriminator("global", "finalize_proposed_authority").to_vec(),
    }
}

pub fn ncn_cast_vote_ix(operator: &Address, ballot_box: Address, ballot: &Ballot) -> Instruction {
    let mut data = anchor_discriminator("global", "cast_vote").to_vec();
    data.extend(ballot.try_to_vec().expect("serialize Ballot"));
    Instruction {
        program_id: NCN_SNAPSHOT_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*operator, true),
            AccountMeta::new(ballot_box, false),
        ],
        data,
    }
}

pub fn ncn_remove_vote_ix(operator: &Address, ballot_box: Address) -> Instruction {
    Instruction {
        program_id: NCN_SNAPSHOT_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*operator, true),
            AccountMeta::new(ballot_box, false),
        ],
        data: anchor_discriminator("global", "remove_vote").to_vec(),
    }
}

pub fn set_tie_breaker_ix(admin: &Address, ballot_box: Address, ballot: &Ballot) -> Instruction {
    let mut data = anchor_discriminator("global", "set_tie_breaker").to_vec();
    data.extend(ballot.try_to_vec().expect("serialize Ballot"));
    Instruction {
        program_id: NCN_SNAPSHOT_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*admin, true),
            AccountMeta::new(ballot_box, false),
            AccountMeta::new_readonly(ncn_program_config_pda(), false),
        ],
        data,
    }
}

pub fn reset_ballot_box_ix(admin: &Address, ballot_box: Address) -> Instruction {
    Instruction {
        program_id: NCN_SNAPSHOT_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*admin, true),
            AccountMeta::new(ballot_box, false),
            AccountMeta::new_readonly(ncn_program_config_pda(), false),
        ],
        data: anchor_discriminator("global", "reset_ballot_box").to_vec(),
    }
}

pub fn finalize_ballot_ix(
    payer: &Address,
    ballot_box: Address,
    consensus_result: Address,
) -> Instruction {
    Instruction {
        program_id: NCN_SNAPSHOT_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(ballot_box, false),
            AccountMeta::new(consensus_result, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: anchor_discriminator("global", "finalize_ballot").to_vec(),
    }
}

pub fn init_meta_merkle_proof_ix(
    payer: &Address,
    consensus_result: Address,
    meta_merkle_leaf: &MetaMerkleLeaf,
    meta_merkle_proof: &[[u8; 32]],
    close_timestamp: i64,
) -> Instruction {
    let merkle_proof_account = meta_merkle_proof_pda(
        &consensus_result,
        &Address::new_from_array(meta_merkle_leaf.vote_account.to_bytes()),
    );
    let mut data = anchor_discriminator("global", "init_meta_merkle_proof").to_vec();
    data.extend(
        meta_merkle_leaf
            .try_to_vec()
            .expect("serialize MetaMerkleLeaf"),
    );
    data.extend((meta_merkle_proof.len() as u32).to_le_bytes());
    for node in meta_merkle_proof {
        data.extend(node);
    }
    data.extend(close_timestamp.to_le_bytes());
    Instruction {
        program_id: NCN_SNAPSHOT_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(merkle_proof_account, false),
            AccountMeta::new_readonly(consensus_result, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    }
}

pub fn ncn_verify_merkle_proof_ix(
    meta_merkle_proof_account: Address,
    consensus_result: Address,
    stake_merkle_proof: Option<&[[u8; 32]]>,
    stake_merkle_leaf: Option<&StakeMerkleLeaf>,
) -> Instruction {
    let mut data = anchor_discriminator("global", "verify_merkle_proof").to_vec();
    match stake_merkle_proof {
        None => data.push(0),
        Some(proof) => {
            data.push(1);
            data.extend((proof.len() as u32).to_le_bytes());
            for node in proof {
                data.extend(node);
            }
        }
    }
    match stake_merkle_leaf {
        None => data.push(0),
        Some(leaf) => {
            data.push(1);
            data.extend(leaf.try_to_vec().expect("serialize StakeMerkleLeaf"));
        }
    }
    Instruction {
        program_id: NCN_SNAPSHOT_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(meta_merkle_proof_account, false),
            AccountMeta::new_readonly(consensus_result, false),
        ],
        data,
    }
}

/// `payer` receives the reclaimed rent and must match the recorded payer;
/// `payer_signs` selects the signed (immediate) vs unsigned
/// (post-close-timestamp) close path.
pub fn close_meta_merkle_proof_ix(
    payer: &Address,
    payer_signs: bool,
    meta_merkle_proof_account: Address,
) -> Instruction {
    Instruction {
        program_id: NCN_SNAPSHOT_PROGRAM_ID,
        accounts: vec![
            AccountMeta {
                pubkey: *payer,
                is_signer: payer_signs,
                is_writable: true,
            },
            AccountMeta::new(meta_merkle_proof_account, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: anchor_discriminator("global", "close_meta_merkle_proof").to_vec(),
    }
}
