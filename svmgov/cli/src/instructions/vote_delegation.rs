use std::{str::FromStr, sync::Arc};

use anchor_client::{
    DynSigner,
    solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer},
};
use anchor_lang::system_program;
use anyhow::{Result, anyhow};

use crate::{
    svmgov_program::client::{accounts, args},
    utils::utils::{
        anchor_client_setup, create_spinner, derive_vote_delegation_pda, fetch_vote_delegation,
        resolve_vote_account, setup_signer_and_program, vote_delegation_rent_recipient,
    },
};

/// Resolves what a delegation command acts on: the signer's own wallet when
/// `wallet` is set (a staker delegating their override vote), otherwise an SPL
/// vote account (see `resolve_vote_account`).
async fn resolve_subject(
    program: &anchor_client::Program<Arc<DynSigner>>,
    signer: &Pubkey,
    vote_account: Option<String>,
    wallet: bool,
) -> Result<Pubkey> {
    if wallet {
        if vote_account.is_some() {
            return Err(anyhow!(
                "--wallet and --vote-account are mutually exclusive"
            ));
        }
        return Ok(*signer);
    }
    resolve_vote_account(program, signer, vote_account).await
}

/// Names `delegate` as the governance voter for a subject (step 1 of 2).
///
/// With `--wallet` the subject is the signer's own wallet (a staker delegating
/// the override vote for their stake). Otherwise the subject is an SPL vote
/// account: the signer must be its node identity or authorized withdrawer, and
/// `vote_account` defaults to the vote account whose node identity is the
/// signer, so a withdrawer must pass it explicitly. The delegation takes effect
/// only once the delegate runs `accept-vote-delegation`.
pub async fn delegate_vote(
    delegate: String,
    vote_account: Option<String>,
    wallet: bool,
    identity_keypair: Option<String>,
    rpc_url: Option<String>,
) -> Result<()> {
    let delegate =
        Pubkey::from_str(&delegate).map_err(|_| anyhow!("Invalid delegate pubkey: {delegate}"))?;
    let (payer, program, _client) = setup_signer_and_program(identity_keypair, rpc_url)?;
    let subject = resolve_subject(&program, &payer.pubkey(), vote_account, wallet).await?;

    if let Some(existing) = fetch_vote_delegation(&program, &subject).await? {
        return Err(anyhow!(
            "{subject} already delegates to {} ({}). Remove it first with \
             `svmgov remove-vote-delegation{}`.",
            existing.delegate,
            if existing.delegation_accepted {
                "accepted"
            } else {
                "pending acceptance"
            },
            if wallet {
                " --wallet".to_string()
            } else {
                format!(" --vote-account {subject}")
            }
        ));
    }

    let spinner = create_spinner("Delegating vote...");
    let signature = program
        .request()
        .args(args::DelegateVote { delegate })
        .accounts(accounts::DelegateVote {
            signer: payer.pubkey(),
            subject,
            vote_delegation: derive_vote_delegation_pda(&subject, &program.id()),
            system_program: system_program::ID,
        })
        .send()
        .await?;

    spinner.finish_with_message(format!(
        "Vote of {subject} delegated to {delegate}, pending acceptance. The delegate must run \
         `svmgov -k <delegate-keypair> accept-vote-delegation --subject {subject}`. \
         https://explorer.solana.com/tx/{signature}"
    ));
    Ok(())
}

