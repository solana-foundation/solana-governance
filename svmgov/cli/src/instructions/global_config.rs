use std::sync::Arc;

use anchor_client::solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};
use anchor_lang::system_program;
use anyhow::Result;

use crate::{
    svmgov_program::client::{accounts, args},
    utils::utils::{
        anchor_client_setup, create_spinner, derive_global_config_pda, fetch_global_config,
        setup_all,
    },
};

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
) -> Result<()> {
    let (payer, _vote_account, program, _) = setup_all(keypair, rpc_url).await?;

    let global_config_pda = derive_global_config_pda(&program.id());

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
        })
        .accounts(accounts::InitializeConfig {
            admin: payer.pubkey(),
            global_config: global_config_pda,
            system_program: system_program::ID,
        })
        .instructions()?;

    let blockhash = program.rpc().get_latest_blockhash().await?;
    let transaction =
        Transaction::new_signed_with_payer(&ixs, Some(&payer.pubkey()), &[&payer], blockhash);

    let sig = program
        .rpc()
        .send_and_confirm_transaction(&transaction)
        .await?;

    spinner.finish_with_message(format!(
        "Global config initialized. https://explorer.solana.com/tx/{}",
        sig
    ));

    Ok(())
}

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
) -> Result<()> {
    let (payer, _vote_account, program, _) = setup_all(keypair, rpc_url).await?;

    let global_config_pda = derive_global_config_pda(&program.id());

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
        })
        .accounts(accounts::UpdateConfig {
            admin: payer.pubkey(),
            global_config: global_config_pda,
        })
        .instructions()?;

    let blockhash = program.rpc().get_latest_blockhash().await?;
    let transaction =
        Transaction::new_signed_with_payer(&ixs, Some(&payer.pubkey()), &[&payer], blockhash);

    let sig = program
        .rpc()
        .send_and_confirm_transaction(&transaction)
        .await?;

    spinner.finish_with_message(format!(
        "Global config updated. https://explorer.solana.com/tx/{}",
        sig
    ));

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

    Ok(())
}
