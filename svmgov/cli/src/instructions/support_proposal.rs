use std::str::FromStr;

use anchor_client::solana_sdk::{pubkey::Pubkey, signer::Signer, transaction::Transaction};
use anchor_lang::system_program;
use anyhow::{Result, anyhow};
use ncn_snapshot::ID as SNAPSHOT_PROGRAM_ID;

use crate::{
    svmgov_program::client::{accounts, args},
    utils::utils::{
        create_spinner, derive_global_config_pda, derive_program_config_pda, derive_support_pda,
        fetch_global_config, get_epoch_slot_range, setup_all,
    },
};

pub async fn support_proposal(
    proposal_id: String,
    identity_keypair: Option<String>,
    rpc_url: Option<String>,
    _network: String,
) -> Result<()> {
    let proposal_pubkey = Pubkey::from_str(&proposal_id)
        .map_err(|_| anyhow!("Invalid proposal ID: {}", proposal_id))?;

    let (payer, vote_account, program, _merkle_proof_program) =
        setup_all(identity_keypair, rpc_url).await?;

    let support_pda = derive_support_pda(&proposal_pubkey, &vote_account, &program.id());

    let global_config = fetch_global_config(&program).await?;

    let spinner = create_spinner("Supporting proposal...");

    let clock = program.rpc().get_epoch_info().await?;
    let target_epoch = clock.epoch + global_config.discussion_epochs + global_config.snapshot_epoch_extension;

    let (start_slot, _) = get_epoch_slot_range(target_epoch);
    let snapshot_slot = ((start_slot as i64) + global_config.snapshot_slot_offset) as u64;

    let ballot_box_pda = {
        let seeds = &[b"BallotBox".as_ref(), &snapshot_slot.to_le_bytes()];
        let (pda, _) = Pubkey::find_program_address(seeds, &SNAPSHOT_PROGRAM_ID);
        pda
    };

    let program_config_pda = derive_program_config_pda(&SNAPSHOT_PROGRAM_ID);
    let global_config_pda = derive_global_config_pda(&program.id());

    let support_proposal_ixs = program
        .request()
        .args(args::SupportProposal {})
        .accounts(accounts::SupportProposal {
            signer: payer.pubkey(),
            proposal: proposal_pubkey,
            support: support_pda,
            spl_vote_account: vote_account,
            ballot_box: ballot_box_pda,
            program_config: program_config_pda,
            ballot_program: SNAPSHOT_PROGRAM_ID,
            global_config: global_config_pda,
            system_program: system_program::ID,
        })
        .instructions()?;

    let blockhash = program.rpc().get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &support_proposal_ixs,
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );

    let sig = program
        .rpc()
        .send_and_confirm_transaction(&transaction)
        .await?;

    spinner.finish_with_message(format!(
        "Proposal supported. https://explorer.solana.com/tx/{}",
        sig
    ));

    Ok(())
}
