use std::sync::Arc;

use anchor_client::solana_sdk::{signature::Keypair, signer::Signer};
use anchor_lang::system_program;
use anyhow::Result;

use crate::{
    svmgov_program::client::{accounts, args},
    utils::{
        squads::{effective_signer, SquadsCliOpts},
        utils::{
            anchor_client_setup, create_spinner, derive_global_config_pda, fetch_global_config,
            setup_admin,
        },
    },
};

#[allow(clippy::too_many_arguments)]
pub async fn initialize_global_config(
    keypair: Option<String>,
    rpc_url: Option<String>,
    max_title_length: u16,
    max_description_length: u16,
    max_support_epochs: u64,
    min_proposal_stake_lamports: u64,
    cluster_support_pct_min_bps: u64,
    discussion_epochs: u64,
    voting_epochs: u64,
    snapshot_epoch_extension: u64,
    snapshot_slot_offset: i64,
    squads: Option<SquadsCliOpts>,
) -> Result<()> {
    let (payer, program) = setup_admin(keypair, rpc_url)?;

    let global_config_pda = derive_global_config_pda(&program.id());
    let admin = effective_signer(squads.as_ref(), payer.pubkey());

    let spinner = create_spinner("Initializing global config...");

    let ixs = program
        .request()
        .args(args::InitializeConfig {
            max_title_length,
            max_description_length,
            max_support_epochs,
            min_proposal_stake_lamports,
            cluster_support_pct_min_bps,
            discussion_epochs,
            voting_epochs,
            snapshot_epoch_extension,
            snapshot_slot_offset,
        })
        .accounts(accounts::InitializeConfig {
            admin,
            global_config: global_config_pda,
            system_program: system_program::ID,
        })
        .instructions()?;

    let rpc = program.rpc();
    let squads_config = squads.as_ref().map(|opts| opts.to_config(payer.pubkey()));
    let outcome = crate::utils::squads::route(
        &rpc,
        ixs,
        Vec::new(),
        &[payer.as_ref()],
        squads_config.as_ref(),
    )
    .await?;

    spinner.finish_and_clear();
    println!("{}", outcome.format_structured());

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn update_global_config(
    keypair: Option<String>,
    rpc_url: Option<String>,
    max_title_length: Option<u16>,
    max_description_length: Option<u16>,
    max_support_epochs: Option<u64>,
    min_proposal_stake_lamports: Option<u64>,
    cluster_support_pct_min_bps: Option<u64>,
    discussion_epochs: Option<u64>,
    voting_epochs: Option<u64>,
    snapshot_epoch_extension: Option<u64>,
    snapshot_slot_offset: Option<i64>,
    squads: Option<SquadsCliOpts>,
) -> Result<()> {
    let (payer, program) = setup_admin(keypair, rpc_url)?;

    let global_config_pda = derive_global_config_pda(&program.id());
    let admin = effective_signer(squads.as_ref(), payer.pubkey());

    let spinner = create_spinner("Updating global config...");

    let ixs = program
        .request()
        .args(args::UpdateConfig {
            max_title_length,
            max_description_length,
            max_support_epochs,
            min_proposal_stake_lamports,
            cluster_support_pct_min_bps,
            discussion_epochs,
            voting_epochs,
            snapshot_epoch_extension,
            snapshot_slot_offset,
        })
        .accounts(accounts::UpdateConfig {
            admin,
            global_config: global_config_pda,
        })
        .instructions()?;

    let rpc = program.rpc();
    let squads_config = squads.as_ref().map(|opts| opts.to_config(payer.pubkey()));
    let outcome = crate::utils::squads::route(
        &rpc,
        ixs,
        Vec::new(),
        &[payer.as_ref()],
        squads_config.as_ref(),
    )
    .await?;

    spinner.finish_and_clear();
    println!("{}", outcome.format_structured());

    Ok(())
}

pub async fn show_global_config(
    rpc_url: Option<String>,
) -> Result<()> {
    let mock_payer = Arc::new(Keypair::new());
    let program = anchor_client_setup(rpc_url, mock_payer)?;

    let config = fetch_global_config(&program).await?;

    println!("\nOn-chain Global Config:");
    println!("  admin:                       {}", config.admin);
    println!("  max_title_length:            {}", config.max_title_length);
    println!("  max_description_length:      {}", config.max_description_length);
    println!("  max_support_epochs:          {}", config.max_support_epochs);
    println!("  min_proposal_stake_lamports: {}", config.min_proposal_stake_lamports);
    println!("  cluster_support_pct_min_bps: {}", config.cluster_support_pct_min_bps);
    println!("  discussion_epochs:           {}", config.discussion_epochs);
    println!("  voting_epochs:               {}", config.voting_epochs);
    println!("  snapshot_epoch_extension:    {}", config.snapshot_epoch_extension);
    println!("  snapshot_slot_offset:        {}", config.snapshot_slot_offset);

    Ok(())
}
