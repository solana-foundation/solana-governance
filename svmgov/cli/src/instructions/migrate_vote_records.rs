use std::sync::Arc;

use anchor_client::{
    DynSigner,
    solana_account_decoder::{UiAccountEncoding, UiDataSliceConfig},
    solana_client::{
        rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
        rpc_filter::{Memcmp, RpcFilterType},
    },
    solana_sdk::{
        commitment_config::CommitmentConfig, compute_budget::ComputeBudgetInstruction,
        instruction::AccountMeta, pubkey::Pubkey, signer::Signer, transaction::Transaction,
    },
};
use anchor_lang::{Discriminator, system_program};
use anyhow::{Result, anyhow};

use crate::{
    constants::{
        LEGACY_VOTE_ACCOUNT_LEN, LEGACY_VOTE_OVERRIDE_ACCOUNT_LEN, MIGRATE_RECORDS_CU_LIMIT,
        MIGRATE_RECORDS_PER_TX,
    },
    svmgov_program::{
        accounts::{Vote, VoteOverride},
        client::{accounts, args},
    },
    utils::utils::{create_spinner, setup_signer_and_program},
};

/// Lists the program accounts carrying `discriminator` at exactly `len` bytes,
/// i.e. the records written before `delegate_signer` was appended.
async fn find_legacy_records(
    program: &anchor_client::Program<Arc<DynSigner>>,
    discriminator: &[u8],
    len: u64,
) -> Result<Vec<Pubkey>> {
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![
            RpcFilterType::DataSize(len),
            RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, discriminator.to_vec())),
        ]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            // Only the addresses are needed; skip downloading the record bytes.
            data_slice: Some(UiDataSliceConfig {
                offset: 0,
                length: 0,
            }),
            commitment: Some(CommitmentConfig::confirmed()),
            ..Default::default()
        },
        ..Default::default()
    };
    let accounts = program
        .rpc()
        .get_program_accounts_with_config(&program.id(), config)
        .await
        .map_err(|e| anyhow!("Failed to list legacy vote records: {e}"))?;
    Ok(accounts.into_iter().map(|(pubkey, _)| pubkey).collect())
}

/// Grows every `Vote` and `VoteOverride` recorded before `delegate_signer`
/// existed to the current layout, in batches, paying the added rent from the
/// signer. With `dry_run` it only reports how many records need migrating.
/// Safe to re-run: the program skips records already at the current size.
pub async fn migrate_vote_records(
    dry_run: bool,
    identity_keypair: Option<String>,
    rpc_url: Option<String>,
) -> Result<()> {
    let (payer, program, _client) = setup_signer_and_program(identity_keypair, rpc_url)?;

    let spinner = create_spinner("Scanning for legacy vote records...");
    let votes = find_legacy_records(&program, Vote::DISCRIMINATOR, LEGACY_VOTE_ACCOUNT_LEN).await?;
    let overrides = find_legacy_records(
        &program,
        VoteOverride::DISCRIMINATOR,
        LEGACY_VOTE_OVERRIDE_ACCOUNT_LEN,
    )
    .await?;
    spinner.finish_with_message(format!(
        "Found {} legacy Vote and {} legacy VoteOverride account(s).",
        votes.len(),
        overrides.len()
    ));

    let records: Vec<Pubkey> = votes.into_iter().chain(overrides).collect();
    if records.is_empty() {
        println!("Nothing to migrate.");
        return Ok(());
    }
    if dry_run {
        println!(
            "Dry run: {} record(s) would be migrated in {} transaction(s).",
            records.len(),
            records.len().div_ceil(MIGRATE_RECORDS_PER_TX)
        );
        return Ok(());
    }

    let batches = records.chunks(MIGRATE_RECORDS_PER_TX);
    let total = batches.len();
    for (i, batch) in batches.enumerate() {
        let spinner = create_spinner(&format!(
            "Migrating batch {}/{} ({} record(s))...",
            i + 1,
            total,
            batch.len()
        ));
        let remaining: Vec<AccountMeta> = batch
            .iter()
            .map(|record| AccountMeta::new(*record, false))
            .collect();
        let instructions = program
            .request()
            .instruction(ComputeBudgetInstruction::set_compute_unit_limit(
                MIGRATE_RECORDS_CU_LIMIT,
            ))
            .args(args::MigrateVoteRecords {})
            .accounts(accounts::MigrateVoteRecords {
                payer: payer.pubkey(),
                system_program: system_program::ID,
            })
            .accounts(remaining)
            .instructions()?;

        let blockhash = program.rpc().get_latest_blockhash().await?;
        let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
        transaction.try_sign(&[&payer], blockhash)?;
        let signature = program
            .rpc()
            .send_and_confirm_transaction(&transaction)
            .await?;
        spinner.finish_with_message(format!(
            "Batch {}/{} migrated. https://explorer.solana.com/tx/{signature}",
            i + 1,
            total
        ));
    }

    println!("Migrated {} record(s).", records.len());
    Ok(())
}
