use super::{
    config::{self, Args, BodyFormat, Case, Kind, Target},
    http::{fetch, fetch_vote_accounts},
    report::Journal,
    Error, Result,
};
use reqwest::Client;
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;
use std::{cmp::Reverse, collections::HashSet, time::Instant};
use tokio::time::sleep;

const MAX_CANDIDATES: usize = 10;
const MAX_PROOF_PROBES: usize = 20;

/// Resolve latest vote proof and voter cases once, using accounts present in the selected snapshot.
pub async fn resolve(
    args: &Args,
    client: &Client,
    verifiers: &[Target],
    cases: &mut [Case],
    slot: u64,
    journal: &mut Journal,
) -> Result<()> {
    if cases.iter().any(|case| case.path == "latest") {
        let (vote_account, wallet) =
            discover_account(args, client, verifiers, slot, journal).await?;
        for case in cases.iter_mut().filter(|case| case.path == "latest") {
            let prefix = match case.name.as_str() {
                "vote_proof" => format!("/proof/vote_account/{vote_account}"),
                "voter" => format!("/voter/{wallet}"),
                _ => {
                    return Err(Error::Invalid(
                        "latest is supported only for vote_proof and voter".into(),
                    ))
                }
            };
            case.path = format!("{prefix}?network={}&slot={slot}", args.network.as_str());
        }
    }
    config::render(cases, args.network.as_str(), slot)
}

async fn discover_account(
    args: &Args,
    client: &Client,
    verifiers: &[Target],
    slot: u64,
    journal: &mut Journal,
) -> Result<(String, String)> {
    let interval = config::seconds(1.0 / args.rps)?;
    let started = Instant::now();
    let rpc = fetch_vote_accounts(
        client,
        args.rpc_url.as_deref().unwrap_or(args.network.rpc_url()),
    )
    .await?;
    let payload = if rpc.ok {
        rpc.body
            .as_ref()
            .map(|body| serde_json::from_slice::<Value>(body))
            .transpose()?
    } else {
        None
    };
    journal.consume(rpc)?;
    sleep(interval.saturating_sub(started.elapsed())).await;
    let payload = payload.ok_or_else(|| {
        Error::Invalid(
            "cannot discover latest accounts: getVoteAccounts failed; see RPC discovery evidence"
                .into(),
        )
    })?;
    let mut candidates: Vec<_> = ["current", "delinquent"]
        .into_iter()
        .flat_map(|key| payload["result"][key].as_array().into_iter().flatten())
        .filter_map(|account| {
            let address = account["votePubkey"].as_str()?;
            let stake = account["activatedStake"].as_u64()?;
            (stake > 0 && address.parse::<Pubkey>().is_ok()).then_some((stake, address))
        })
        .collect();
    candidates.sort_by_key(|(stake, address)| (Reverse(*stake), *address));
    let mut seen = HashSet::new();
    candidates.retain(|(_, address)| seen.insert(*address));
    if candidates.is_empty() {
        return Err(Error::Invalid(
            "getVoteAccounts returned no staked vote accounts for latest discovery".into(),
        ));
    }
    let mut attempts = 0;
    for (_, address) in candidates.into_iter().take(MAX_CANDIDATES) {
        let case = Case {
            name: "discover_vote_proof".into(),
            path: format!(
                "/proof/vote_account/{address}?network={}&slot={slot}",
                args.network.as_str()
            ),
            expected_status: 200,
            format: BodyFormat::Json,
            expected_body: None,
            targets: vec![Kind::Verifier],
        };
        for verifier in verifiers {
            if attempts >= MAX_PROOF_PROBES {
                break;
            }
            attempts += 1;
            let started = Instant::now();
            let mut row = fetch(client, verifier, &case, verifiers, "account_discovery").await;
            let mut resolved = None;
            if row.ok {
                let value: Value = serde_json::from_slice(row.body.as_ref().unwrap())?;
                let leaf = &value["meta_merkle_leaf"];
                let wallet = leaf["voting_wallet"]
                    .as_str()
                    .filter(|key| key.parse::<Pubkey>().is_ok());
                if leaf["vote_account"].as_str() == Some(address) && wallet.is_some() {
                    resolved = wallet.map(str::to_owned);
                } else {
                    row.issues.push("discovery proof must identify the requested vote account and a valid voting wallet".into());
                    row.ok = false;
                }
            }
            journal.consume(row)?;
            sleep(interval.saturating_sub(started.elapsed())).await;
            if let Some(wallet) = resolved {
                return Ok((address.to_owned(), wallet));
            }
        }
        if attempts >= MAX_PROOF_PROBES {
            break;
        }
    }
    Err(Error::Invalid(format!("no usable vote proof for snapshot {slot} after {attempts} discovery probes; set explicit request paths or choose another snapshot")))
}
