use std::{str::FromStr, sync::Arc};

use anchor_client::solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use anchor_client::solana_client::rpc_filter::{Memcmp, RpcFilterType};
use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::signature::Keypair;

use anchor_lang::{prelude::Pubkey, AccountDeserialize, Discriminator};
use anyhow::{Result, anyhow};
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::{Cell, Table, presets::UTF8_FULL};
use serde::{Deserialize, Serialize};

use crate::{anchor_client_setup, govcontract::accounts::Proposal};

/// Detect terminal width using various methods
fn detect_terminal_width() -> Option<u16> {
    // Method 1: Check COLUMNS environment variable
    if let Ok(cols) = std::env::var("COLUMNS") {
        if let Ok(width) = cols.parse::<u16>() {
            return Some(width);
        }
    }

    // Method 2: Try tput command (Unix-like systems)
    #[cfg(unix)]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("tput").arg("cols").output() {
            if let Ok(s) = String::from_utf8(output.stdout) {
                if let Ok(width) = s.trim().parse::<u16>() {
                    return Some(width);
                }
            }
        }
    }

    // Method 3: Try stty command (Unix-like systems)
    #[cfg(unix)]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("stty").arg("size").output() {
            if let Ok(s) = String::from_utf8(output.stdout) {
                let parts: Vec<&str> = s.trim().split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(width) = parts[1].parse::<u16>() {
                        return Some(width);
                    }
                }
            }
        }
    }

    None
}

pub async fn get_proposal(rpc_url: Option<String>, proposal_id: &String) -> Result<()> {
    // Parse the proposal ID into a Pubkey
    let proposal_pubkey = Pubkey::from_str(proposal_id)
        .map_err(|_| anyhow!("Invalid proposal ID: {}", proposal_id))?;
    // Create a mock Payer
    let mock_payer = Arc::new(Keypair::new());

    // Create the Anchor client
    let program = anchor_client_setup(rpc_url, mock_payer)?;

    let rpc = program.rpc();
    let current_epoch = rpc
        .get_epoch_info()
        .await
        .map_err(|e| anyhow!("Failed to fetch epoch info: {}", e))?
        .epoch;

    let proposal_acc = program.account::<Proposal>(proposal_pubkey).await?;

    print_proposal_detail(proposal_id, &proposal_acc, current_epoch);

    Ok(())
}

