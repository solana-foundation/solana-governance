use std::str::FromStr;

use anchor_client::solana_sdk::{pubkey::Pubkey, signer::Signer};
use anyhow::{anyhow, Result};

use crate::{
    svmgov_program::{accounts::Proposal, client::{accounts, args}},
    utils::{
        proposal_link::validate_description,
        utils::{create_spinner, derive_global_config_pda, setup_signer_and_program},
    },
};

/// Updates the commit-pinned document URL of an eligible proposal.
pub async fn update_proposal_description(
    proposal_id: String,
    description: String,
    identity_keypair: Option<String>,
    rpc_url: Option<String>,
    skip_link_check: bool,
) -> Result<()> {
    log::debug!(
        "update_proposal_description: proposal_id={}, description={}, identity_keypair={:?}, rpc_url={:?}, skip_link_check={}",
        proposal_id,
        description,
        identity_keypair,
        rpc_url,
        skip_link_check
    );
    let description = validate_description(&description, skip_link_check).await?;
    let proposal_pubkey = Pubkey::from_str(&proposal_id)
        .map_err(|_| anyhow!("Invalid proposal ID: {proposal_id}"))?;
    let (payer, program, _client) = setup_signer_and_program(identity_keypair, rpc_url)?;

    let proposal = program
        .account::<Proposal>(proposal_pubkey)
        .await
        .map_err(|e| anyhow!("Failed to fetch proposal {proposal_pubkey}: {e}"))?;
    if proposal.author != payer.pubkey() {
        return Err(anyhow!(
            "Only proposal author {} can update this proposal; signer is {}",
            proposal.author,
            payer.pubkey()
        ));
    }

    let spinner = create_spinner("Updating proposal document...");
    let signature = program
        .request()
        .args(args::UpdateProposalDescription { description: description.clone() })
        .accounts(accounts::UpdateProposalDescription {
            signer: payer.pubkey(),
            proposal: proposal_pubkey,
            global_config: derive_global_config_pda(&program.id()),
        })
        .send()
        .await?;

    spinner.finish_with_message(format!(
        "Proposal document updated to {description}. https://explorer.solana.com/tx/{signature}"
    ));
    Ok(())
}
