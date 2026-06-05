use std::{str::FromStr, sync::Arc};

use anchor_client::{
    Program,
    solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction},
};
use anchor_lang::{prelude::Pubkey, system_program};
use anyhow::{Result, anyhow};

use crate::{
    svmgov_program::client::{accounts, args},
    utils::{
        squads::{SquadsCliOpts, effective_signer},
        utils::{
            anchor_client_setup, create_spinner, derive_global_config_pda, derive_program_data_pda,
            fetch_global_config, setup_admin,
        },
    },
};

// Upper bounds enforced on-chain. Mirrored here so the CLI fails fast with a clear
// message instead of paying for a transaction the program will reject.
// Keep in sync with the program's MAX_TITLE_ACCOUNT_SIZE / MAX_DESC_ACCOUNT_SIZE / BASIS_POINTS_MAX.
const MAX_TITLE_LENGTH: u16 = 200;
const MAX_DESCRIPTION_LENGTH: u16 = 500;
const BASIS_POINTS_MAX: u64 = 10_000;

/// Validates the bounded config fields against the same limits the program enforces.
/// Only the provided (`Some`) fields are checked, so this works for both the full set
/// supplied at initialization and the partial set supplied on update.
fn validate_config_values(
    max_title_length: Option<u16>,
    max_description_length: Option<u16>,
    cluster_support_pct_min_bps: Option<u64>,
) -> Result<()> {
    if let Some(v) = max_title_length {
        if v == 0 || v > MAX_TITLE_LENGTH {
            return Err(anyhow!(
                "max_title_length must be between 1 and {} bytes",
                MAX_TITLE_LENGTH
            ));
        }
    }
    if let Some(v) = max_description_length {
        if v == 0 || v > MAX_DESCRIPTION_LENGTH {
            return Err(anyhow!(
                "max_description_length must be between 1 and {} bytes",
                MAX_DESCRIPTION_LENGTH
            ));
        }
    }
    if let Some(v) = cluster_support_pct_min_bps {
        if v > BASIS_POINTS_MAX {
            return Err(anyhow!(
                "cluster_support_pct_min_bps must be between 0 and {} basis points",
                BASIS_POINTS_MAX
            ));
        }
    }
    Ok(())
}

/// Best-effort check that `signer` is the program's upgrade authority before sending the
/// init transaction. `initialize_config` is gated on-chain to the upgrade authority, so
/// this catches the most common mistake (running init with the wrong key) early. It only
/// errors when a mismatch can be positively determined; otherwise it defers to the
/// on-chain constraint.
async fn ensure_upgrade_authority(
    program: &Program<Arc<Keypair>>,
    program_data: &Pubkey,
    signer: &Pubkey,
) -> Result<()> {
    let data = match program.rpc().get_account_data(program_data).await {
        Ok(d) => d,
        // Can't read the ProgramData account (e.g. RPC issue) — let the program enforce it.
        Err(_) => return Ok(()),
    };

    // UpgradeableLoaderState::ProgramData layout (bincode):
    //   [0..4]   enum discriminant (3 = ProgramData)
    //   [4..12]  slot (u64)
    //   [12]     Option tag for upgrade_authority_address (0 = None, 1 = Some)
    //   [13..45] upgrade authority pubkey (present only when the tag is 1)
    if data.len() >= 45 && data[12] == 1 {
        let authority =
            Pubkey::new_from_array(data[13..45].try_into().expect("slice is exactly 32 bytes"));
        if &authority != signer {
            return Err(anyhow!(
                "Signer {} is not the program's upgrade authority ({}).\n\
                 `init-global-config` must be signed by the program upgrade authority.",
                signer,
                authority
            ));
        }
    }
    Ok(())
}