fn print_proposal_detail(proposal_id: &str, proposal: &Proposal, current_epoch: u64) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic);

    // Set table width based on terminal size
    // Try multiple methods to detect terminal width
    let terminal_width = detect_terminal_width().unwrap_or(120);
    table.set_width(terminal_width);

    table.set_header(vec!["Field", "Value"]);

    // With ContentArrangement::Dynamic, comfy-table automatically handles column widths
    // and wraps long text in the Value column appropriately

    let for_sol = proposal.for_votes_lamports as f64 / 1_000_000_000.0;
    let against_sol = proposal.against_votes_lamports as f64 / 1_000_000_000.0;
    let abstain_sol = proposal.abstain_votes_lamports as f64 / 1_000_000_000.0;
    let cluster_support_sol = proposal.cluster_support_lamports as f64 / 1_000_000_000.0;
    println!("{:?}", proposal.cluster_support_lamports);
    let proposer_stake_bp = proposal.proposer_stake_weight_bp as f64 / 100.0;

    let status = if proposal.finalized {
        "Finalized"
    } else if current_epoch >= proposal.end_epoch {
        "Ended"
    } else if proposal.voting {
        "Voting"
    } else {
        "Support Period"
    };

    table.add_row(vec![Cell::new("Proposal ID"), Cell::new(proposal_id)]);
    table.add_row(vec![Cell::new("Title"), Cell::new(&proposal.title)]);
    table.add_row(vec![
        Cell::new("Description"),
        Cell::new(&proposal.description),
    ]);
    table.add_row(vec![
        Cell::new("Author"),
        Cell::new(proposal.author.to_string()),
    ]);
    table.add_row(vec![Cell::new("Status"), Cell::new(status)]);
    table.add_row(vec![
        Cell::new("Index"),
        Cell::new(proposal.index.to_string()),
    ]);
    table.add_row(vec![
        Cell::new("Creation Epoch"),
        Cell::new(proposal.creation_epoch.to_string()),
    ]);
    table.add_row(vec![
        Cell::new("Start Epoch"),
        Cell::new(proposal.start_epoch.to_string()),
    ]);
    table.add_row(vec![
        Cell::new("End Epoch"),
        Cell::new(proposal.end_epoch.to_string()),
    ]);
    table.add_row(vec![
        Cell::new("Snapshot Slot"),
        Cell::new(proposal.snapshot_slot.to_string()),
    ]);
    table.add_row(vec![
        Cell::new("Proposer Stake Weight"),
        Cell::new(format!("{:.2}%", proposer_stake_bp)),
    ]);
    table.add_row(vec![
        Cell::new("Cluster Support"),
        Cell::new(format!("{:.2} SOL", cluster_support_sol)),
    ]);
    table.add_row(vec![
        Cell::new("Vote Count"),
        Cell::new(proposal.vote_count.to_string()),
    ]);
    table.add_row(vec![
        Cell::new("For Votes"),
        Cell::new(format!(
            "{} lamports ({:.2} SOL)",
            proposal.for_votes_lamports, for_sol
        )),
    ]);
    table.add_row(vec![
        Cell::new("Against Votes"),
        Cell::new(format!(
            "{} lamports ({:.2} SOL)",
            proposal.against_votes_lamports, against_sol
        )),
    ]);
    table.add_row(vec![
        Cell::new("Abstain Votes"),
        Cell::new(format!(
            "{} lamports ({:.2} SOL)",
            proposal.abstain_votes_lamports, abstain_sol
        )),
    ]);
    if let Some(consensus_result) = proposal.consensus_result {
        table.add_row(vec![
            Cell::new("Consensus Result"),
            Cell::new(consensus_result.to_string()),
        ]);
    }
    table.add_row(vec![
        Cell::new("Creation Timestamp"),
        Cell::new(proposal.creation_timestamp.to_string()),
    ]);

    println!("\n{}", table);
}

#[derive(Serialize, Deserialize)]
struct ProposalOutput {
    id: String,
    title: String,
    description: String,
    author: String,
    status: String,
    index: u32,
    creation_epoch: u64,
    start_epoch: u64,
    end_epoch: u64,
    snapshot_slot: u64,
    proposer_stake_weight_bp: u64,
    cluster_support_lamports: u64,
    for_votes_lamports: u64,
    against_votes_lamports: u64,
    abstain_votes_lamports: u64,
    vote_count: u32,
    voting: bool,
    finalized: bool,
    creation_timestamp: i64,
}

fn get_proposal_status(proposal: &Proposal, current_epoch: u64) -> &'static str {
    if proposal.finalized {
        "finalized"
    } else if current_epoch >= proposal.end_epoch {
        "ended"
    } else if proposal.voting {
        "active"
    } else {
        "support"
    }
}

