use anchor_client::solana_sdk::{signer::Signer, system_program};
use anyhow::Result;

use crate::{
    svmgov_program::client::{accounts, args},
    utils::utils::{create_spinner, derive_proposal_index_pda, setup_admin},
};

pub async fn initialize_index(
    identity_keypair: Option<String>,
    rpc_url: Option<String>,
) -> Result<()> {
    // init-index is permissionless on-chain: the signer only pays rent for the
    // ProposalIndex PDA. Use setup_admin (no vote-account lookup) rather than
    // setup_all, which requires the signer to be a validator identity.
    let (payer, program) = setup_admin(identity_keypair, rpc_url)?;

    let proposal_index = derive_proposal_index_pda(&program.id());

    let spinner = create_spinner("Sending init_index transaction...");

    let sig = program
        .request()
        .args(args::InitializeIndex {})
        .accounts(accounts::InitializeIndex {
            signer: payer.pubkey(),
            proposal_index,
            system_program: system_program::ID,
        })
        .send()
        .await?;
    log::debug!("Transaction sent successfully: signature={}", sig);

    spinner.finish_with_message(format!(
        "Proposal index initialized successfully. https://explorer.solana.com/tx/{}",
        sig
    ));

    log::debug!("init_index completed successfully");
    Ok(())
}