/// Accepts a pending vote delegation (step 2 of 2). Must be signed by the delegate
/// named in `delegate-vote`; afterwards the delegate alone votes for the subject
/// (an SPL vote account or a staker's wallet).
pub async fn accept_vote_delegation(
    subject: String,
    identity_keypair: Option<String>,
    rpc_url: Option<String>,
) -> Result<()> {
    let subject =
        Pubkey::from_str(&subject).map_err(|_| anyhow!("Invalid subject pubkey: {subject}"))?;
    let (payer, program, _client) = setup_signer_and_program(identity_keypair, rpc_url)?;

    let delegation = fetch_vote_delegation(&program, &subject)
        .await?
        .ok_or_else(|| anyhow!("{subject} has no vote delegation"))?;
    if delegation.delegate != payer.pubkey() {
        return Err(anyhow!(
            "{subject} delegates to {}; signer is {}",
            delegation.delegate,
            payer.pubkey()
        ));
    }
    if delegation.delegation_accepted {
        return Err(anyhow!("Vote delegation of {subject} is already accepted"));
    }

    let spinner = create_spinner("Accepting vote delegation...");
    let signature = program
        .request()
        .args(args::AcceptVoteDelegation {})
        .accounts(accounts::AcceptVoteDelegation {
            signer: payer.pubkey(),
            vote_delegation: derive_vote_delegation_pda(&subject, &program.id()),
        })
        .send()
        .await?;

    spinner.finish_with_message(format!(
        "Vote delegation of {subject} accepted; {} now votes for it. \
         https://explorer.solana.com/tx/{signature}",
        payer.pubkey()
    ));
    Ok(())
}

/// Removes a vote delegation and returns its rent to the subject's root
/// authority: a vote account's authorized withdrawer, or the wallet itself.
///
/// The delegate may always remove. For a vote account the authorized withdrawer
/// may always remove and the node identity only while the delegation is pending;
/// for a wallet (`--wallet`) the wallet may always remove. Without `--wallet`,
/// `vote_account` defaults to the signer's own vote account or the one
/// delegated to it.
pub async fn remove_vote_delegation(
    vote_account: Option<String>,
    wallet: bool,
    identity_keypair: Option<String>,
    rpc_url: Option<String>,
) -> Result<()> {
    let (payer, program, _client) = setup_signer_and_program(identity_keypair, rpc_url)?;
    let subject = resolve_subject(&program, &payer.pubkey(), vote_account, wallet).await?;

    let delegation = fetch_vote_delegation(&program, &subject)
        .await?
        .ok_or_else(|| anyhow!("{subject} has no vote delegation"))?;
    let rent_recipient = vote_delegation_rent_recipient(&program, &subject).await?;

    let spinner = create_spinner("Removing vote delegation...");
    let signature = program
        .request()
        .args(args::RemoveVoteDelegation {})
        .accounts(accounts::RemoveVoteDelegation {
            signer: payer.pubkey(),
            subject,
            vote_delegation: derive_vote_delegation_pda(&subject, &program.id()),
            rent_recipient,
        })
        .send()
        .await?;

    spinner.finish_with_message(format!(
        "Vote delegation of {subject} (delegate {}) removed. \
         https://explorer.solana.com/tx/{signature}",
        delegation.delegate
    ));
    Ok(())
}

/// Prints the vote delegation of a vote account, if any.
pub async fn show_vote_delegation(
    vote_account: String,
    rpc_url: Option<String>,
    json_output: bool,
) -> Result<()> {
    let vote_account = Pubkey::from_str(&vote_account)
        .map_err(|_| anyhow!("Invalid vote account: {vote_account}"))?;
    let mock_payer = Arc::new(DynSigner(Arc::new(Keypair::new())));
    let program = anchor_client_setup(rpc_url, mock_payer)?;

    let delegation = fetch_vote_delegation(&program, &vote_account).await?;

    if json_output {
        let output = match &delegation {
            Some(d) => serde_json::json!({
                "vote_account": vote_account.to_string(),
                "delegate": d.delegate.to_string(),
                "delegation_accepted": d.delegation_accepted,
            }),
            None => {
                serde_json::json!({ "vote_account": vote_account.to_string(), "delegate": null })
            }
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    match delegation {
        Some(d) => {
            println!("\nVote delegation for {vote_account}:");
            println!("  delegate:            {}", d.delegate);
            println!(
                "  status:              {}",
                if d.delegation_accepted {
                    "accepted (the delegate votes; the identity is locked out)"
                } else {
                    "pending acceptance (the identity still votes)"
                }
            );
        }
        None => println!("\nVote account {vote_account} has no vote delegation."),
    }
    Ok(())
}
