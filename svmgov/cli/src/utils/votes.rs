//! Individual votes on a proposal, for display by the CLI.
//!
//! The proposal detail view (`svmgov proposal`) only had aggregate For / Against
//! / Abstain totals. Who cast those votes lives on `Vote` (validator) and
//! `VoteOverride` (staker) accounts, filtered on-chain by the proposal pubkey.
//! This module maps those accounts into plain [`VoteRecord`]s and renders them
//! as a table — the mapping is unit-testable without an RPC.

use std::cmp::Ordering;

use anchor_client::solana_account_decoder::UiAccountEncoding;
use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use anchor_client::solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use anchor_client::solana_client::rpc_filter::{Memcmp, RpcFilterType};
use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::native_token::LAMPORTS_PER_SOL;
use anchor_lang::{AccountDeserialize, Discriminator, prelude::Pubkey};
use anyhow::{Result, anyhow};
use chrono::TimeZone;
use chrono::Utc;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::{Cell, CellAlignment, Table, presets::UTF8_FULL};

use crate::svmgov_program::accounts::{Vote, VoteOverride};

/// Byte offset of `Vote.proposal`: 8-byte discriminator + 32-byte validator.
pub const VOTE_PROPOSAL_OFFSET: usize = 40;

/// Byte offset of `VoteOverride.proposal`: 8-byte discriminator + three pubkeys
/// (delegator, stake_account, validator).
pub const VOTE_OVERRIDE_PROPOSAL_OFFSET: usize = 104;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoterKind {
    Validator,
    Staker,
}

impl VoterKind {
    pub fn label(self) -> &'static str {
        match self {
            VoterKind::Validator => "Validator",
            VoterKind::Staker => "Staker",
        }
    }
}

/// One row in the proposal votes table. Copied off the on-chain account so
/// formatting and sorting do not depend on the generated types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoteRecord {
    /// Validator identity (voting wallet) for validator votes; delegator for
    /// staker overrides.
    pub voter: String,
    pub kind: VoterKind,
    /// Present only for staker overrides: the stake account that voted.
    pub stake_account: Option<String>,
    pub for_votes_bp: u64,
    pub against_votes_bp: u64,
    pub abstain_votes_bp: u64,
    /// Snapshot stake attached to this vote. For validators this is
    /// `Vote.stake` minus `Vote.override_lamports`, matching the weight that
    /// actually votes with the validator's split.
    pub stake_lamports: u64,
    pub timestamp: i64,
}

impl VoteRecord {
    pub fn from_vote(vote: &Vote) -> Self {
        Self {
            voter: vote.validator.to_string(),
            kind: VoterKind::Validator,
            stake_account: None,
            for_votes_bp: vote.for_votes_bp,
            against_votes_bp: vote.against_votes_bp,
            abstain_votes_bp: vote.abstain_votes_bp,
            stake_lamports: vote.stake.saturating_sub(vote.override_lamports),
            timestamp: vote.vote_timestamp,
        }
    }

    pub fn from_override(vote: &VoteOverride) -> Self {
        Self {
            voter: vote.delegator.to_string(),
            kind: VoterKind::Staker,
            stake_account: Some(vote.stake_account.to_string()),
            for_votes_bp: vote.for_votes_bp,
            against_votes_bp: vote.against_votes_bp,
            abstain_votes_bp: vote.abstain_votes_bp,
            stake_lamports: vote.stake_amount,
            timestamp: vote.vote_override_timestamp,
        }
    }
}

pub fn format_basis_points(bp: u64) -> String {
    format!("{:.2}%", bp as f64 / 100.0)
}

pub fn format_stake_sol(lamports: u64) -> String {
    format!("{:.2} SOL", lamports as f64 / LAMPORTS_PER_SOL as f64)
}

pub fn format_vote_timestamp(ts: i64) -> String {
    Utc.timestamp_opt(ts, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| ts.to_string())
}

