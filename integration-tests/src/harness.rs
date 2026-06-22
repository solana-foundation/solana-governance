//! Test harness for the cross-program governance integration test.
//!
//! Boots an in-process Surfpool surfnet, deploys both programs (with our admin as
//! the upgrade authority — `initialize_config` requires it), and provides the raw
//! instruction encoders, PDA helpers, a self-consistent sorted merkle tree, and
//! cheatcode wrappers the flow needs.
//!
//! We deliberately do NOT depend on the anchor program crates (svmgov_program pulls
//! solana-vote-interface 6.0, which conflicts with surfpool's litesvm pin on
//! solana-instruction). Instead instructions are encoded raw (anchor discriminator
//! `sha256("global:<name>")[..8]` + borsh args) and accounts decoded with borsh
//! mirror structs — keeping a single solana (3.x) version with no bridging.
#![cfg(test)]
#![allow(dead_code)]

use borsh::{BorshDeserialize, BorshSerialize};
use sha2::{Digest, Sha256};
use solana_instruction::{AccountMeta, Instruction};
use solana_rpc_client_api::request::RpcRequest;
use solana_transaction::Transaction;
use surfpool_sdk::{
    cheatcodes::builders::{SetEpochStake, SetStakeAccount, SetVoteAccount},
    Cheatcodes, Keypair, Pubkey, RpcClient, Signer, Surfnet, SurfnetResult,
};

// ---------------------------------------------------------------------------
// Program ids & native programs
// ---------------------------------------------------------------------------

pub const SVMGOV_ID: Pubkey = Pubkey::from_str_const("govYkyQ3ePtGULAtY6V75qjWE8UH4vCUVQ1W4HdCAZU");
pub const NCN_ID: Pubkey = Pubkey::from_str_const("ncnwF8AgynRcdEnGLcprSQNaKvgSMTgk3yPRc8cf9Zf");
pub const SYSTEM_ID: Pubkey = Pubkey::from_str_const("11111111111111111111111111111111");
pub const BPF_LOADER_UPGRADEABLE: Pubkey =
    Pubkey::from_str_const("BPFLoaderUpgradeab1e11111111111111111111111");
pub const VOTE_PROGRAM: Pubkey = Pubkey::from_str_const("Vote111111111111111111111111111111111111111");
/// `VoteState::size_of()` — the fixed allocation size the governance program
/// requires for `spl_vote_account` (`data_len() == VoteState::size_of()`).
pub const VOTE_STATE_SIZE: usize = 3762;

pub const SVMGOV_SO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../svmgov/program/target/deploy/svmgov_program.so"
);
pub const NCN_SO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../ncn/target/deploy/ncn_snapshot.so"
);

/// `svmgov_program`'s baked-in slots-per-epoch (see its `utils.rs`). The program
/// derives `snapshot_slot` from this, independent of surfpool's epoch schedule.
pub const SLOTS_PER_EPOCH: u64 = 432_000;
pub const SOL: u64 = 1_000_000_000;

// ---------------------------------------------------------------------------
// Deploy (raw surfnet_writeProgram with our chosen upgrade authority)
// ---------------------------------------------------------------------------

/// Deploys `program_id` from `so_path` via the upgradeable loader, setting
/// `upgrade_authority` as the program's upgrade authority. The SDK's `deploy`
/// would leave the authority as the system program, which `initialize_config`
/// rejects — so we call `surfnet_writeProgram` directly with the authority.
pub fn deploy_program(
    rpc: &RpcClient,
    program_id: &Pubkey,
    so_path: &str,
    upgrade_authority: &Pubkey,
) -> SurfnetResult<()> {
    let bytes = std::fs::read(so_path)
        .unwrap_or_else(|e| panic!("read {so_path} (run `make build-programs`): {e}"));
    let params = serde_json::json!([
        program_id.to_string(),
        hex::encode(&bytes),
        0,
        upgrade_authority.to_string(),
    ]);
    rpc.send::<serde_json::Value>(RpcRequest::Custom { method: "surfnet_writeProgram" }, params)
        .map(|_| ())
        .map_err(|e| surfpool_sdk::SurfnetError::Cheatcode(format!("surfnet_writeProgram: {e}")))
}

// ---------------------------------------------------------------------------
// Transactions & account reads
// ---------------------------------------------------------------------------

