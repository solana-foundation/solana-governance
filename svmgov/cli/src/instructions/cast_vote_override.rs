use std::str::FromStr;

use anchor_client::solana_sdk::{pubkey::Pubkey, signer::Signer};
use anchor_lang::system_program;
use anyhow::{Result, anyhow};
use ncn_snapshot::{ID as SNAPSHOT_PROGRAM_ID, MetaMerkleLeaf, MetaMerkleProof};
use log::info;

use crate::{
    constants::*,
    svmgov_program::{
        accounts::Proposal,
        client::{accounts, args},
    },
    utils::{
        api_helpers::{
            self, convert_merkle_proof_strings, convert_stake_merkle_leaf_data_to_idl_type,
            get_stake_account_proof, get_vote_account_proof,
        },
        squads::{effective_signer, SquadsCliOpts},
        utils::{
            compute_vote_expiry_timestamp, create_spinner, derive_vote_override_cache_pda,
            derive_vote_override_pda, derive_vote_pda, setup_all_with_staker,
        },
    },
};

#[allow(clippy::too_many_arguments)]
pub async fn cast_vote_override(
    proposal_id: String,
    for_votes: u64,
    against_votes: u64,
    abstain_votes: u64,
    staker_keypair: String,
    rpc_url: Option<String>,
    stake_account_override: String,
    vote_account: String,
    network: String,
    squads: Option<SquadsCliOpts>,
    close_timestamp_override: Option<i64>,
) -> Result<()> {
    if for_votes + against_votes + abstain_votes != BASIS_POINTS_TOTAL {
        return Err(anyhow!(
            "Total vote basis points must sum to {}",
            BASIS_POINTS_TOTAL
        ));
    }

    let proposal_pubkey = Pubkey::from_str(&proposal_id)
        .map_err(|_| anyhow!("Invalid proposal ID: {}", proposal_id))?;

    let (payer, program, merkle_proof_program) = setup_all_with_staker(staker_keypair, rpc_url)?;

    // Fetch proposal to get snapshot_slot and consensus_result
    let proposal = program
        .account::<Proposal>(proposal_pubkey)
        .await
        .map_err(|e| anyhow!("Failed to fetch proposal: {}", e))?;

    let snapshot_slot = proposal.snapshot_slot;
    let consensus_result_pda = proposal
        .consensus_result
        .ok_or_else(|| anyhow!("Proposal consensus_result is not set"))?;

    let stake_account_str = stake_account_override.clone();
    let vote_account_pubkey = Pubkey::from_str(&vote_account)
        .map_err(|_| anyhow!("Invalid vote account: {}", vote_account))?;

    let meta_merkle_proof = get_vote_account_proof(&vote_account, snapshot_slot, &network).await?;

    let stake_merkle_proof =
        get_stake_account_proof(&stake_account_str, snapshot_slot, &network).await?;

    // Generate meta_merkle_proof_pda using the consensus_result from proposal
    let meta_merkle_proof_pda =
        api_helpers::generate_meta_merkle_proof_pda(&consensus_result_pda, &vote_account_pubkey)?;

    let validator_vote_pda = derive_vote_pda(&proposal_pubkey, &vote_account_pubkey, &program.id());
    let vote_override_pda = derive_vote_override_pda(
        &proposal_pubkey,
        &Pubkey::from_str(&stake_account_str)?,
        &validator_vote_pda,
        &program.id(),
    );
    let vote_override_cache_pda =
        derive_vote_override_cache_pda(&proposal_pubkey, &validator_vote_pda, &program.id());

    let stake_merkle_proof_vec =
        convert_merkle_proof_strings(&stake_merkle_proof.stake_merkle_proof)?;

    let stake_merkle_leaf =
        convert_stake_merkle_leaf_data_to_idl_type(&stake_merkle_proof.stake_merkle_leaf)?;

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

    // A close_timestamp override only takes effect when the account is created below; warn if
    // the user passed one but the account already exists so the value is not silently dropped.
    if meta_merkle_proof_account.is_some() && close_timestamp_override.is_some() {
        log::warn!(
            "--close-timestamp was provided, but the MetaMerkleProof account already exists. \
             close_timestamp is only set when the account is created, so the provided value will be ignored."
        );
    }

    // Preflight: initialize the meta merkle proof account if it does not yet exist. This
    // instruction accepts any payer, so it is sent by the proposer rather than the vault.
    let mut preflight_ixs = Vec::new();
    if meta_merkle_proof_account.is_none() {
        info!("Creating meta merkle proof account");

        let voting_wallet = Pubkey::from_str(&meta_merkle_proof.meta_merkle_leaf.voting_wallet)
            .map_err(|e| anyhow!("Invalid voting wallet in proof: {}", e))?;

        let close_timestamp = match close_timestamp_override {
            Some(ts) => ts,
            None => compute_vote_expiry_timestamp(&program, proposal.end_epoch).await?,
        };
        info!("Setting MetaMerkleProof close_timestamp to {}", close_timestamp);

        let init_meta_merkle_proof_ix = merkle_proof_program
            .request()
            .args(ncn_snapshot::instruction::InitMetaMerkleProof {
                close_timestamp,
                meta_merkle_leaf: MetaMerkleLeaf {
                    voting_wallet,
                    vote_account: vote_account_pubkey,
                    stake_merkle_root: Pubkey::from_str_const(
                        meta_merkle_proof
                            .meta_merkle_leaf
                            .stake_merkle_root
                            .as_str(),
                    )
                    .to_bytes(),
                    active_stake: meta_merkle_proof.meta_merkle_leaf.active_stake,
                },
                meta_merkle_proof: meta_merkle_proof
                    .meta_merkle_proof
                    .iter()
                    .map(|s| Pubkey::from_str_const(s).to_bytes())
                    .collect(),
            })
            .accounts(ncn_snapshot::accounts::InitMetaMerkleProof {
                consensus_result: consensus_result_pda,
                merkle_proof: meta_merkle_proof_pda,
                payer: payer.pubkey(),
                system_program: system_program::ID,
            })
            .instructions()?;

        preflight_ixs.extend(init_meta_merkle_proof_ix);
    }

    let spinner = create_spinner("Sending vote override transaction...");

    let signer = effective_signer(squads.as_ref(), payer.pubkey());
    let cast_vote_override_ixs = program
        .request()
        .args(args::CastVoteOverride {
            for_votes_bp: for_votes,
            against_votes_bp: against_votes,
            abstain_votes_bp: abstain_votes,
            stake_merkle_proof: stake_merkle_proof_vec,
            stake_merkle_leaf,
        })
        .accounts(accounts::CastVoteOverride {
            signer,
            spl_vote_account: vote_account_pubkey,
            spl_stake_account: Pubkey::from_str(&stake_account_str)?,
            proposal: proposal_pubkey,
            validator_vote: validator_vote_pda,
            vote_override: vote_override_pda,
            vote_override_cache: vote_override_cache_pda,
            consensus_result: consensus_result_pda,
            meta_merkle_proof: meta_merkle_proof_pda,
            snapshot_program: SNAPSHOT_PROGRAM_ID,
            system_program: system_program::ID,
        })
        .instructions()?;

    let rpc = program.rpc();
    let squads_config = squads.as_ref().map(|opts| opts.to_config(payer.pubkey()));
    let outcome = crate::utils::squads::route(
        &rpc,
        cast_vote_override_ixs,
        preflight_ixs,
        &[payer.as_ref()],
        squads_config.as_ref(),
    )
    .await?;

    spinner.finish_and_clear();
    println!("{}", outcome.format_structured());

    Ok(())
}