pub async fn list_proposals(
    rpc_url: Option<String>,
    status_filter: Option<String>,
    limit: Option<usize>,
    json_output: bool,
) -> Result<()> {
    // Create a mock Payer
    let mock_payer = Arc::new(Keypair::new());

    // Create the Anchor client
    let program = anchor_client_setup(rpc_url.clone(), mock_payer.clone())?;

    // Get the RPC client
    let rpc = program.rpc();
    let program_id = program.id();

    // Fetch current epoch to determine ended proposals
    let current_epoch = rpc
        .get_epoch_info()
        .await
        .map_err(|e| anyhow!("Failed to fetch epoch info: {}", e))?
        .epoch;

    // Use memcmp filter on the Proposal account discriminator
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            0,
            Proposal::DISCRIMINATOR.to_vec(),
        ))]),
        account_config: RpcAccountInfoConfig {
            commitment: Some(CommitmentConfig::confirmed()),
            ..Default::default()
        },
        ..Default::default()
    };

    let accounts = rpc
        .get_program_accounts_with_config(&program_id, config)
        .await
        .map_err(|e| anyhow!("Failed to fetch proposal accounts: {}", e))?;

    let mut proposals: Vec<(Pubkey, Proposal)> = accounts
        .into_iter()
        .filter_map(|(pubkey, account)| {
            match Proposal::try_deserialize(&mut account.data.as_slice()) {
                Ok(proposal) => Some((pubkey, proposal)),
                Err(e) => {
                    log::warn!("Failed to deserialize proposal account {}: {}", pubkey, e);
                    None
                }
            }
        })
        .collect();

    if proposals.is_empty() {
        if json_output {
            println!("[]");
        } else {
            println!("No proposals found.");
        }
        return Ok(());
    }

    // Sort by creation timestamp (most recent first)
    proposals.sort_by(|a, b| b.1.creation_timestamp.cmp(&a.1.creation_timestamp));

    // Filter by status if provided
    if let Some(status) = status_filter {
        let status_lower = status.to_lowercase();
        proposals.retain(|(_, proposal)| {
            let proposal_status = get_proposal_status(proposal, current_epoch);
            proposal_status == status_lower.as_str()
        });
    }

    // Apply limit if provided
    if let Some(limit_val) = limit {
        proposals.truncate(limit_val);
    }

    if proposals.is_empty() {
        if json_output {
            println!("[]");
        } else {
            println!("No proposals found matching the criteria.");
        }
        return Ok(());
    }

    // Output in JSON format if requested
    if json_output {
        let json_proposals: Vec<ProposalOutput> = proposals
            .iter()
            .map(|(pubkey, proposal)| ProposalOutput {
                id: pubkey.to_string(),
                title: proposal.title.clone(),
                description: proposal.description.clone(),
                author: proposal.author.to_string(),
                status: get_proposal_status(proposal, current_epoch).to_string(),
                index: proposal.index,
                creation_epoch: proposal.creation_epoch,
                start_epoch: proposal.start_epoch,
                end_epoch: proposal.end_epoch,
                snapshot_slot: proposal.snapshot_slot,
                proposer_stake_weight_bp: proposal.proposer_stake_weight_bp,
                cluster_support_lamports: proposal.cluster_support_lamports,
                for_votes_lamports: proposal.for_votes_lamports,
                against_votes_lamports: proposal.against_votes_lamports,
                abstain_votes_lamports: proposal.abstain_votes_lamports,
                vote_count: proposal.vote_count,
                voting: proposal.voting,
                finalized: proposal.finalized,
                creation_timestamp: proposal.creation_timestamp,
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_proposals)?);
    } else {
        print_proposals_table(&proposals, current_epoch);
    }

    Ok(())
}

fn print_proposals_table(proposals: &[(Pubkey, Proposal)], current_epoch: u64) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS);

    // Use a wider terminal width to accommodate full IDs without wrapping
    let terminal_width = detect_terminal_width().unwrap_or(200);
    table.set_width(terminal_width);

    table.set_header(vec!["ID", "Title", "Status"]);

    // Set ContentWidth constraint for ID column to prevent wrapping
    if let Some(column) = table.column_mut(0) {
        column.set_constraint(comfy_table::ColumnConstraint::ContentWidth);
    }

    for (pubkey, proposal) in proposals {
        let status = if proposal.finalized {
            "Finalized"
        } else if current_epoch >= proposal.end_epoch {
            "Ended"
        } else if proposal.voting {
            "Voting"
        } else {
            "Support"
        };

        // Truncate title if too long
        let title = if proposal.title.len() > 40 {
            format!("{}...", &proposal.title[..37])
        } else {
            proposal.title.clone()
        };

        let pubkey_str = pubkey.to_string();

        table.add_row(vec![
            Cell::new(pubkey_str),
            Cell::new(title),
            Cell::new(status),
        ]);
    }

    println!("\nFound {} proposal(s):\n", proposals.len());
    println!("{}", table);
    println!("\nTo view details of a specific proposal, use:");
    println!("  svmgov proposal <PROPOSAL_ID>");
}