/// Signs `ixs` with `signers` (first = fee payer), sends, and confirms — returning
/// the error (debug-formatted, includes program logs) on failure. Use this to
/// assert on expected failures.
pub fn try_send(rpc: &RpcClient, ixs: &[Instruction], signers: &[&Keypair]) -> Result<(), String> {
    let blockhash = rpc.get_latest_blockhash().map_err(|e| format!("{e:?}"))?;
    let tx = Transaction::new_signed_with_payer(ixs, Some(&signers[0].pubkey()), signers, blockhash);
    rpc.send_and_confirm_transaction(&tx)
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))
}

/// Signs `ixs` with `signers` (first = fee payer) and confirms, panicking on failure.
pub fn send(rpc: &RpcClient, ixs: &[Instruction], signers: &[&Keypair]) {
    try_send(rpc, ixs, signers).expect("send_and_confirm_transaction");
}

/// Fetches an anchor account and borsh-decodes it after the 8-byte discriminator.
/// Uses `deserialize` (not `try_from_slice`) because anchor accounts are allocated
/// at max `INIT_SPACE` with trailing zero padding that must be ignored.
pub fn fetch<T: BorshDeserialize>(rpc: &RpcClient, address: &Pubkey) -> T {
    let account = rpc.get_account(address).expect("account exists");
    let mut slice: &[u8] = &account.data[8..];
    T::deserialize(&mut slice).expect("borsh decode account")
}

// ---------------------------------------------------------------------------
// Raw anchor instruction encoding
// ---------------------------------------------------------------------------

fn discriminator(name: &str) -> [u8; 8] {
    let hash = Sha256::digest(format!("global:{name}").as_bytes());
    hash[..8].try_into().unwrap()
}

/// Builds an anchor instruction: data = discriminator(name) ++ borsh(args).
fn anchor_ix(program: Pubkey, name: &str, args: &[u8], accounts: Vec<AccountMeta>) -> Instruction {
    let mut data = discriminator(name).to_vec();
    data.extend_from_slice(args);
    Instruction { program_id: program, accounts, data }
}

fn ser(v: &impl BorshSerialize) -> Vec<u8> {
    borsh::to_vec(v).unwrap()
}

// ---------------------------------------------------------------------------
// Borsh structs (must match the on-chain field order; pubkeys as raw [u8;32])
// ---------------------------------------------------------------------------

type Key = [u8; 32];

#[derive(BorshSerialize, Clone)]
pub struct Ballot {
    pub meta_merkle_root: [u8; 32],
    pub snapshot_hash: [u8; 32],
}

#[derive(BorshSerialize, Clone)]
pub struct MetaMerkleLeaf {
    pub voting_wallet: Key,
    pub vote_account: Key,
    pub stake_merkle_root: [u8; 32],
    pub active_stake: u64,
}

#[derive(BorshSerialize, Clone)]
pub struct StakeMerkleLeaf {
    pub voting_wallet: Key,
    pub stake_account: Key,
    pub active_stake: u64,
}

// ---- account mirror structs (BorshDeserialize) ----

#[derive(BorshDeserialize, Debug)]
pub struct ProposalAcct {
    pub author: Key,
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
    pub consensus_result: Option<Key>,
    pub snapshot_slot: u64,
    pub proposal_seed: u64,
    pub vote_account_pubkey: Key,
}

#[derive(BorshDeserialize, Debug)]
pub struct BallotData {
    pub meta_merkle_root: [u8; 32],
    pub snapshot_hash: [u8; 32],
}

#[derive(BorshDeserialize, Debug)]
pub struct ConsensusResultAcct {
    pub snapshot_slot: u64,
    pub ballot: BallotData,
    pub tie_breaker_consensus: bool,
}

#[derive(BorshDeserialize, Debug)]
pub struct NcnProgramConfigAcct {
    pub authority: Key,
    pub proposed_authority: Option<Key>,
    pub whitelisted_operators: Vec<Key>,
    pub min_consensus_threshold_bps: u16,
    pub tie_breaker_admin: Key,
    pub vote_duration: i64,
    pub svmgov_program_pubkey: Key,
}

// ---------------------------------------------------------------------------
// PDA helpers
// ---------------------------------------------------------------------------

