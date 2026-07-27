use std::str::FromStr;

use anchor_client::solana_sdk::{pubkey::Pubkey, signer::Signer};
use anyhow::{anyhow, Result};

use crate::{
    svmgov_program::client::{accounts, args},
    utils::utils::{create_spinner, setup_signer_and_program},
};

pub async fn finalize_proposal(
    proposal_id: String,
    identity_keypair: Option<String>,
    rpc_url: Option<String>,
) -> Result<()> {
    let proposal_pubkey = Pubkey::from_str(&proposal_id)
        .map_err(|_| anyhow!("Invalid proposal ID: {}", proposal_id))?;

    let (payer, program, _client) = setup_signer_and_program(identity_keypair, rpc_url)?;

    let spinner = create_spinner("Finalizing proposal...");

    let sig = program
        .request()
        .args(args::FinalizeProposal {})
        .accounts(accounts::FinalizeProposal {
            signer: payer.pubkey(),
            proposal: proposal_pubkey,
        })
        .send()
        .await?;

    spinner.finish_with_message(format!(
        "Proposal finalized successfully. https://explorer.solana.com/tx/{}",
        sig
    ));

    Ok(())
}