/// Highest stake first, then voter, then stake account so equal-stake rows are
/// stable across runs.
pub fn sort_vote_records(records: &mut [VoteRecord]) {
    records.sort_by(|a, b| {
        b.stake_lamports
            .cmp(&a.stake_lamports)
            .then_with(|| a.voter.cmp(&b.voter))
            .then_with(|| match (&a.stake_account, &b.stake_account) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Less,
                (Some(_), None) => Ordering::Greater,
                (Some(x), Some(y)) => x.cmp(y),
            })
    });
}

pub fn votes_summary(records: &[VoteRecord]) -> String {
    let validators = records
        .iter()
        .filter(|r| r.kind == VoterKind::Validator)
        .count();
    let stakers = records.len().saturating_sub(validators);
    match (validators, stakers) {
        (0, 0) => "No votes have been cast on this proposal.".to_string(),
        (v, 0) => format!("Votes ({} validator{}):", v, plural(v)),
        (0, s) => format!("Votes ({} staker override{}):", s, plural(s)),
        (v, s) => format!(
            "Votes ({} validator{}, {} staker override{}):",
            v,
            plural(v),
            s,
            plural(s)
        ),
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

pub fn vote_row_cells(record: &VoteRecord) -> [String; 8] {
    [
        record.voter.clone(),
        record.kind.label().to_string(),
        record
            .stake_account
            .clone()
            .unwrap_or_else(|| "—".to_string()),
        format_stake_sol(record.stake_lamports),
        format_basis_points(record.for_votes_bp),
        format_basis_points(record.against_votes_bp),
        format_basis_points(record.abstain_votes_bp),
        format_vote_timestamp(record.timestamp),
    ]
}

pub fn print_votes_table(records: &[VoteRecord], terminal_width: u16) {
    println!("\n{}", votes_summary(records));
    if records.is_empty() {
        return;
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_width(terminal_width);

    table.set_header(vec![
        "Voter",
        "Type",
        "Stake Account",
        "Stake",
        "For",
        "Against",
        "Abstain",
        "Date",
    ]);

    // Full pubkeys wrap into unreadable fragments unless the columns keep
    // content width, matching the proposal-list ID column.
    for column in [0, 2] {
        if let Some(col) = table.column_mut(column) {
            col.set_constraint(comfy_table::ColumnConstraint::ContentWidth);
        }
    }

    for record in records {
        let cells = vote_row_cells(record);
        table.add_row(vec![
            Cell::new(&cells[0]),
            Cell::new(&cells[1]),
            Cell::new(&cells[2]),
            Cell::new(&cells[3]).set_alignment(CellAlignment::Right),
            Cell::new(&cells[4]).set_alignment(CellAlignment::Right),
            Cell::new(&cells[5]).set_alignment(CellAlignment::Right),
            Cell::new(&cells[6]).set_alignment(CellAlignment::Right),
            Cell::new(&cells[7]),
        ]);
    }

    println!("{}", table);
}

/// Fetch validator votes and staker overrides for `proposal`, mapped into
/// [`VoteRecord`]s. Does not sort — call [`sort_vote_records`] before display.
pub async fn fetch_vote_records(
    rpc: &RpcClient,
    program_id: &Pubkey,
    proposal: &Pubkey,
) -> Result<Vec<VoteRecord>> {
    let votes = fetch_program_accounts::<Vote>(rpc, program_id, VOTE_PROPOSAL_OFFSET, proposal)
        .await?
        .into_iter()
        .map(|(_, vote)| VoteRecord::from_vote(&vote));

    let overrides = fetch_program_accounts::<VoteOverride>(
        rpc,
        program_id,
        VOTE_OVERRIDE_PROPOSAL_OFFSET,
        proposal,
    )
    .await?
    .into_iter()
    .map(|(_, vote)| VoteRecord::from_override(&vote));

    Ok(votes.chain(overrides).collect())
}

async fn fetch_program_accounts<T: AccountDeserialize + Discriminator>(
    rpc: &RpcClient,
    program_id: &Pubkey,
    proposal_offset: usize,
    proposal: &Pubkey,
) -> Result<Vec<(Pubkey, T)>> {
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![
            RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, T::DISCRIMINATOR.to_vec())),
            RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                proposal_offset,
                proposal.to_bytes().to_vec(),
            )),
        ]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            ..Default::default()
        },
        ..Default::default()
    };

    let accounts = rpc
        .get_program_accounts_with_config(program_id, config)
        .await
        .map_err(|e| anyhow!("Failed to fetch vote accounts: {}", e))?;

    Ok(accounts
        .into_iter()
        .filter_map(
            |(pubkey, account)| match T::try_deserialize(&mut account.data.as_slice()) {
                Ok(parsed) => Some((pubkey, parsed)),
                Err(e) => {
                    log::warn!("Failed to deserialize vote account {}: {}", pubkey, e);
                    None
                }
            },
        )
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::AnchorSerialize;

    fn pubkey(byte: u8) -> Pubkey {
        Pubkey::new_from_array([byte; 32])
    }

    fn record(voter: &str, kind: VoterKind, stake: u64) -> VoteRecord {
        VoteRecord {
            voter: voter.to_string(),
            kind,
            stake_account: None,
            for_votes_bp: 10_000,
            against_votes_bp: 0,
            abstain_votes_bp: 0,
            stake_lamports: stake,
            timestamp: 1_700_000_000,
        }
    }

    fn sample_vote(proposal: Pubkey) -> Vote {
        Vote {
            validator: pubkey(1),
            proposal,
            for_votes_bp: 6_000,
            against_votes_bp: 3_000,
            abstain_votes_bp: 1_000,
            for_votes_lamports: 6,
            against_votes_lamports: 3,
            abstain_votes_lamports: 1,
            stake: 42,
            override_lamports: 0,
            vote_timestamp: 1_700_000_000,
            bump: 255,
        }
    }

    fn sample_override(proposal: Pubkey) -> VoteOverride {
        VoteOverride {
            delegator: pubkey(2),
            stake_account: pubkey(3),
            validator: pubkey(4),
            proposal,
            vote_account_validator: pubkey(5),
            for_votes_bp: 10_000,
            against_votes_bp: 0,
            abstain_votes_bp: 0,
            for_votes_lamports: 10,
            against_votes_lamports: 0,
            abstain_votes_lamports: 0,
            stake_amount: 7,
            vote_override_timestamp: 1_700_000_100,
            bump: 254,
        }
    }

    #[test]
    fn vote_proposal_offset_matches_serialized_account() {
        let proposal = pubkey(7);
        let vote = sample_vote(proposal);
        let mut data = Vote::DISCRIMINATOR.to_vec();
        vote.serialize(&mut data).unwrap();
        assert_eq!(
            &data[VOTE_PROPOSAL_OFFSET..VOTE_PROPOSAL_OFFSET + 32],
            proposal.as_ref()
        );
    }

    #[test]
    fn vote_override_proposal_offset_matches_serialized_account() {
        let proposal = pubkey(9);
        let vote = sample_override(proposal);
        let mut data = VoteOverride::DISCRIMINATOR.to_vec();
        vote.serialize(&mut data).unwrap();
        assert_eq!(
            &data[VOTE_OVERRIDE_PROPOSAL_OFFSET..VOTE_OVERRIDE_PROPOSAL_OFFSET + 32],
            proposal.as_ref()
        );
    }

    #[test]
    fn from_vote_maps_validator_identity_and_split() {
        let vote = sample_vote(pubkey(7));
        let record = VoteRecord::from_vote(&vote);
        assert_eq!(record.voter, pubkey(1).to_string());
        assert_eq!(record.kind, VoterKind::Validator);
        assert_eq!(record.stake_account, None);
        assert_eq!(record.for_votes_bp, 6_000);
        assert_eq!(record.against_votes_bp, 3_000);
        assert_eq!(record.abstain_votes_bp, 1_000);
        assert_eq!(record.stake_lamports, 42);
        assert_eq!(record.timestamp, 1_700_000_000);
    }

    #[test]
    fn from_vote_excludes_overridden_stake() {
        let mut vote = sample_vote(pubkey(7));
        vote.stake = 100;
        vote.override_lamports = 30;
        assert_eq!(VoteRecord::from_vote(&vote).stake_lamports, 70);
    }

    #[test]
    fn from_override_maps_delegator_and_stake_account() {
        let vote = sample_override(pubkey(9));
        let record = VoteRecord::from_override(&vote);
        assert_eq!(record.voter, pubkey(2).to_string());
        assert_eq!(record.kind, VoterKind::Staker);
        assert_eq!(record.stake_account, Some(pubkey(3).to_string()));
        assert_eq!(record.for_votes_bp, 10_000);
        assert_eq!(record.stake_lamports, 7);
        assert_eq!(record.timestamp, 1_700_000_100);
    }

    #[test]
    fn basis_points_render_as_percentages() {
        assert_eq!(format_basis_points(0), "0.00%");
        assert_eq!(format_basis_points(6_000), "60.00%");
        assert_eq!(format_basis_points(10_000), "100.00%");
        assert_eq!(format_basis_points(1), "0.01%");
    }

    #[test]
    fn stake_renders_in_sol() {
        assert_eq!(format_stake_sol(0), "0.00 SOL");
        assert_eq!(format_stake_sol(LAMPORTS_PER_SOL), "1.00 SOL");
        assert_eq!(format_stake_sol(1_500_000_000), "1.50 SOL");
    }

    #[test]
    fn timestamp_renders_in_utc() {
        assert_eq!(format_vote_timestamp(0), "1970-01-01 00:00 UTC");
        assert_eq!(format_vote_timestamp(1_700_000_000), "2023-11-14 22:13 UTC");
    }

    #[test]
    fn sort_puts_highest_stake_first_and_breaks_ties_by_voter() {
        let mut records = vec![
            record("b", VoterKind::Validator, 100),
            record("a", VoterKind::Validator, 100),
            record("c", VoterKind::Staker, 500),
            record("d", VoterKind::Validator, 50),
        ];
        sort_vote_records(&mut records);
        let order: Vec<&str> = records.iter().map(|r| r.voter.as_str()).collect();
        assert_eq!(order, vec!["c", "a", "b", "d"]);
    }

    #[test]
    fn summary_names_validators_and_staker_overrides() {
        assert_eq!(
            votes_summary(&[]),
            "No votes have been cast on this proposal."
        );
        assert_eq!(
            votes_summary(&[record("a", VoterKind::Validator, 1)]),
            "Votes (1 validator):"
        );
        assert_eq!(
            votes_summary(&[
                record("a", VoterKind::Validator, 1),
                record("b", VoterKind::Validator, 2),
            ]),
            "Votes (2 validators):"
        );
        assert_eq!(
            votes_summary(&[record("a", VoterKind::Staker, 1)]),
            "Votes (1 staker override):"
        );
        assert_eq!(
            votes_summary(&[
                record("a", VoterKind::Validator, 1),
                record("b", VoterKind::Staker, 2),
                record("c", VoterKind::Staker, 3),
            ]),
            "Votes (1 validator, 2 staker overrides):"
        );
    }

    #[test]
    fn row_cells_show_who_voted_and_the_split() {
        let vote = VoteRecord::from_vote(&sample_vote(pubkey(7)));
        let cells = vote_row_cells(&vote);
        assert_eq!(cells[0], pubkey(1).to_string());
        assert_eq!(cells[1], "Validator");
        assert_eq!(cells[2], "—");
        assert_eq!(cells[3], "0.00 SOL");
        assert_eq!(cells[4], "60.00%");
        assert_eq!(cells[5], "30.00%");
        assert_eq!(cells[6], "10.00%");
        assert_eq!(cells[7], "2023-11-14 22:13 UTC");

        let staker = VoteRecord::from_override(&sample_override(pubkey(9)));
        let cells = vote_row_cells(&staker);
        assert_eq!(cells[0], pubkey(2).to_string());
        assert_eq!(cells[1], "Staker");
        assert_eq!(cells[2], pubkey(3).to_string());
        assert_eq!(cells[4], "100.00%");
        assert_eq!(cells[5], "0.00%");
        assert_eq!(cells[6], "0.00%");
    }
}