pub fn global_config_pda() -> Pubkey {
    Pubkey::find_program_address(&[b"global_config"], &SVMGOV_ID).0
}
pub fn proposal_index_pda() -> Pubkey {
    Pubkey::find_program_address(&[b"index"], &SVMGOV_ID).0
}
pub fn proposal_pda(seed: u64, vote_account: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"proposal", &seed.to_le_bytes(), vote_account.as_ref()],
        &SVMGOV_ID,
    )
    .0
}
pub fn support_pda(proposal: &Pubkey, vote_account: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"support", proposal.as_ref(), vote_account.as_ref()],
        &SVMGOV_ID,
    )
    .0
}
pub fn vote_pda(proposal: &Pubkey, vote_account: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"vote", proposal.as_ref(), vote_account.as_ref()],
        &SVMGOV_ID,
    )
    .0
}
pub fn vote_override_pda(proposal: &Pubkey, stake_account: &Pubkey, validator_vote: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"vote_override", proposal.as_ref(), stake_account.as_ref(), validator_vote.as_ref()],
        &SVMGOV_ID,
    )
    .0
}
pub fn vote_override_cache_pda(proposal: &Pubkey, validator_vote: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"vote_override_cache", proposal.as_ref(), validator_vote.as_ref()],
        &SVMGOV_ID,
    )
    .0
}
pub fn program_data_pda(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[program_id.as_ref()], &BPF_LOADER_UPGRADEABLE).0
}
pub fn ncn_program_config_pda() -> Pubkey {
    Pubkey::find_program_address(&[b"ProgramConfig"], &NCN_ID).0
}
pub fn ballot_box_pda(snapshot_slot: u64) -> Pubkey {
    Pubkey::find_program_address(&[&b"BallotBox"[..], &snapshot_slot.to_le_bytes()], &NCN_ID).0
}
pub fn consensus_result_pda(snapshot_slot: u64) -> Pubkey {
    Pubkey::find_program_address(&[&b"ConsensusResult"[..], &snapshot_slot.to_le_bytes()], &NCN_ID).0
}
pub fn meta_merkle_proof_pda(consensus_result: &Pubkey, vote_account: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"MetaMerkleProof", consensus_result.as_ref(), vote_account.as_ref()],
        &NCN_ID,
    )
    .0
}

/// Snapshot-slot math from `svmgov_program::utils` (support activates in the
/// creation epoch when `max_support_epochs == 0`).
pub fn expected_snapshot_slot(
    support_epoch: u64,
    discussion_epochs: u64,
    snapshot_epoch_extension: u64,
    snapshot_slot_offset: i64,
) -> u64 {
    let target_epoch = support_epoch + discussion_epochs + snapshot_epoch_extension;
    ((target_epoch * SLOTS_PER_EPOCH) as i64 + snapshot_slot_offset) as u64
}

// ---------------------------------------------------------------------------
// svmgov instruction builders
// ---------------------------------------------------------------------------

#[derive(BorshSerialize)]
pub struct GovConfigArgs {
    pub max_title_length: u16,
    pub max_description_length: u16,
    pub max_support_epochs: u64,
    pub min_proposal_stake_lamports: u64,
    pub cluster_support_pct_min_bps: u64,
    pub discussion_epochs: u64,
    pub voting_epochs: u64,
    pub snapshot_epoch_extension: u64,
    pub snapshot_slot_offset: i64,
}

pub fn ix_initialize_config(admin: &Pubkey, args: &GovConfigArgs) -> Instruction {
    anchor_ix(
        SVMGOV_ID,
        "initialize_config",
        &ser(args),
        vec![
            AccountMeta::new(*admin, true),
            AccountMeta::new(global_config_pda(), false),
            AccountMeta::new_readonly(SYSTEM_ID, false),
            AccountMeta::new_readonly(SVMGOV_ID, false),
            AccountMeta::new_readonly(program_data_pda(&SVMGOV_ID), false),
        ],
    )
}

pub fn ix_initialize_index(signer: &Pubkey) -> Instruction {
    anchor_ix(
        SVMGOV_ID,
        "initialize_index",
        &[],
        vec![
            AccountMeta::new(*signer, true),
            AccountMeta::new(proposal_index_pda(), false),
            AccountMeta::new_readonly(SYSTEM_ID, false),
        ],
    )
}

#[derive(BorshSerialize)]
struct CreateProposalArgs {
    seed: u64,
    title: String,
    description: String,
}

pub fn ix_create_proposal(
    signer: &Pubkey,
    vote_account: &Pubkey,
    seed: u64,
    title: &str,
    description: &str,
) -> Instruction {
    let args = CreateProposalArgs {
        seed,
        title: title.to_string(),
        description: description.to_string(),
    };
    anchor_ix(
        SVMGOV_ID,
        "create_proposal",
        &ser(&args),
        vec![
            AccountMeta::new(*signer, true),
            AccountMeta::new(proposal_pda(seed, vote_account), false),
            AccountMeta::new(proposal_index_pda(), false),
            AccountMeta::new_readonly(*vote_account, false),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(SYSTEM_ID, false),
        ],
    )
}

