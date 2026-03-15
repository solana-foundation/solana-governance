use std::{collections::HashMap, fmt, fs, str::FromStr, sync::Arc, time::Duration};

use anchor_client::{
    Client, Cluster, Program,
    solana_client::nonblocking::rpc_client::RpcClient,
    solana_sdk::{native_token::LAMPORTS_PER_SOL, signature::Keypair, signer::Signer},
};
use anchor_lang::{Id, prelude::Pubkey};
use anyhow::{Result, anyhow};
use chrono::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};
use textwrap::wrap;

use crate::{
    constants::*,
    govcontract::{
        accounts::{Proposal, Vote},
        program::Govcontract,
    },
};

/// Creates and configures a progress spinner with a custom message
pub fn create_spinner(message: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
            .tick_strings(&["⠏", "⠇", "⠦", "⠴", "⠼", "⠸", "⠹", "⠙", "⠋", "⠓"]),
    );
    spinner.set_message(message.to_string());
    spinner.enable_steady_tick(Duration::from_millis(SPINNER_TICK_DURATION_MS));
    spinner
}

pub async fn setup_all(
    keypair_path: Option<String>,
    rpc_url: Option<String>,
) -> Result<(
    Arc<Keypair>,
    Pubkey,
    Program<Arc<Keypair>>,
    Program<Arc<Keypair>>,
)> {
    // Step 1: Load the identity keypair
    let identity_keypair = load_identity_keypair(keypair_path)?;
    let identity_keypair_arc = Arc::new(identity_keypair);

    // Step 2: Set the cluster
    let cluster = set_cluster(rpc_url);

    // Step 3: Create the Anchor client and program
    let client = Client::new(cluster.clone(), identity_keypair_arc.clone());
    let program = client.program(Govcontract::id())?;

    let merkle_proof_program = client.program(gov_v1::id())?;
    // Step 4: Find the vote account using the program's RpcClient
    let rpc_client = program.rpc();
    let validator_identity = identity_keypair_arc.pubkey();
    let vote_account = find_spl_vote_account(&validator_identity, &rpc_client).await?;

    // Step 5: Log the setup completion
    log::debug!(
        "setup_all completed successfully: payer_pubkey={}, vote_account={}",
        identity_keypair_arc.pubkey(),
        vote_account
    );

    // Return all variables
    Ok((
        identity_keypair_arc,
        vote_account,
        program,
        merkle_proof_program,
    ))
}

pub fn setup_all_with_staker(
    staker_keypair_path: String,
    rpc_url: Option<String>,
) -> Result<(Arc<Keypair>, Program<Arc<Keypair>>, Program<Arc<Keypair>>)> {
    // Step 1: Load the staker keypair
    let staker_keypair = load_staker_keypair(staker_keypair_path)?;
    let staker_keypair_arc = Arc::new(staker_keypair);

    // Step 2: Set the cluster
    let cluster = set_cluster(rpc_url);

    // Step 3: Create the Anchor client and program
    let client = Client::new(cluster.clone(), staker_keypair_arc.clone());
    let program = client.program(Govcontract::id())?;

    let merkle_proof_program = client.program(gov_v1::id())?;

    // Step 4: Log the setup completion
    log::debug!(
        "setup_all_with_staker completed successfully: staker_pubkey={}",
        staker_keypair_arc.pubkey()
    );

    // Return all variables
    Ok((staker_keypair_arc, program, merkle_proof_program))
}

fn load_staker_keypair(keypair_path: String) -> Result<Keypair> {
    let file_content = fs::read_to_string(&keypair_path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => {
            anyhow!(
                "The specified staker keypair file does not exist: {}",
                keypair_path
            )
        }
        _ => anyhow!("Failed to read staker keypair file {}: {}", keypair_path, e),
    })?;

    let keypair_bytes: Vec<u8> = serde_json::from_str(&file_content).map_err(|e| {
        anyhow!(
            "The staker keypair file is not a valid JSON array of bytes: {}. Error: {}",
            keypair_path,
            e
        )
    })?;

    // Create the Keypair from the bytes
    let staker_keypair = Keypair::from_bytes(&keypair_bytes).map_err(|e| {
        anyhow!(
            "The provided bytes do not form a valid Solana keypair: {}. This might be due to invalid key data.",
            e
        )
    })?;

    Ok(staker_keypair)
}

