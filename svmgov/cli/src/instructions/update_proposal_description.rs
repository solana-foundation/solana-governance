use std::str::FromStr;

use anchor_client::solana_sdk::{pubkey::Pubkey, signer::Signer};
use anyhow::{anyhow, Result};

use crate::{
    svmgov_program::{
        accounts::Proposal,
        client::{accounts, args},
    },
    utils::{
        proposal_link::{validate_description, validate_description_structure},
        squads::{SquadsCliOpts, effective_signer, route},
        utils::{
            create_spinner, derive_global_config_pda, derive_vote_delegation_pda,
            fetch_global_config, fetch_vote_delegation, setup_signer_and_program,
        },
    },
};

/// Updates the commit-pinned document URL of an eligible proposal. Signed by
/// the proposal author, or by the accepted vote delegate of its vote account;
/// with `squads` the vault PDA is that signer.
pub async fn update_proposal_description(
    proposal_id: String,
    description: String,
    identity_keypair: Option<String>,
    rpc_url: Option<String>,
    skip_link_check: bool,
    squads: Option<SquadsCliOpts>,
) -> Result<()> {
    log::debug!(
        "update_proposal_description: proposal_id={}, description={}, identity_keypair={:?}, rpc_url={:?}, skip_link_check={}",
        proposal_id,
        description,
        identity_keypair,
        rpc_url,
        skip_link_check
    );
    validate_description_structure(&description)?;
    let proposal_pubkey = Pubkey::from_str(&proposal_id)
        .map_err(|_| anyhow!("Invalid proposal ID: {proposal_id}"))?;
    let (payer, program, _client) = setup_signer_and_program(identity_keypair, rpc_url)?;
    let signer = effective_signer(squads.as_ref(), payer.pubkey());
    let config = fetch_global_config(&program).await?;
    let description =
        validate_description(&description, config.max_description_length, skip_link_check).await?;

    let proposal = program
        .account::<Proposal>(proposal_pubkey)
        .await
        .map_err(|e| anyhow!("Failed to fetch proposal {proposal_pubkey}: {e}"))?;
    // With an accepted delegation on the proposal's vote account only the
    // delegate may update it; otherwise only the author (the vote account's
    // identity) may.
    match fetch_vote_delegation(&program, &proposal.vote_account_pubkey).await? {
        Some(delegation) if delegation.delegation_accepted => {
            if delegation.delegate != signer {
                return Err(anyhow!(
                    "Vote account {} has delegated its governance vote to {}; only that delegate \
                     can update this proposal (signer is {signer})",
                    proposal.vote_account_pubkey,
                    delegation.delegate,
                ));
            }
        }
        _ => {
            if proposal.author != signer {
                return Err(anyhow!(
                    "Only proposal author {} can update this proposal; signer is {signer}",
                    proposal.author,
                ));
            }
        }
    }

    let spinner = create_spinner("Updating proposal document...");
    let ixs = program
        .request()
        .args(args::UpdateProposalDescription {
            description: description.clone(),
        })
        .accounts(accounts::UpdateProposalDescription {
            signer,
            proposal: proposal_pubkey,
            vote_delegation: derive_vote_delegation_pda(
                &proposal.vote_account_pubkey,
                &program.id(),
            ),
            global_config: derive_global_config_pda(&program.id()),
        })
        .instructions()?;

    let rpc = program.rpc();
    let squads_config = squads.as_ref().map(|opts| opts.to_config(payer.pubkey()));
    let outcome = route(
        &rpc,
        ixs,
        Vec::new(),
        &[payer.as_ref()],
        squads_config.as_ref(),
    )
    .await?;

    spinner.finish_and_clear();
    println!("{}", outcome.format_structured());
    println!("Updating the document of proposal {proposal_pubkey} to {description}.");
    Ok(())
}