pub fn ix_support_proposal(
    signer: &Pubkey,
    proposal: &Pubkey,
    vote_account: &Pubkey,
    snapshot_slot: u64,
) -> Instruction {
    anchor_ix(
        SVMGOV_ID,
        "support_proposal",
        &[],
        vec![
            AccountMeta::new(*signer, true),
            AccountMeta::new(*proposal, false),
            AccountMeta::new(support_pda(proposal, vote_account), false),
            AccountMeta::new_readonly(*vote_account, false),
            AccountMeta::new(ballot_box_pda(snapshot_slot), false),
            AccountMeta::new_readonly(NCN_ID, false),
            AccountMeta::new_readonly(ncn_program_config_pda(), false),
            AccountMeta::new_readonly(global_config_pda(), false),
            AccountMeta::new_readonly(SYSTEM_ID, false),
        ],
    )
}

#[derive(BorshSerialize)]
struct CastVoteArgs {
    for_votes_bp: u64,
    against_votes_bp: u64,
    abstain_votes_bp: u64,
}

pub fn ix_cast_vote(
    signer: &Pubkey,
    proposal: &Pubkey,
    vote_account: &Pubkey,
    snapshot_slot: u64,
    for_bp: u64,
    against_bp: u64,
    abstain_bp: u64,
) -> Instruction {
    let consensus_result = consensus_result_pda(snapshot_slot);
    let args = CastVoteArgs { for_votes_bp: for_bp, against_votes_bp: against_bp, abstain_votes_bp: abstain_bp };
    let vote = vote_pda(proposal, vote_account);
    anchor_ix(
        SVMGOV_ID,
        "cast_vote",
        &ser(&args),
        vec![
            AccountMeta::new(*signer, true),
            AccountMeta::new(*proposal, false),
            AccountMeta::new(vote, false),
            AccountMeta::new_readonly(*vote_account, false),
            AccountMeta::new(vote_override_cache_pda(proposal, &vote), false),
            AccountMeta::new_readonly(NCN_ID, false),
            AccountMeta::new_readonly(consensus_result, false),
            AccountMeta::new_readonly(meta_merkle_proof_pda(&consensus_result, vote_account), false),
            AccountMeta::new_readonly(SYSTEM_ID, false),
        ],
    )
}

#[derive(BorshSerialize)]
struct CastVoteOverrideArgs {
    for_votes_bp: u64,
    against_votes_bp: u64,
    abstain_votes_bp: u64,
    stake_merkle_proof: Vec<[u8; 32]>,
    stake_merkle_leaf: StakeMerkleLeaf,
}

pub fn ix_cast_vote_override(
    signer: &Pubkey,
    proposal: &Pubkey,
    vote_account: &Pubkey,
    stake_account: &Pubkey,
    snapshot_slot: u64,
    for_bp: u64,
    against_bp: u64,
    abstain_bp: u64,
    stake_merkle_proof: Vec<[u8; 32]>,
    stake_merkle_leaf: StakeMerkleLeaf,
) -> Instruction {
    let consensus_result = consensus_result_pda(snapshot_slot);
    let validator_vote = vote_pda(proposal, vote_account);
    let args = CastVoteOverrideArgs {
        for_votes_bp: for_bp,
        against_votes_bp: against_bp,
        abstain_votes_bp: abstain_bp,
        stake_merkle_proof,
        stake_merkle_leaf,
    };
    anchor_ix(
        SVMGOV_ID,
        "cast_vote_override",
        &ser(&args),
        vec![
            AccountMeta::new(*signer, true),
            AccountMeta::new(*proposal, false),
            AccountMeta::new(validator_vote, false),
            AccountMeta::new_readonly(*vote_account, false),
            AccountMeta::new(vote_override_pda(proposal, stake_account, &validator_vote), false),
            AccountMeta::new(vote_override_cache_pda(proposal, &validator_vote), false),
            AccountMeta::new_readonly(*stake_account, false),
            AccountMeta::new_readonly(NCN_ID, false),
            AccountMeta::new_readonly(consensus_result, false),
            AccountMeta::new_readonly(meta_merkle_proof_pda(&consensus_result, vote_account), false),
            AccountMeta::new_readonly(SYSTEM_ID, false),
        ],
    )
}