fn load_identity_keypair(keypair_path: Option<String>) -> Result<Keypair> {
    // Check if the keypair path is provided
    let identity_keypair_path = if let Some(path) = keypair_path {
        path
    } else {
        return Err(anyhow!(
            "No identity keypair path provided. Please specify the path using the --identity_keypair flag."
        ));
    };

    let file_content = fs::read_to_string(&identity_keypair_path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => {
            anyhow!(
                "The specified keypair file does not exist: {}",
                identity_keypair_path
            )
        }
        _ => anyhow!(
            "Failed to read keypair file {}: {}",
            identity_keypair_path,
            e
        ),
    })?;

    let keypair_bytes: Vec<u8> = serde_json::from_str(&file_content).map_err(|e| {
        anyhow!(
            "The keypair file is not a valid JSON array of bytes: {}. Error: {}",
            identity_keypair_path,
            e
        )
    })?;

    // Create the Keypair from the bytes
    let identity_keypair = Keypair::from_bytes(&keypair_bytes).map_err(|e| {
        anyhow!(
            "The provided bytes do not form a valid Solana keypair: {}. This might be due to invalid key data.",
            e
        )
    })?;

    println!(
        "Loaded identity keypair address -> {:?}",
        identity_keypair.pubkey()
    );

    Ok(identity_keypair)
}

async fn find_spl_vote_account(
    validator_identity: &Pubkey,
    rpc_client: &RpcClient,
) -> Result<Pubkey> {
    let vote_accounts = rpc_client.get_vote_accounts().await?;

    let vote_account = vote_accounts
        .current
        .iter()
        .find(|vote_acc| vote_acc.node_pubkey == validator_identity.to_string())
        .ok_or(anyhow!(
            "No Vote account found associated with this validator identity"
        ))?;

    Ok(Pubkey::from_str(&vote_account.vote_pubkey)?)
}

// Returns a vector with vote account pubkeys sequentially collected
pub async fn find_spl_vote_accounts(
    validator_identities: Vec<&Pubkey>,
    rpc_client: &RpcClient,
) -> Result<Vec<Pubkey>> {
    log::debug!(
        "find_spl_vote_accounts called with validator_identities: {:?}",
        validator_identities
    );

    let vote_accounts = rpc_client.get_vote_accounts().await?;
    log::debug!(
        "Fetched {} current vote accounts from RPC",
        vote_accounts.current.len()
    );

    let mut spl_vote_pubkeys = Vec::with_capacity(validator_identities.len());

    // Map of node_pubkey to vote_pubkey
    let vote_account_map = vote_accounts
        .current
        .iter()
        .filter_map(|vote_acc| {
            Pubkey::from_str(&vote_acc.node_pubkey)
                .ok()
                .map(|pk| Ok((pk, Pubkey::from_str(&vote_acc.vote_pubkey)?)))
        })
        .collect::<Result<HashMap<_, _>>>()?;
    log::debug!(
        "Constructed vote_account_map with {} entries",
        vote_account_map.len()
    );

    // Build the result in the order of validator_identities
    for identity in validator_identities {
        if let Some(vote_pubkey) = vote_account_map.get(identity) {
            log::debug!(
                "Found SPL vote pubkey {} for validator identity {}",
                vote_pubkey,
                identity
            );
            spl_vote_pubkeys.push(*vote_pubkey);
        } else {
            log::debug!(
                "No SPL vote account found for validator identity {}",
                identity
            );
            return Err(anyhow!(
                "No SPL vote account found for validator identity {}",
                identity
            ));
        }
    }

    log::debug!("Returning SPL vote pubkeys: {:?}", spl_vote_pubkeys);
    Ok(spl_vote_pubkeys)
}

