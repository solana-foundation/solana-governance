use anchor_client::solana_sdk::{signer::Signer, transaction::Transaction};
use anchor_lang::system_program;
use anyhow::Result;

use crate::{
    instructions::support_proposal::build_support_proposal_instructions,
    svmgov_program::client::{accounts, args},
    utils::proposal_link::validate_description,
    utils::utils::{
        create_spinner, derive_global_config_pda, derive_proposal_index_pda, derive_proposal_pda,
        setup_all,
    },
};

pub async fn create_proposal(
    proposal_title: String,
    proposal_description: String,
    seed: Option<u64>,
    identity_keypair: Option<String>,
    rpc_url: Option<String>,
    _network: String,
    skip_link_check: bool,
    with_support: bool,
) -> Result<()> {
    log::debug!(
        "create_proposal: title={}, description={}, seed={:?}, identity_keypair={:?}, rpc_url={:?}, with_support={}",
        proposal_title,
        proposal_description,
        seed,
        identity_keypair,
        rpc_url,
        with_support
    );

    // Validated before loading a keypair, touching RPC, or starting a spinner, so a bad link
    // fails immediately and cleanly rather than as an opaque custom program error. The
    // normalized link is what gets submitted, so the value we checked is the value the program
    // sees — a description with surrounding whitespace would otherwise fail on chain.
    let proposal_description = validate_description(&proposal_description, skip_link_check).await?;

    let (payer, vote_account, program, _merkle_proof_program) =
        setup_all(identity_keypair, rpc_url).await?;

    let seed_value = seed.unwrap_or_else(rand::random::<u64>);

    let proposal_pda = derive_proposal_pda(seed_value, &vote_account, &program.id());

    let proposal_index_pda = derive_proposal_index_pda(&program.id());
    let global_config_pda = derive_global_config_pda(&program.id());

    // Create proposal - snapshot_slot and consensus_result will be set later in support_proposal
    let spinner = create_spinner(if with_support {
        "Creating and supporting proposal..."
    } else {
        "Creating proposal..."
    });

    let mut instructions = program
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
            global_config: global_config_pda,
            system_program: system_program::ID,
        })
        .instructions()?;

    if with_support {
        let support_proposal_ixs = build_support_proposal_instructions(
            &program,
            payer.pubkey(),
            proposal_pda,
            vote_account,
        )
        .await?;

        instructions.extend(support_proposal_ixs);
    }

    let blockhash = program.rpc().get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
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
        "Proposal {} created{}. https://explorer.solana.com/tx/{}",
        proposal_pda,
        if with_support { " and supported" } else { "" },
        sig
    ));

    Ok(())
}