pub fn ix_finalize_proposal(signer: &Pubkey, proposal: &Pubkey) -> Instruction {
    anchor_ix(
        SVMGOV_ID,
        "finalize_proposal",
        &[],
        vec![
            AccountMeta::new_readonly(*signer, true),
            AccountMeta::new(*proposal, false),
        ],
    )
}

// ---------------------------------------------------------------------------
// ncn instruction builders
// ---------------------------------------------------------------------------

pub fn ix_init_program_config(payer: &Pubkey, authority: &Pubkey, svmgov_program: &Pubkey) -> Instruction {
    anchor_ix(
        NCN_ID,
        "init_program_config",
        &ser(&svmgov_program.to_bytes()),
        vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new(ncn_program_config_pda(), false),
            AccountMeta::new_readonly(SYSTEM_ID, false),
        ],
    )
}

#[derive(BorshSerialize)]
struct UpdateProgramConfigArgs {
    proposed_authority: Option<Key>,
    min_consensus_threshold_bps: Option<u16>,
    tie_breaker_admin: Option<Key>,
    vote_duration: Option<i64>,
    svmgov_program_pubkey: Option<Key>,
}

pub fn ix_update_program_config(
    authority: &Pubkey,
    min_consensus_threshold_bps: Option<u16>,
    tie_breaker_admin: Option<Pubkey>,
    vote_duration: Option<i64>,
) -> Instruction {
    let args = UpdateProgramConfigArgs {
        proposed_authority: None,
        min_consensus_threshold_bps,
        tie_breaker_admin: tie_breaker_admin.map(|p| p.to_bytes()),
        vote_duration,
        svmgov_program_pubkey: None,
    };
    anchor_ix(
        NCN_ID,
        "update_program_config",
        &ser(&args),
        vec![
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new(ncn_program_config_pda(), false),
        ],
    )
}

#[derive(BorshSerialize)]
struct UpdateOperatorWhitelistArgs {
    operators_to_add: Option<Vec<Key>>,
    operators_to_remove: Option<Vec<Key>>,
}

pub fn ix_update_operator_whitelist(authority: &Pubkey, add: &[Pubkey]) -> Instruction {
    let args = UpdateOperatorWhitelistArgs {
        operators_to_add: Some(add.iter().map(|p| p.to_bytes()).collect()),
        operators_to_remove: None,
    };
    anchor_ix(
        NCN_ID,
        "update_operator_whitelist",
        &ser(&args),
        vec![
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new(ncn_program_config_pda(), false),
        ],
    )
}

pub fn ix_ncn_cast_vote(operator: &Pubkey, snapshot_slot: u64, ballot: Ballot) -> Instruction {
    anchor_ix(
        NCN_ID,
        "cast_vote",
        &ser(&ballot),
        vec![
            AccountMeta::new_readonly(*operator, true),
            AccountMeta::new(ballot_box_pda(snapshot_slot), false),
        ],
    )
}

pub fn ix_finalize_ballot(payer: &Pubkey, snapshot_slot: u64) -> Instruction {
    anchor_ix(
        NCN_ID,
        "finalize_ballot",
        &[],
        vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(ballot_box_pda(snapshot_slot), false),
            AccountMeta::new(consensus_result_pda(snapshot_slot), false),
            AccountMeta::new_readonly(SYSTEM_ID, false),
        ],
    )
}

#[derive(BorshSerialize)]
struct InitMetaMerkleProofArgs {
    meta_merkle_leaf: MetaMerkleLeaf,
    meta_merkle_proof: Vec<[u8; 32]>,
    close_timestamp: i64,
}

pub fn ix_init_meta_merkle_proof(
    payer: &Pubkey,
    snapshot_slot: u64,
    meta_merkle_leaf: MetaMerkleLeaf,
    meta_merkle_proof: Vec<[u8; 32]>,
    close_timestamp: i64,
) -> Instruction {
    let consensus_result = consensus_result_pda(snapshot_slot);
    let vote_account = Pubkey::new_from_array(meta_merkle_leaf.vote_account);
    let args = InitMetaMerkleProofArgs { meta_merkle_leaf, meta_merkle_proof, close_timestamp };
    anchor_ix(
        NCN_ID,
        "init_meta_merkle_proof",
        &ser(&args),
        vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(meta_merkle_proof_pda(&consensus_result, &vote_account), false),
            AccountMeta::new_readonly(consensus_result, false),
            AccountMeta::new_readonly(SYSTEM_ID, false),
        ],
    )
}

// ---------------------------------------------------------------------------
// Cheatcode setup wrappers
// ---------------------------------------------------------------------------

