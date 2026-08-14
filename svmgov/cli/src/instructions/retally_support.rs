use std::str::FromStr;

use anchor_client::solana_sdk::{
    compute_budget::ComputeBudgetInstruction, pubkey::Pubkey, signer::Signer,
    transaction::Transaction,
};
use anchor_lang::system_program;
use anyhow::{Result, anyhow};
use ncn_snapshot::ID as SNAPSHOT_PROGRAM_ID;

use crate::{
    constants::{support_compute_unit_limit, MAX_SUPPORTERS_LIMIT},
    svmgov_program::accounts::Proposal,
    svmgov_program::client::{accounts, args},
    utils::utils::{
        create_spinner, derive_global_config_pda, derive_program_config_pda, fetch_global_config,
        get_epoch_slot_range, setup_signer_and_program,
    },
};


pub async fn retally_support(
    proposal_id: String,
    identity_keypair: Option<String>,
    rpc_url: Option<String>,
    _network: String,
) -> Result<()> {
    let proposal_pubkey = Pubkey::from_str(&proposal_id)
        .map_err(|_| anyhow!("Invalid proposal ID: {}", proposal_id))?;

    let (payer, program, _client) = setup_signer_and_program(identity_keypair, rpc_url)?;

    let global_config = fetch_global_config(&program).await?;

    // The handler re-tallies every existing supporter, so the compute budget is
    // sized from the current list length rather than a flat worst case. If the
    // read fails the instruction itself may still succeed, so fall back to the
    // program's supporter cap rather than aborting on a budgeting detail.
    let num_supporters = match program.account::<Proposal>(proposal_pubkey).await {
        Ok(proposal) => proposal.num_supporters,
        Err(e) => {
            log::warn!(
                "Could not read the supporter count for {proposal_pubkey}: {e}. Requesting the \
                 compute budget for a full {MAX_SUPPORTERS_LIMIT}-supporter list."
            );
            MAX_SUPPORTERS_LIMIT
        }
    };
    let compute_unit_limit = support_compute_unit_limit(num_supporters);

    let spinner = create_spinner("Re-tallying proposal support...");

    // Same prospective snapshot slot as support_proposal: if this retally
    // crosses the threshold, the program binds the ballot box derived from it.
    let clock = program.rpc().get_epoch_info().await?;
    let target_epoch =
        clock.epoch + global_config.discussion_epochs + global_config.snapshot_epoch_extension;

    let (start_slot, _) = get_epoch_slot_range(target_epoch);
    let snapshot_slot = ((start_slot as i64) + global_config.snapshot_slot_offset) as u64;

    let ballot_box_pda = {
        let seeds = &[b"BallotBox".as_ref(), &snapshot_slot.to_le_bytes()];
        let (pda, _) = Pubkey::find_program_address(seeds, &SNAPSHOT_PROGRAM_ID);
        pda
    };

    let program_config_pda = derive_program_config_pda(&SNAPSHOT_PROGRAM_ID);
    let global_config_pda = derive_global_config_pda(&program.id());

    let retally_support_ixs = program
        .request()
        .instruction(ComputeBudgetInstruction::set_compute_unit_limit(
            compute_unit_limit,
        ))
        .args(args::RetallySupport {})
        .accounts(accounts::RetallySupport {
            signer: payer.pubkey(),
            proposal: proposal_pubkey,
            ballot_box: ballot_box_pda,
            ballot_program: SNAPSHOT_PROGRAM_ID,
            program_config: program_config_pda,
            global_config: global_config_pda,
            system_program: system_program::ID,
        })
        .instructions()?;

    let blockhash = program.rpc().get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &retally_support_ixs,
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );

    let sig = program
        .rpc()
        .send_and_confirm_transaction(&transaction)
        .await?;

    spinner.finish_with_message(format!(
        "Proposal support re-tallied. https://explorer.solana.com/tx/{}",
        sig
    ));

    Ok(())
}
