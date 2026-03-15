use std::str::FromStr;

use anchor_client::solana_sdk::{pubkey::Pubkey, signer::Signer};
use anchor_lang::system_program;
use anyhow::{Result, anyhow};
use gov_v1::ID as SNAPSHOT_PROGRAM_ID;

use crate::{
    constants::*,
    govcontract::{accounts::Proposal, client::{accounts, args}},
    utils::{
        api_helpers::{self, get_vote_account_proof},
        utils::{create_spinner, derive_vote_pda, setup_all},
    },
};

pub async fn modify_vote(
    proposal_id: String,
    for_votes: u64,
    against_votes: u64,
    abstain_votes: u64,
    identity_keypair: Option<String>,
    rpc_url: Option<String>,
    network: String,
) -> Result<()> {
    if for_votes + against_votes + abstain_votes != BASIS_POINTS_TOTAL {
        return Err(anyhow!(
            "Total vote basis points must sum to {}",
            BASIS_POINTS_TOTAL
        ));
    }

    let proposal_pubkey = Pubkey::from_str(&proposal_id)
        .map_err(|_| anyhow!("Invalid proposal ID: {}", proposal_id))?;

    let (payer, vote_account, program, _merkle_proof_program) =
        setup_all(identity_keypair, rpc_url).await?;

    // Fetch proposal to get snapshot_slot and consensus_result
    let proposal = program
        .account::<Proposal>(proposal_pubkey)
        .await
        .map_err(|e| anyhow!("Failed to fetch proposal: {}", e))?;

    let snapshot_slot = proposal.snapshot_slot;
    let consensus_result_pda = proposal
        .consensus_result
        .ok_or_else(|| anyhow!("Proposal consensus_result is not set"))?;

    let proof_response =
        get_vote_account_proof(&vote_account.to_string(), snapshot_slot, &network).await?;

    // Generate meta_merkle_proof_pda using the consensus_result from proposal
    let vote_account_pubkey = Pubkey::from_str(&proof_response.meta_merkle_leaf.vote_account)
        .map_err(|e| anyhow!("Invalid vote_account pubkey in response: {}", e))?;
    let meta_merkle_proof_pda = api_helpers::generate_meta_merkle_proof_pda(&consensus_result_pda, &vote_account_pubkey)?;

    let vote_pda = derive_vote_pda(&proposal_pubkey, &vote_account, &program.id());

    let spinner = create_spinner("Modifying vote...");

    let sig = program
        .request()
        .args(args::ModifyVote {
            for_votes_bp: for_votes,
            against_votes_bp: against_votes,
            abstain_votes_bp: abstain_votes,
        })
        .accounts(accounts::ModifyVote {
            signer: payer.pubkey(),
            spl_vote_account: vote_account,
            proposal: proposal_pubkey,
            vote: vote_pda,
            consensus_result: consensus_result_pda,
            meta_merkle_proof: meta_merkle_proof_pda,
            snapshot_program: SNAPSHOT_PROGRAM_ID,
            system_program: system_program::ID,
        })
        .send()
        .await?;

    spinner.finish_with_message(format!(
        "Vote modified successfully. https://explorer.solana.com/tx/{}",
        sig
    ));

    Ok(())
}