pub fn fund(cheats: &Cheatcodes, address: &Pubkey, lamports: u64) {
    cheats.fund_sol(address, lamports).expect("fund_sol");
}

/// Directly sets a vote account's epoch stake via the `surfnet_setEpochStake`
/// cheatcode — as read by the on-chain `sol_get_epoch_stake` syscall — without
/// modeling individual stake accounts.
pub fn set_epoch_stake(cheats: &Cheatcodes, vote_account: &Pubkey, stake: u64) {
    cheats
        .execute(SetEpochStake::new(*vote_account, stake))
        .expect("set_epoch_stake");
}

/// Creates a vote account whose validator/node identity is `node`.
///
/// `SetVoteAccount` writes a compact bincode `VoteStateVersions`, but the program
/// requires `data_len() == VoteState::size_of()`. So we read that valid data back
/// and rewrite it padded to `VOTE_STATE_SIZE` (matching a real on-chain vote
/// account); the rewrite re-indexes the vote account, preserving the node identity.
pub fn set_vote_account(cheats: &Cheatcodes, rpc: &RpcClient, vote_account: &Pubkey, node: &Pubkey) {
    cheats
        .execute(
            SetVoteAccount::new(*vote_account)
                .node_pubkey(*node)
                .authorized_voter(*node)
                .authorized_withdrawer(*node)
                .commission(0)
                .last_vote(1_000_000_000),
        )
        .expect("set_vote_account");

    let account = rpc.get_account(vote_account).expect("vote account exists");
    let mut data = account.data;
    assert!(data.len() <= VOTE_STATE_SIZE, "vote data larger than VoteState::size_of()");
    data.resize(VOTE_STATE_SIZE, 0);
    cheats
        .set_account(vote_account, account.lamports.max(SOL), &data, &VOTE_PROGRAM)
        .expect("pad vote account to VoteState::size_of()");
}

/// Creates a stake account delegated to `vote_account`, with `voting_wallet` as
/// the authorized withdrawer (this is the wallet recorded as the snapshot voting
/// wallet for the stake leaf).
pub fn set_stake_account(
    cheats: &Cheatcodes,
    stake_account: &Pubkey,
    vote_account: &Pubkey,
    voting_wallet: &Pubkey,
    stake: u64,
) {
    cheats
        .execute(
            SetStakeAccount::new(*stake_account)
                .voter_pubkey(*vote_account)
                .authorized_staker(*voting_wallet)
                .authorized_withdrawer(*voting_wallet)
                .stake(stake)
                .activation_epoch(0),
        )
        .expect("set_stake_account");
}

// ---------------------------------------------------------------------------
// Self-consistent sorted merkle tree (matches ncn_snapshot::merkle_helper::verify_helper)
// ---------------------------------------------------------------------------

pub mod merkle {
    use super::*;

    fn hashv(parts: &[&[u8]]) -> [u8; 32] {
        let mut h = Sha256::new();
        for p in parts {
            h.update(p);
        }
        h.finalize().into()
    }

    /// Leaf content hashes (NO prefix) — match the program's `*::hash()`.
    pub fn meta_leaf_content(l: &MetaMerkleLeaf) -> [u8; 32] {
        hashv(&[&l.voting_wallet, &l.vote_account, &l.stake_merkle_root, &l.active_stake.to_le_bytes()])
    }
    pub fn stake_leaf_content(l: &StakeMerkleLeaf) -> [u8; 32] {
        hashv(&[&l.voting_wallet, &l.stake_account, &l.active_stake.to_le_bytes()])
    }

    fn leaf_node(content: &[u8; 32]) -> [u8; 32] {
        hashv(&[&[0u8], content])
    }
    fn parent(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
        if a <= b {
            hashv(&[&[1u8], a, b])
        } else {
            hashv(&[&[1u8], b, a])
        }
    }

    /// Builds a sorted merkle tree over the given leaf-content hashes and returns
    /// the root plus the sibling proof for each leaf (bottom-up). Lone nodes are
    /// promoted unchanged — self-consistent with `verify_helper`.
    pub fn build(contents: &[[u8; 32]]) -> ([u8; 32], Vec<Vec<[u8; 32]>>) {
        let n = contents.len();
        assert!(n > 0, "merkle needs at least one leaf");
        let mut level: Vec<[u8; 32]> = contents.iter().map(leaf_node).collect();
        let mut proofs: Vec<Vec<[u8; 32]>> = vec![Vec::new(); n];
        let mut positions: Vec<usize> = (0..n).collect(); // leaf -> index in current level

        while level.len() > 1 {
            let mut next = Vec::new();
            let mut i = 0;
            while i < level.len() {
                if i + 1 < level.len() {
                    for (leaf, pos) in positions.iter().enumerate() {
                        if *pos == i {
                            proofs[leaf].push(level[i + 1]);
                        } else if *pos == i + 1 {
                            proofs[leaf].push(level[i]);
                        }
                    }
                    next.push(parent(&level[i], &level[i + 1]));
                } else {
                    next.push(level[i]);
                }
                i += 2;
            }
            for p in positions.iter_mut() {
                *p /= 2;
            }
            level = next;
        }
        (level[0], proofs)
    }
}

