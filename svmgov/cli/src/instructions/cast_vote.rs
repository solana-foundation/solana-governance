use std::str::FromStr;

use anchor_client::solana_sdk::{pubkey::Pubkey, signer::Signer, transaction::Transaction};
use anchor_lang::system_program;
use anyhow::{Result, anyhow};
use gov_v1::{ID as SNAPSHOT_PROGRAM_ID, MetaMerkleLeaf, MetaMerkleProof};
use log::info;

use crate::{
    constants::*,
    govcontract::{accounts::Proposal, client::{accounts, args}},
    utils::{
        api_helpers::{self, get_vote_account_proof},
        utils::{create_spinner, derive_vote_override_cache_pda, derive_vote_pda, setup_all},
    },
};

pub async fn cast_vote(
    proposal_id: String,
    votes_for: u64,
    votes_against: u64,
    abstain: u64,
    identity_keypair: Option<String>,
    rpc_url: Option<String>,
    network: String,
) -> Result<()> {
    if votes_for + votes_against + abstain != BASIS_POINTS_TOTAL {
        return Err(anyhow!(
            "Total vote basis points must sum to {}",
            BASIS_POINTS_TOTAL
        ));
    }

    let proposal_pubkey = Pubkey::from_str(&proposal_id)
        .map_err(|_| anyhow!("Invalid proposal ID: {}", proposal_id))?;

    let (payer, vote_account, program, merkle_proof_program) =
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
    let vote_override_cache_pda =
        derive_vote_override_cache_pda(&proposal_pubkey, &vote_pda, &program.id());

    // Check if meta merkle proof account exists, create if missing
    let meta_merkle_proof_account = match program
        .account::<MetaMerkleProof>(meta_merkle_proof_pda)
        .await
    {
        Ok(account) => Some(account),
        Err(_e) => {
            info!("Unable to get meta merkle proof account, will create it");
            None
        }
    };

    // First transaction: Initialize meta merkle proof if needed
    if meta_merkle_proof_account.is_none() {
        info!("Creating meta merkle proof account");

        let init_spinner = create_spinner("Initializing meta merkle proof...");

        let voting_wallet = Pubkey::from_str(&proof_response.meta_merkle_leaf.voting_wallet)
            .map_err(|e| anyhow!("Invalid voting wallet in proof: {}", e))?;

        let init_meta_merkle_proof_ix = merkle_proof_program
            .request()
            .args(gov_v1::instruction::InitMetaMerkleProof {
                close_timestamp: 1,
                meta_merkle_leaf: MetaMerkleLeaf {
                    voting_wallet,
                    vote_account,
                    stake_merkle_root: Pubkey::from_str_const(
                        proof_response.meta_merkle_leaf.stake_merkle_root.as_str(),
                    )
                    .to_bytes(),
                    active_stake: proof_response.meta_merkle_leaf.active_stake,
                },
                meta_merkle_proof: proof_response
                    .meta_merkle_proof
                    .iter()
                    .map(|s| Pubkey::from_str_const(s).to_bytes())
                    .collect(),
            })
            .accounts(gov_v1::accounts::InitMetaMerkleProof {
                consensus_result: consensus_result_pda,
                merkle_proof: meta_merkle_proof_pda,
                payer: payer.pubkey(),
                system_program: system_program::ID,
            })
            .instructions()?;

        let blockhash = merkle_proof_program.rpc().get_latest_blockhash().await?;
        let transaction = Transaction::new_signed_with_payer(
            &init_meta_merkle_proof_ix,
            Some(&payer.pubkey()),
            &[&payer],
            blockhash,
        );

        let sig = merkle_proof_program
            .rpc()
            .send_and_confirm_transaction(&transaction)
            .await?;
        log::debug!(
            "Meta merkle proof initialization transaction sent successfully: signature={}",
            sig
        );

        init_spinner.finish_with_message(format!(
            "Meta merkle proof initialized. https://explorer.solana.com/tx/{}",
            sig
        ));
    }

    // Second transaction: Cast vote
    let spinner = create_spinner("Sending cast-vote transaction...");

    let cast_vote_ixs = program
        .request()
        .args(args::CastVote {
            for_votes_bp: votes_for,
            against_votes_bp: votes_against,
            abstain_votes_bp: abstain,
        })
        .accounts(accounts::CastVote {
            signer: payer.pubkey(),
            spl_vote_account: vote_account,
            proposal: proposal_pubkey,
            vote: vote_pda,
            vote_override_cache: vote_override_cache_pda,
            consensus_result: consensus_result_pda,
            meta_merkle_proof: meta_merkle_proof_pda,
            snapshot_program: SNAPSHOT_PROGRAM_ID,
            system_program: system_program::ID,
        })
        .instructions()?;

    let blockhash = program.rpc().get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &cast_vote_ixs,
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );

    let sig = program
        .rpc()
        .send_and_confirm_transaction(&transaction)
        .await?;
    log::debug!(
        "Cast vote transaction sent successfully: signature={}",
        sig
    );

    spinner.finish_with_message(format!(
        "Vote cast successfully. https://explorer.solana.com/tx/{}",
        sig
    ));

    Ok(())
}