fn set_cluster(rpc_url: Option<String>) -> Cluster {
    if let Some(rpc_url) = rpc_url {
        let wss_url = rpc_url.replace("https://", "wss://");
        Cluster::Custom(rpc_url, wss_url)
    } else {
        Cluster::Custom(DEFAULT_RPC_URL.to_string(), DEFAULT_WSS_URL.to_string())
    }
}

pub fn anchor_client_setup(
    rpc_url: Option<String>,
    payer: Arc<Keypair>,
) -> Result<Program<Arc<Keypair>>> {
    // Set up the cluster
    let cluster = set_cluster(rpc_url);

    // Create the Anchor client
    let client = Client::new(cluster, payer.clone());
    let program = client.program(Govcontract::id())?;
    Ok(program)
}

impl fmt::Display for Proposal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let author_str = self.author.to_string();
        let short_author = format!(
            "{}...{}",
            &author_str[..4],
            &author_str[author_str.len() - 4..]
        );
        let wrapped_desc = wrap(&self.description, 80);

        writeln!(f, "{:<25} {}", "Proposal:", self.title)?;
        writeln!(f, "{:<25} {}", "Author:", short_author)?;
        writeln!(f, "{:<25} epoch {}", "Created:", self.creation_epoch)?;
        writeln!(f, "{:<25} epoch {}", "Starts:", self.start_epoch)?;
        writeln!(f, "{:<25} epoch {}", "Ends:", self.end_epoch)?;
        writeln!(
            f,
            "{:<25} {} bp ({:.2}%)",
            "Proposer Stake Weight:",
            self.proposer_stake_weight_bp,
            self.proposer_stake_weight_bp as f64 / 100.0
        )?;
        writeln!(
            f,
            "{:<25} {} lamports (~{:.2} SOL)",
            "Cluster Support:",
            self.cluster_support_lamports,
            self.cluster_support_lamports / LAMPORTS_PER_SOL
        )?;
        writeln!(
            f,
            "{:<25} {} lamports (~{:.2} SOL)",
            "For Votes:",
            self.for_votes_lamports,
            self.for_votes_lamports / LAMPORTS_PER_SOL
        )?;
        writeln!(
            f,
            "{:<25} {} lamports (~{:.2} SOL)",
            "Against Votes:",
            self.against_votes_lamports,
            self.against_votes_lamports / LAMPORTS_PER_SOL
        )?;
        writeln!(
            f,
            "{:<25} {} lamports (~{:.2} SOL)",
            "Abstain Votes:",
            self.abstain_votes_lamports,
            self.abstain_votes_lamports / LAMPORTS_PER_SOL
        )?;
        writeln!(
            f,
            "{:<25} {}",
            "Voting:",
            if self.voting { "Yes" } else { "No" }
        )?;
        writeln!(
            f,
            "{:<25} {}",
            "Finalized:",
            if self.finalized { "Yes" } else { "No" }
        )?;

        writeln!(f, "{:<25}", "Description:")?;
        for line in wrapped_desc {
            writeln!(f, "  {}", line)?;
        }
        Ok(())
    }
}