// ---------------------------------------------------------------------------
// Fake snapshot construction
// ---------------------------------------------------------------------------

pub struct StakeEntry {
    pub stake_account: Pubkey,
    pub voting_wallet: Pubkey,
    pub active_stake: u64,
}

pub struct ValidatorEntry {
    pub vote_account: Pubkey,
    pub voting_wallet: Pubkey,
    pub stakes: Vec<StakeEntry>,
}

/// A built fake snapshot: the meta root the operators vote on, plus per-validator
/// meta leaf + meta proof and per-stake leaf + stake proof.
pub struct FakeSnapshot {
    pub root: [u8; 32],
    pub validators: Vec<ValidatorBundle>,
}

pub struct ValidatorBundle {
    pub vote_account: Pubkey,
    pub meta_leaf: MetaMerkleLeaf,
    pub meta_proof: Vec<[u8; 32]>,
    pub stakes: Vec<StakeBundle>,
}

pub struct StakeBundle {
    pub leaf: StakeMerkleLeaf,
    pub proof: Vec<[u8; 32]>,
}

/// Builds the fake snapshot the same way the production generator does: a per-vote
/// stake tree feeding each MetaMerkleLeaf, then a meta tree over all validators.
pub fn build_fake_snapshot(validators: &[ValidatorEntry]) -> FakeSnapshot {
    // Per-validator stake trees + meta leaves.
    let mut meta_leaves: Vec<MetaMerkleLeaf> = Vec::new();
    let mut stake_bundles_per_validator: Vec<Vec<StakeBundle>> = Vec::new();

    for v in validators {
        let stake_leaves: Vec<StakeMerkleLeaf> = v
            .stakes
            .iter()
            .map(|s| StakeMerkleLeaf {
                voting_wallet: s.voting_wallet.to_bytes(),
                stake_account: s.stake_account.to_bytes(),
                active_stake: s.active_stake,
            })
            .collect();
        let contents: Vec<[u8; 32]> = stake_leaves.iter().map(merkle::stake_leaf_content).collect();
        let (stake_root, stake_proofs) = merkle::build(&contents);

        meta_leaves.push(MetaMerkleLeaf {
            voting_wallet: v.voting_wallet.to_bytes(),
            vote_account: v.vote_account.to_bytes(),
            stake_merkle_root: stake_root,
            active_stake: v.stakes.iter().map(|s| s.active_stake).sum(),
        });
        stake_bundles_per_validator.push(
            stake_leaves
                .into_iter()
                .zip(stake_proofs)
                .map(|(leaf, proof)| StakeBundle { leaf, proof })
                .collect(),
        );
    }

    // Meta tree over all validators.
    let meta_contents: Vec<[u8; 32]> = meta_leaves.iter().map(merkle::meta_leaf_content).collect();
    let (root, meta_proofs) = merkle::build(&meta_contents);

    let validators = validators
        .iter()
        .enumerate()
        .map(|(i, v)| ValidatorBundle {
            vote_account: v.vote_account,
            meta_leaf: meta_leaves[i].clone(),
            meta_proof: meta_proofs[i].clone(),
            stakes: std::mem::take(&mut stake_bundles_per_validator[i]),
        })
        .collect();

    FakeSnapshot { root, validators }
}

// ---------------------------------------------------------------------------
// Scenario setup
// ---------------------------------------------------------------------------

/// ncn defaults applied by [`setup_scenario`]. 6666 bps => 2-of-3 operator
/// consensus; a long vote window so operators can finish voting in-test.
pub const NCN_CONSENSUS_BPS: u16 = 6666;
pub const NCN_VOTE_DURATION: i64 = 10_000_000;

/// A delegated stake account: `amount` lamports delegated to a validator's vote
/// account, withdraw-authorized by `voting_wallet` (the wallet that signs vote
/// overrides — the test holds its keypair when it needs to sign).
pub struct StakeSpec {
    pub stake_account: Pubkey,
    pub voting_wallet: Pubkey,
    pub amount: u64,
}

