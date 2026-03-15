
use anchor_client::solana_sdk::{signer::Signer, transaction::Transaction};
use anchor_lang::system_program;
use anyhow::Result;

use crate::{
    govcontract::client::{accounts, args},
    utils::utils::{create_spinner, derive_proposal_index_pda, derive_proposal_pda, setup_all},
};

pub async fn create_proposal(
    proposal_title: String,
    proposal_description: String,
    seed: Option<u64>,
    identity_keypair: Option<String>,
    rpc_url: Option<String>,
    _network: String,
) -> Result<()> {
    log::debug!(
        "create_proposal: title={}, description={}, seed={:?}, identity_keypair={:?}, rpc_url={:?}",
        proposal_title,
        proposal_description,
        seed,
        identity_keypair,
        rpc_url
    );

    let (payer, vote_account, program, _merkle_proof_program) =
        setup_all(identity_keypair, rpc_url).await?;

    let seed_value = seed.unwrap_or_else(rand::random::<u64>);

    let proposal_pda = derive_proposal_pda(seed_value, &vote_account, &program.id());

    let proposal_index_pda = derive_proposal_index_pda(&program.id());

    // Create proposal - snapshot_slot and consensus_result will be set later in support_proposal
    let spinner = create_spinner("Creating proposal...");

    let create_proposal_ixs = program
        .request()
        .args(args::CreateProposal {
            title: proposal_title,
            description: proposal_description,
            seed: seed_value,
        })
        .accounts(accounts::CreateProposal {
            signer: payer.pubkey(),
            spl_vote_account: vote_account,
            proposal: proposal_pda,
            proposal_index: proposal_index_pda,
            system_program: system_program::ID,
        })
        .instructions()?;

    let blockhash = program.rpc().get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &create_proposal_ixs,
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );

    let sig = program
        .rpc()
        .send_and_confirm_transaction(&transaction)
        .await?;
    log::debug!(
        "Proposal creation transaction sent successfully: signature={}",
        sig
    );

    spinner.finish_with_message(format!(
        "Proposal {} created. https://explorer.solana.com/tx/{}",
        proposal_pda, sig
    ));

    Ok(())
}
