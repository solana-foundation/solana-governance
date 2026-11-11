use std::{str::FromStr, sync::Arc};

use anchor_client::{
    DynSigner,
    solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer},
};
use anchor_lang::system_program;
use anyhow::{Result, anyhow};

use crate::{
    svmgov_program::client::{accounts, args},
    utils::{
        squads::{SquadsCliOpts, effective_signer, route},
        utils::{
            anchor_client_setup, create_spinner, derive_vote_delegation_pda, fetch_vote_delegation,
            resolve_vote_account, setup_signer_and_program, vote_account_authorities,
        },
    },
};

/// Resolves what a delegation command acts on: `subject` verbatim when given,
/// the signer's own wallet when `wallet` is set (a staker delegating their
/// override vote), otherwise an SPL vote account (see `resolve_vote_account`).
async fn resolve_subject(
    program: &anchor_client::Program<Arc<DynSigner>>,
    signer: &Pubkey,
    subject: Option<String>,
    vote_account: Option<String>,
    wallet: bool,
) -> Result<Pubkey> {
    if let Some(subject) = subject {
        if wallet || vote_account.is_some() {
            return Err(anyhow!(
                "--subject cannot be combined with --wallet or --vote-account"
            ));
        }
        return Pubkey::from_str(&subject).map_err(|_| anyhow!("Invalid subject: {subject}"));
    }
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
///
/// With `squads` the vault PDA is the signer: it must hold one of those roles
/// and pays the delegation's rent when the vault transaction executes.
pub async fn delegate_vote(
    delegate: String,
    vote_account: Option<String>,
    wallet: bool,
    identity_keypair: Option<String>,
    rpc_url: Option<String>,
    squads: Option<SquadsCliOpts>,
) -> Result<()> {
    let delegate =
        Pubkey::from_str(&delegate).map_err(|_| anyhow!("Invalid delegate pubkey: {delegate}"))?;
    let (payer, program, _client) = setup_signer_and_program(identity_keypair, rpc_url)?;
    let signer = effective_signer(squads.as_ref(), payer.pubkey());
    let subject = resolve_subject(&program, &signer, None, vote_account, wallet).await?;

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

    // Mirror the program's signer rule so a wrong key fails here rather than in
    // a transaction (or, under Squads, in a vault transaction that would be
    // left behind as a dead proposal).
    match vote_account_authorities(&program, &subject).await? {
        Some(authorities) => {
            if signer != authorities.node_pubkey && signer != authorities.authorized_withdrawer {
                return Err(anyhow!(
                    "Only the node identity ({}) or authorized withdrawer ({}) of vote account \
                     {subject} can delegate its vote; signer is {signer}",
                    authorities.node_pubkey,
                    authorities.authorized_withdrawer
                ));
            }
        }
        None => {
            if signer != subject {
                return Err(anyhow!(
                    "{subject} is not a live vote account, so only that wallet itself can \
                     delegate its vote; signer is {signer}"
                ));
            }
        }
    }

    let spinner = create_spinner("Delegating vote...");
    let ixs = program
        .request()
        .args(args::DelegateVote { delegate })
        .accounts(accounts::DelegateVote {
            signer,
            subject,
            vote_delegation: derive_vote_delegation_pda(&subject, &program.id()),
            system_program: system_program::ID,
        })
        .instructions()?;

    let rpc = program.rpc();
    let squads_config = squads.as_ref().map(|opts| opts.to_config(payer.pubkey()));
    let outcome = route(
        &rpc,
        ixs,
        Vec::new(),
        &[payer.as_ref()],
        squads_config.as_ref(),
    )
    .await?;
    spinner.finish_and_clear();
    println!("{}", outcome.format_structured());

    println!(
        "Delegating the vote of {subject} to {delegate}. The delegate must then run \
         `svmgov -k <delegate-keypair> accept-vote-delegation --subject {subject}`."
    );
    Ok(())
}

/// Accepts a pending vote delegation (step 2 of 2). Must be signed by the delegate
/// named in `delegate-vote`; afterwards the delegate alone votes for the subject
/// (an SPL vote account or a staker's wallet). With `squads` the vault PDA is
/// the delegate and the acceptance lands when the vault transaction executes.
pub async fn accept_vote_delegation(
    subject: String,
    identity_keypair: Option<String>,
    rpc_url: Option<String>,
    squads: Option<SquadsCliOpts>,
) -> Result<()> {
    let subject =
        Pubkey::from_str(&subject).map_err(|_| anyhow!("Invalid subject pubkey: {subject}"))?;
    let (payer, program, _client) = setup_signer_and_program(identity_keypair, rpc_url)?;
    let signer = effective_signer(squads.as_ref(), payer.pubkey());

    let delegation = fetch_vote_delegation(&program, &subject)
        .await?
        .ok_or_else(|| anyhow!("{subject} has no vote delegation"))?;
    if delegation.delegate != signer {
        return Err(anyhow!(
            "{subject} delegates to {}; signer is {signer}",
            delegation.delegate
        ));
    }
    if delegation.delegation_accepted {
        return Err(anyhow!("Vote delegation of {subject} is already accepted"));
    }

    let spinner = create_spinner("Accepting vote delegation...");
    let ixs = program
        .request()
        .args(args::AcceptVoteDelegation {})
        .accounts(accounts::AcceptVoteDelegation {
            signer,
            vote_delegation: derive_vote_delegation_pda(&subject, &program.id()),
        })
        .instructions()?;

    let rpc = program.rpc();
    let squads_config = squads.as_ref().map(|opts| opts.to_config(payer.pubkey()));
    let outcome = route(
        &rpc,
        ixs,
        Vec::new(),
        &[payer.as_ref()],
        squads_config.as_ref(),
    )
    .await?;
    spinner.finish_and_clear();
    println!("{}", outcome.format_structured());

    println!(
        "Accepting the vote delegation of {subject}: {signer} votes for it and the subject's \
         own key is locked out until the delegation is removed."
    );
    Ok(())
}

/// Removes a vote delegation and returns its rent to the subject's root
/// authority: a vote account's authorized withdrawer, or the wallet itself.
///
/// The delegate may always remove. For a vote account the authorized withdrawer
/// may always remove and the node identity only while the delegation is pending;
/// for a wallet (`--wallet`) the wallet may always remove. `subject` names any
/// vote account or wallet explicitly, which a delegate resigning from a staker's
/// delegation needs. Without `subject` or `--wallet`, `vote_account` defaults to
/// the signer's own vote account or the one delegated to it. With `squads` the
/// vault PDA is the signer and must hold one of those roles.
pub async fn remove_vote_delegation(
    subject: Option<String>,
    vote_account: Option<String>,
    wallet: bool,
    identity_keypair: Option<String>,
    rpc_url: Option<String>,
    squads: Option<SquadsCliOpts>,
) -> Result<()> {
    let (payer, program, _client) = setup_signer_and_program(identity_keypair, rpc_url)?;
    let signer = effective_signer(squads.as_ref(), payer.pubkey());
    let subject = resolve_subject(&program, &signer, subject, vote_account, wallet).await?;

    let delegation = fetch_vote_delegation(&program, &subject)
        .await?
        .ok_or_else(|| anyhow!("{subject} has no vote delegation"))?;

    // Mirror the program's removal rules (and its choice of rent recipient) so a
    // wrong key fails here rather than in a transaction the program rejects.
    let (authorized, rent_recipient) = match vote_account_authorities(&program, &subject).await? {
        Some(authorities) => (
            signer == delegation.delegate
                || signer == authorities.authorized_withdrawer
                || (!delegation.delegation_accepted && signer == authorities.node_pubkey),
            authorities.authorized_withdrawer,
        ),
        None => (signer == delegation.delegate || signer == subject, subject),
    };
    if !authorized {
        return Err(anyhow!(
            "{signer} may not remove the vote delegation of {subject}: only the delegate ({}), \
             the subject's root authority, or (while pending) its node identity can",
            delegation.delegate
        ));
    }

    let spinner = create_spinner("Removing vote delegation...");
    let ixs = program
        .request()
        .args(args::RemoveVoteDelegation {})
        .accounts(accounts::RemoveVoteDelegation {
            signer,
            subject,
            vote_delegation: derive_vote_delegation_pda(&subject, &program.id()),
            rent_recipient,
        })
        .instructions()?;

    let rpc = program.rpc();
    let squads_config = squads.as_ref().map(|opts| opts.to_config(payer.pubkey()));
    let outcome = route(
        &rpc,
        ixs,
        Vec::new(),
        &[payer.as_ref()],
        squads_config.as_ref(),
    )
    .await?;
    spinner.finish_and_clear();
    println!("{}", outcome.format_structured());

    println!(
        "Removing the vote delegation of {subject} (delegate {}); its rent returns to \
         {rent_recipient}.",
        delegation.delegate
    );
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