impl fmt::Display for Vote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let validator_str = self.validator.to_string();
        let short_validator = format!(
            "{}...{}",
            &validator_str[..4],
            &validator_str[validator_str.len() - 4..]
        );
        let proposal_str = self.proposal.to_string();
        let short_proposal = format!(
            "{}...{}",
            &proposal_str[..4],
            &proposal_str[proposal_str.len() - 4..]
        );
        let timestamp = Utc
            .timestamp_opt(self.vote_timestamp, 0)
            .single()
            .unwrap_or_default();
        let formatted_timestamp = timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string();

        writeln!(f, "{:<15} {}", "Validator:", short_validator)?;
        writeln!(f, "{:<15} {}", "Proposal:", short_proposal)?;
        writeln!(
            f,
            "{:<15} {} bp ({:.2}%)",
            "For Votes:",
            self.for_votes_bp,
            self.for_votes_bp as f64 / 100.0
        )?;
        writeln!(
            f,
            "{:<15} {} bp ({:.2}%)",
            "Against Votes:",
            self.against_votes_bp,
            self.against_votes_bp as f64 / 100.0
        )?;
        writeln!(
            f,
            "{:<15} {} bp ({:.2}%)",
            "Abstain Votes:",
            self.abstain_votes_bp,
            self.abstain_votes_bp as f64 / 100.0
        )?;
        writeln!(f, "{:<15} {}", "Timestamp:", formatted_timestamp)?;
        Ok(())
    }
}

pub fn derive_vote_pda(
    proposal_pubkey: &Pubkey,
    vote_account: &Pubkey,
    program_id: &Pubkey,
) -> Pubkey {
    let seeds = &[b"vote", proposal_pubkey.as_ref(), vote_account.as_ref()];
    let (pda, _) = Pubkey::find_program_address(seeds, program_id);
    pda
}

pub fn derive_proposal_pda(seed: u64, vote_account: &Pubkey, program_id: &Pubkey) -> Pubkey {
    let seeds = &[b"proposal", &seed.to_le_bytes(), vote_account.as_ref()];
    let (pda, _) = Pubkey::find_program_address(seeds, program_id);
    pda
}

pub fn derive_proposal_index_pda(program_id: &Pubkey) -> Pubkey {
    let seeds = &[&b"index"[..]];
    let (pda, _) = Pubkey::find_program_address(seeds, program_id);
    pda
}

/// Derives the Support PDA using the seeds [b"support", proposal, spl_vote_account]
/// This matches the on-chain derivation in the support_proposal instruction.
pub fn derive_support_pda(
    proposal_pubkey: &Pubkey,
    vote_account: &Pubkey,
    program_id: &Pubkey,
) -> Pubkey {
    let seeds = &[b"support", proposal_pubkey.as_ref(), vote_account.as_ref()];
    let (pda, _) = Pubkey::find_program_address(seeds, program_id);
    pda
}

pub fn derive_vote_override_pda(
    proposal_pubkey: &Pubkey,
    stake_account: &Pubkey,
    validator_vote_pda: &Pubkey,
    program_id: &Pubkey,
) -> Pubkey {
    let seeds = &[
        b"vote_override",
        proposal_pubkey.as_ref(),
        stake_account.as_ref(),
        validator_vote_pda.as_ref(),
    ];
    let (pda, _) = Pubkey::find_program_address(seeds, program_id);
    pda
}

pub fn derive_vote_override_cache_pda(
    proposal_pubkey: &Pubkey,
    validator_vote_pda: &Pubkey,
    program_id: &Pubkey,
) -> Pubkey {
    let seeds = &[
        b"vote_override_cache",
        proposal_pubkey.as_ref(),
        validator_vote_pda.as_ref(),
    ];
    let (pda, _) = Pubkey::find_program_address(seeds, program_id);
    pda
}
/// Derives the ProgramConfig PDA using the seeds [b"ProgramConfig"]
/// This matches the on-chain derivation in the support_proposal instruction.
pub fn derive_program_config_pda(ballot_program_id: &Pubkey) -> Pubkey {
    let seeds = &[b"ProgramConfig".as_ref()];
    let (pda, _) = Pubkey::find_program_address(seeds, ballot_program_id);
    pda
}

pub fn get_epoch_slot_range(epoch: u64) -> (u64, u64) {
    const SLOTS_PER_EPOCH: u64 = 432_000;

    let start_slot = epoch * SLOTS_PER_EPOCH;
    let end_slot = (epoch + 1) * SLOTS_PER_EPOCH - 1;

    (start_slot, end_slot)
}