pub async fn initialize_global_config(
    keypair: Option<String>,
    rpc_url: Option<String>,
    max_title_length: u16,
    max_description_length: u16,
    max_support_epochs: u64,
    min_proposal_stake_lamports: u64,
    cluster_support_pct_min_bps: u64,
    discussion_epochs: u64,
    voting_epochs: u64,
    snapshot_epoch_extension: u64,
    snapshot_slot_offset: i64,
    squads: Option<SquadsCliOpts>,
) -> Result<()> {
    validate_config_values(
        Some(max_title_length),
        Some(max_description_length),
        Some(cluster_support_pct_min_bps),
    )?;

    let (payer, program) = setup_admin(keypair, rpc_url)?;

    let program_id = program.id();
    let global_config_pda = derive_global_config_pda(&program_id);
    let program_data = derive_program_data_pda(&program_id);
    let admin = effective_signer(squads.as_ref(), payer.pubkey());

    ensure_upgrade_authority(&program, &program_data, &payer.pubkey()).await?;

    let spinner = create_spinner("Initializing global config...");

    let ixs = program
        .request()
        .args(args::InitializeConfig {
            max_title_length,
            max_description_length,
            max_support_epochs,
            min_proposal_stake_lamports,
            cluster_support_pct_min_bps,
            discussion_epochs,
            voting_epochs,
            snapshot_epoch_extension,
            snapshot_slot_offset,
        })
        .accounts(accounts::InitializeConfig {
            admin,
            global_config: global_config_pda,
            system_program: system_program::ID,
            program: program_id,
            program_data,
        })
        .instructions()?;

    let rpc = program.rpc();
    let squads_config = squads.as_ref().map(|opts| opts.to_config(payer.pubkey()));
    let outcome = crate::utils::squads::route(
        &rpc,
        ixs,
        Vec::new(),
        &[payer.as_ref()],
        squads_config.as_ref(),
    )
    .await?;

    spinner.finish_and_clear();
    println!("{}", outcome.format_structured());

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn update_global_config(
    keypair: Option<String>,
    rpc_url: Option<String>,
    max_title_length: Option<u16>,
    max_description_length: Option<u16>,
    max_support_epochs: Option<u64>,
    min_proposal_stake_lamports: Option<u64>,
    cluster_support_pct_min_bps: Option<u64>,
    discussion_epochs: Option<u64>,
    voting_epochs: Option<u64>,
    snapshot_epoch_extension: Option<u64>,
    snapshot_slot_offset: Option<i64>,
    squads: Option<SquadsCliOpts>,
) -> Result<()> {
    validate_config_values(
        max_title_length,
        max_description_length,
        cluster_support_pct_min_bps,
    )?;

    let (payer, program) = setup_admin(keypair, rpc_url)?;

    let global_config_pda = derive_global_config_pda(&program.id());
    let admin = effective_signer(squads.as_ref(), payer.pubkey());

    let spinner = create_spinner("Updating global config...");

    let ixs = program
        .request()
        .args(args::UpdateConfig {
            max_title_length,
            max_description_length,
            max_support_epochs,
            min_proposal_stake_lamports,
            cluster_support_pct_min_bps,
            discussion_epochs,
            voting_epochs,
            snapshot_epoch_extension,
            snapshot_slot_offset,
        })
        .accounts(accounts::UpdateConfig {
            admin,
            global_config: global_config_pda,
        })
        .instructions()?;

    let rpc = program.rpc();
    let squads_config = squads.as_ref().map(|opts| opts.to_config(payer.pubkey()));
    let outcome = crate::utils::squads::route(
        &rpc,
        ixs,
        Vec::new(),
        &[payer.as_ref()],
        squads_config.as_ref(),
    )
    .await?;

    spinner.finish_and_clear();
    println!("{}", outcome.format_structured());

    Ok(())
}

/// Step 1 of the two-step admin transfer. The current admin nominates `new_admin`; the
/// nominee must then run `accept-admin` to complete the transfer. Signed by the current admin.
pub async fn nominate_admin(
    keypair: Option<String>,
    new_admin: String,
    rpc_url: Option<String>,
) -> Result<()> {
    let proposed_admin = Pubkey::from_str(&new_admin)
        .map_err(|e| anyhow!("Invalid new admin pubkey '{}': {}", new_admin, e))?;

    let (payer, program) = setup_admin(keypair, rpc_url)?;

    let global_config_pda = derive_global_config_pda(&program.id());

    let spinner = create_spinner("Nominating new admin...");

    let ixs = program
        .request()
        .args(args::NominateAdmin { proposed_admin })
        .accounts(accounts::NominateAdmin {
            admin: payer.pubkey(),
            global_config: global_config_pda,
        })
        .instructions()?;

    let blockhash = program.rpc().get_latest_blockhash().await?;
    let transaction =
        Transaction::new_signed_with_payer(&ixs, Some(&payer.pubkey()), &[&payer], blockhash);

    let sig = program
        .rpc()
        .send_and_confirm_transaction(&transaction)
        .await?;

    spinner.finish_with_message(format!(
        "Nominated {} as admin. They must run `accept-admin` to complete the transfer. https://explorer.solana.com/tx/{}",
        proposed_admin, sig
    ));

    Ok(())
}

/// Step 2 of the two-step admin transfer. The nominated admin accepts the role and
/// becomes the active admin. Signed by the nominee (the pending admin).
pub async fn accept_admin(keypair: Option<String>, rpc_url: Option<String>) -> Result<()> {
    let (payer, program) = setup_admin(keypair, rpc_url)?;

    let global_config_pda = derive_global_config_pda(&program.id());

    let spinner = create_spinner("Accepting admin role...");

    let ixs = program
        .request()
        .args(args::AcceptAdmin {})
        .accounts(accounts::AcceptAdmin {
            new_admin: payer.pubkey(),
            global_config: global_config_pda,
        })
        .instructions()?;

    let blockhash = program.rpc().get_latest_blockhash().await?;
    let transaction =
        Transaction::new_signed_with_payer(&ixs, Some(&payer.pubkey()), &[&payer], blockhash);

    let sig = program
        .rpc()
        .send_and_confirm_transaction(&transaction)
        .await?;

    spinner.finish_with_message(format!(
        "Admin role accepted. {} is now the config admin. https://explorer.solana.com/tx/{}",
        payer.pubkey(),
        sig
    ));

    Ok(())
}

pub async fn show_global_config(rpc_url: Option<String>) -> Result<()> {
    let mock_payer = Arc::new(Keypair::new());
    let program = anchor_client_setup(rpc_url, mock_payer)?;

    let config = fetch_global_config(&program).await?;

    println!("\nOn-chain Global Config:");
    println!("  admin:                       {}", config.admin);
    match config.pending_admin {
        Some(pending) => println!("  pending_admin:               {}", pending),
        None => println!("  pending_admin:               none"),
    }
    println!("  max_title_length:            {}", config.max_title_length);
    println!(
        "  max_description_length:      {}",
        config.max_description_length
    );
    println!(
        "  max_support_epochs:          {}",
        config.max_support_epochs
    );
    println!(
        "  min_proposal_stake_lamports: {}",
        config.min_proposal_stake_lamports
    );
    println!(
        "  cluster_support_pct_min_bps: {}",
        config.cluster_support_pct_min_bps
    );
    println!(
        "  discussion_epochs:           {}",
        config.discussion_epochs
    );
    println!("  voting_epochs:               {}", config.voting_epochs);
    println!(
        "  snapshot_epoch_extension:    {}",
        config.snapshot_epoch_extension
    );
    println!(
        "  snapshot_slot_offset:        {}",
        config.snapshot_slot_offset
    );

    Ok(())
}