/// A validator: its node `identity` keypair (signs create/support/vote), its
/// vote account, and the stake delegated to it.
pub struct ValidatorSpec {
    pub identity: Keypair,
    pub vote_account: Pubkey,
    pub stakes: Vec<StakeSpec>,
}

/// A full test scenario. Owns the keypairs the test signs with (admin +
/// validator identities + operators); stakers are referenced by `voting_wallet`
/// pubkey (the test keeps their keypairs to sign overrides).
pub struct Scenario {
    pub admin: Keypair,
    pub operators: Vec<Keypair>,
    pub validators: Vec<ValidatorSpec>,
    pub config: GovConfigArgs,
    /// Lamports funded to admin + each validator identity + each stake voting wallet.
    pub fund_lamports: u64,
}

impl Scenario {
    /// The validator/stake set as snapshot inputs (validator voting wallet = its
    /// node identity), for [`build_fake_snapshot`]. Built from the same amounts
    /// configured on-chain, so the snapshot agrees with the epoch stake.
    pub fn snapshot_validators(&self) -> Vec<ValidatorEntry> {
        self.validators
            .iter()
            .map(|v| ValidatorEntry {
                vote_account: v.vote_account,
                voting_wallet: v.identity.pubkey(),
                stakes: v
                    .stakes
                    .iter()
                    .map(|s| StakeEntry {
                        stake_account: s.stake_account,
                        voting_wallet: s.voting_wallet,
                        active_stake: s.amount,
                    })
                    .collect(),
            })
            .collect()
    }
}

/// Starts a surfnet and brings it to a ready state: both programs deployed (admin
/// = upgrade authority), signing wallets funded, svmgov + ncn configured, operator
/// whitelist set, and all vote/stake accounts configured. Tests drive the actual
/// flow (create/support/vote/...) from here. Does NOT advance the epoch — call
/// `cheats.time_travel_to_epoch(..)` as the flow requires.
pub async fn setup_scenario(scenario: &Scenario) -> Surfnet {
    let surfnet = Surfnet::start().await.expect("start surfnet");
    {
        let rpc = surfnet.rpc_client();
        let cheats = surfnet.cheatcodes();
        let admin = &scenario.admin;

        // Fund admin + every signing wallet (validator identities + stake voting wallets).
        let mut to_fund: std::collections::HashSet<Pubkey> = std::collections::HashSet::new();
        to_fund.insert(admin.pubkey());
        for v in &scenario.validators {
            to_fund.insert(v.identity.pubkey());
            for s in &v.stakes {
                to_fund.insert(s.voting_wallet);
            }
        }
        for pk in &to_fund {
            fund(&cheats, pk, scenario.fund_lamports);
        }

        // Deploy both programs with admin as the upgrade authority.
        deploy_program(&rpc, &SVMGOV_ID, SVMGOV_SO, &admin.pubkey()).expect("deploy svmgov");
        deploy_program(&rpc, &NCN_ID, NCN_SO, &admin.pubkey()).expect("deploy ncn");

        // Configure svmgov.
        send(&rpc, &[ix_initialize_config(&admin.pubkey(), &scenario.config)], &[admin]);
        send(&rpc, &[ix_initialize_index(&admin.pubkey())], &[admin]);

        // Configure ncn: link svmgov, set consensus threshold + vote duration, whitelist operators.
        send(&rpc, &[ix_init_program_config(&admin.pubkey(), &admin.pubkey(), &SVMGOV_ID)], &[admin]);
        send(
            &rpc,
            &[ix_update_program_config(
                &admin.pubkey(),
                Some(NCN_CONSENSUS_BPS),
                Some(admin.pubkey()),
                Some(NCN_VOTE_DURATION),
            )],
            &[admin],
        );
        if !scenario.operators.is_empty() {
            let op_keys: Vec<Pubkey> = scenario.operators.iter().map(|o| o.pubkey()).collect();
            send(&rpc, &[ix_update_operator_whitelist(&admin.pubkey(), &op_keys)], &[admin]);
        }

        // Configure vote + stake accounts via cheatcodes.
        for v in &scenario.validators {
            set_vote_account(&cheats, &rpc, &v.vote_account, &v.identity.pubkey());
            for s in &v.stakes {
                set_stake_account(&cheats, &s.stake_account, &v.vote_account, &s.voting_wallet, s.amount);
            }
        }
    }
    surfnet
}
