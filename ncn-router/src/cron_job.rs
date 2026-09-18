use chrono::Utc;
use job_scheduler::{Job, JobScheduler};
use reqwest::blocking::Client;
use serde::Deserialize;
use std::env;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use borsh::BorshDeserialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::fs;
use std::str::FromStr;

const DEFAULT_CONFIG: &str = "config.toml";
const DEFAULT_LOG: &str = "ncn_verifier_meta.log";
const NETWORKS: &[&str] = &["mainnet", "testnet"];
/// Cron: every 2 hours at :00. job_scheduler uses cron crate (6 fields: sec min hour day month dow).
const CRON_EVERY_2_HOURS: &str = "0 0 0,2,4,6,8,10,12,14,16,18,20,22 * * *";

// Anchor discriminator for the BallotBox account, from the IDL.
const BALLOT_BOX_DISCRIMINATOR: [u8; 8] = [155, 169, 156, 8, 92, 14, 24, 101];

/// How far a verifier's latest snapshot may trail the freshest whitelisted
/// verifier before it is dropped from routing. One epoch (432,000 slots, ~2
/// days) absorbs the normal spread in upload times while rejecting operators
/// that have stopped producing snapshots altogether. Override with
/// `NCN_MAX_VERIFIER_SLOT_LAG`.
const DEFAULT_MAX_VERIFIER_SLOT_LAG: u64 = 432_000;

#[derive(Debug, Deserialize)]
struct Config {
    verifiers: Vec<Verifier>,
}

#[derive(Debug, Deserialize)]
struct Verifier {
    name: String,
    verification_domain: String,
}

#[derive(Debug, Deserialize)]
struct MetaResponse {
    network: String,
    slot: u64,
    merkle_root: String,
    snapshot_hash: String,
    created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
struct LogEntry {
    timestamp: String,
    name: String,
    domain: String,
    network: String,
    slot: u64,
    merkle_root: String,
    snapshot_hash: String,
    created_at: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct WhitelistVerifier {
    name: String,
    domain: String,
    /// Slot of the snapshot this verifier last reported. Recorded so staleness
    /// can be judged against the rest of the fleet, and so an operator that has
    /// fallen behind is visible in the whitelist file without re-polling.
    slot: u64,
    status: String,
    reason: Option<String>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct WhitelistSnapshot {
    network: String,
    slot: u64,
    updated_at: String,
    verifiers: Vec<WhitelistVerifier>,
}

#[derive(Debug, BorshDeserialize)]
struct Ballot {
    meta_merkle_root: [u8; 32],
    snapshot_hash: [u8; 32],
}

pub fn run(network_filter: Option<String>) {
    if let Some(ref n) = network_filter {
        if n != "mainnet" && n != "testnet" {
            eprintln!("Invalid --network value: {} (expected mainnet|testnet)", n);
            std::process::exit(1);
        }
    }

    let config_path = env::var("NCN_CONFIG").unwrap_or_else(|_| DEFAULT_CONFIG.to_string());
    let log_path = env::var("NCN_LOG").unwrap_or_else(|_| DEFAULT_LOG.to_string());

    let filter_str = network_filter.as_deref();

    // Run once at startup so we don't wait 2 hours for first data
    if let Err(e) = run_meta_job(&config_path, &log_path, filter_str) {
        eprintln!("[ncn-meta-cron] First run failed: {}", e);
        std::process::exit(1);
    }
    if let Some(net) = filter_str {
        if let Err(e) = compare_with_chain(&log_path, net) {
            eprintln!("[ncn-meta-cron] First compare failed: {}", e);
        }
    } else {
        // If no filter is provided, compare and write whitelists for BOTH networks.
        // Uses the same NCN_PROGRAM_ID for both.
        for net in NETWORKS {
            if let Err(e) = compare_with_chain(&log_path, net) {
                eprintln!("[ncn-meta-cron] First compare failed (network={}): {}", net, e);
            }
        }
    }
    eprintln!("[ncn-meta-cron] First run done. Scheduling every 2 hours.");

    let schedule = match CRON_EVERY_2_HOURS.parse() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[ncn-meta-cron] Invalid cron expression {:?}: {}", CRON_EVERY_2_HOURS, e);
            std::process::exit(1);
        }
    };
    let mut sched = JobScheduler::new();
    let config_path_cl = config_path.clone();
    let log_path_cl = log_path.clone();
    let network_filter_cl = network_filter.clone();
    sched.add(Job::new(schedule, move || {
        let filter = network_filter_cl.as_deref();
        if let Err(e) = run_meta_job(&config_path_cl, &log_path_cl, filter) {
            eprintln!("[ncn-meta-cron] Job run failed: {}", e);
        } else {
            if let Some(net) = filter {
                if let Err(e) = compare_with_chain(&log_path_cl, net) {
                    eprintln!("[ncn-meta-cron] Compare job failed: {}", e);
                }
            } else {
                for net in NETWORKS {
                    if let Err(e) = compare_with_chain(&log_path_cl, net) {
                        eprintln!("[ncn-meta-cron] Compare job failed (network={}): {}", net, e);
                    }
                }
            }
        }
    }));

    loop {
        sched.tick();
        std::thread::sleep(Duration::from_secs(1));
    }
}

/// Fetch meta for all verifiers and overwrite the log file with the latest results.
fn run_meta_job(
    config_path: &str,
    log_path: &str,
    network_filter: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config(config_path)?;

    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?;

    let file = std::fs::File::create(Path::new(log_path))?;
    let mut log_file = std::io::LineWriter::new(file);

    let now = Utc::now().to_rfc3339();

    for verifier in &config.verifiers {
        let base = normalize_base_url(&verifier.verification_domain);

        for network in NETWORKS {
            if let Some(filter) = network_filter {
                if *network != filter {
                    continue;
                }
            }
            let url = format!("{}meta?network={}", base, network);
            let entry = fetch_meta(
                &client,
                &verifier.name,
                &verifier.verification_domain,
                &url,
                network,
                &now,
            );
            let line = serde_json::to_string(&entry).expect("serialize");
            writeln!(log_file, "{}", line)?;
        }
    }

    log_file.flush()?;
    Ok(())
}

fn compare_with_chain(
    log_path: &str,
    network: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(Path::new(log_path))?;

    let mut entries: Vec<LogEntry> = Vec::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<LogEntry>(line) {
            if entry.network == network {
                entries.push(entry);
            }
        }
    }

    if entries.is_empty() {
        eprintln!("[ncn-meta-cron] No {} entries found in {}", network, log_path);
        return Ok(());
    }

    let default_rpc = match network {
        "testnet" => "https://api.testnet.solana.com",
        _ => "https://api.mainnet-beta.solana.com",
    };
    // RPC URL selection (most specific wins):
    // - SOLANA_RPC_URL_MAINNET / SOLANA_RPC_URL_TESTNET
    // - SOLANA_RPC_URL (legacy / shared)
    // - default public RPC for the network
    let rpc_url = match network {
        "testnet" => env::var("SOLANA_RPC_URL_TESTNET")
            .or_else(|_| env::var("SOLANA_RPC_URL"))
            .unwrap_or_else(|_| default_rpc.to_string()),
        _ => env::var("SOLANA_RPC_URL_MAINNET")
            .or_else(|_| env::var("SOLANA_RPC_URL"))
            .unwrap_or_else(|_| default_rpc.to_string()),
    };

    // Program ID must be provided via NCN_PROGRAM_ID; no default is used.
    let program_id_str = env::var("NCN_PROGRAM_ID").unwrap_or_else(|_| {
        eprintln!(
            "[ncn-meta-cron] NCN_PROGRAM_ID env var is required for network '{}' (add it to your .env)",
            network
        );
        std::process::exit(1);
    });
    let program_id = Pubkey::from_str(&program_id_str)?;
    let client = RpcClient::new(rpc_url);
    let http = Client::builder().timeout(Duration::from_secs(15)).build()?;

    println!("network | name | meta_merkle_root | snapshot_hash | (domain)");

    // Read, once each, every ballot box this run might need: one per slot a
    // verifier reported, plus the slot the previous run recorded. See
    // `candidate_slots`.
    //
    // A ballot box has no winning ballot until operators vote it past its
    // consensus threshold, and operators upload their snapshot before they
    // vote. So for the few hours between the first upload for a slot and the
    // vote that settles it, the box exists but `winning_ballot` is still
    // all-zero. Checking a verifier against those zeros marks it `mismatch`,
    // and because operators upload at roughly the same time it marks the
    // whole fleet `mismatch` together, leaving the router with nobody to
    // route to. `slot_consensus_reached` is what tells "not voted yet" apart
    // from "voted, and this verifier disagrees", so read it with the ballot.
    let whitelist_path = whitelist_path_for(network);
    let mut states: std::collections::HashMap<u64, Result<BallotBoxState, String>> =
        std::collections::HashMap::new();
    for slot in candidate_slots(&entries, previous_whitelist_slot(&whitelist_path)) {
        states
            .entry(slot)
            .or_insert_with(|| fetch_ballot_box_state_cron(&client, &program_id, slot));
    }

    // Newest slot that has actually been voted through. A verifier sitting on
    // a newer, not-yet-voted snapshot is checked against this slot instead;
    // see `judge_pending_entry`.
    let reference_slot = states
        .iter()
        .filter_map(|(slot, state)| match state {
            Ok(s) if s.slot_consensus_reached != 0 => Some(*slot),
            _ => None,
        })
        .max();

    let mut whitelist_verifiers: Vec<WhitelistVerifier> = Vec::new();
    let mut chosen_slot: u64 = 0;

    for entry in &entries {
        if entry.error.is_some() {
            println!(
                "{} | {} | meta_merkle_root (error) | snapshot_hash (error) | ({})",
                network, entry.name, entry.domain
            );
            whitelist_verifiers.push(WhitelistVerifier {
                name: entry.name.clone(),
                domain: entry.domain.clone(),
                slot: entry.slot,
                status: "error".to_string(),
                reason: entry.error.clone(),
            });
            continue;
        }

        let verifier = match states.get(&entry.slot) {
            Some(Ok(state)) if state.slot_consensus_reached != 0 => {
                classify_entry_against_ballot(entry, &state.winning_ballot)
            }
            Some(Ok(_pending)) => judge_pending_entry(
                |url| fetch_meta_response(&http, url),
                entry,
                network,
                reference_slot,
                &states,
            ),
            Some(Err(e)) => {
                eprintln!(
                    "[ncn-meta-cron] fetch failed for {} (slot {}): {}",
                    entry.name, entry.slot, e
                );
                WhitelistVerifier {
                    name: entry.name.clone(),
                    domain: entry.domain.clone(),
                    slot: entry.slot,
                    status: "error".to_string(),
                    reason: Some(e.clone()),
                }
            }
            None => unreachable!("every non-error entry has a fetched state"),
        };

        println!(
            "{} | {} | slot {} | {}{} | ({})",
            network,
            entry.name,
            verifier.slot,
            verifier.status,
            verifier
                .reason
                .as_deref()
                .map(|r| format!(" ({r})"))
                .unwrap_or_default(),
            entry.domain
        );

        if verifier.status == "ok" && verifier.slot > chosen_slot {
            chosen_slot = verifier.slot;
        }
        whitelist_verifiers.push(verifier);
    }

    // Build and write the whitelist snapshot for this network. See
    // `persisted_snapshot_slot` for why a run that found no routable verifier
    // keeps the previous slot instead of writing the newest one it saw.
    let snapshot_slot = persisted_snapshot_slot(chosen_slot, reference_slot, &entries);

    // Before de-duplication, so an origin that is stale on one row cannot be
    // preferred as `ok` on another.
    let mut whitelist_verifiers = whitelist_verifiers;
    demote_stale_verifiers(
        &mut whitelist_verifiers,
        chosen_slot,
        max_verifier_slot_lag(),
    );

    // Collapse to one canonical record per verifier origin before persisting.
    // `ncn-router` samples whitelist rows uniformly, so duplicate rows for the
    // same domain would grant that origin extra routing weight.
    let whitelist_verifiers = dedupe_verifiers_by_domain(whitelist_verifiers);

    let whitelist = WhitelistSnapshot {
        network: network.to_string(),
        slot: snapshot_slot,
        updated_at: Utc::now().to_rfc3339(),
        verifiers: whitelist_verifiers,
    };

    if let Err(e) =
        std::fs::write(&whitelist_path, serde_json::to_string_pretty(&whitelist)?)
    {
        eprintln!(
            "[ncn-meta-cron] Failed to write whitelist file {}: {}",
            whitelist_path, e
        );
    } else {
        eprintln!(
            "[ncn-meta-cron] Updated whitelist file {} (slot {}, network={})",
            whitelist_path, snapshot_slot, network
        );
    }

    Ok(())
}

fn whitelist_path_for(network: &str) -> String {
    match network {
        "testnet" => env::var("NCN_WHITELIST_TESTNET_PATH")
            .unwrap_or_else(|_| "ncn_whitelist.testnet.json".to_string()),
        _ => env::var("NCN_WHITELIST_MAINNET_PATH")
            .unwrap_or_else(|_| "ncn_whitelist.mainnet.json".to_string()),
    }
}

/// Slot the previous run wrote to the whitelist file, if that file exists and
/// names a real slot.
///
/// Once every verifier has moved on to a slot that is still waiting on a vote,
/// nothing in this run's own data points at a voted-through slot any more. The
/// file is then the only remaining record of the last one.
fn previous_whitelist_slot(whitelist_path: &str) -> Option<u64> {
    let contents = fs::read_to_string(whitelist_path).ok()?;
    let previous: WhitelistSnapshot = serde_json::from_str(&contents).ok()?;
    (previous.slot != 0).then_some(previous.slot)
}

/// Slots whose ballot boxes this run needs to read: every slot a verifier
/// reported, plus `previous_slot` when there is one. Sorted and de-duplicated,
/// so a slot that half the fleet reports is still only fetched once.
fn candidate_slots(entries: &[LogEntry], previous_slot: Option<u64>) -> Vec<u64> {
    let mut slots: Vec<u64> = entries
        .iter()
        .filter(|e| e.error.is_none())
        .map(|e| e.slot)
        .chain(previous_slot)
        .collect();
    slots.sort_unstable();
    slots.dedup();
    slots
}

/// Slot to record in the whitelist file.
///
/// This field is what the next run reads back as its reference, via
/// `previous_whitelist_slot`, so it must never move backwards: whatever this
/// run writes is the oldest slot a later run can fall back to.
///
/// That makes it the newer of `chosen_slot` (the newest slot with a routable
/// verifier on it) and `reference_slot` (the newest slot voted through). They
/// are usually the same, but either one can be ahead:
///
/// - no routable verifier at all leaves `chosen_slot` at 0, so the reference
///   is written and survives the run;
/// - a lagging operator can be the only routable verifier while everyone else
///   sits on a slot that is still unvoted. Its slot is older than the
///   reference, and writing it would drop the newest voted-through slot.
///
/// Only a first run, with no voted-through slot anywhere, has neither and
/// falls back to the newest slot reported.
fn persisted_snapshot_slot(
    chosen_slot: u64,
    reference_slot: Option<u64>,
    entries: &[LogEntry],
) -> u64 {
    match reference_slot {
        Some(reference_slot) => chosen_slot.max(reference_slot),
        None if chosen_slot != 0 => chosen_slot,
        None => entries.iter().map(|e| e.slot).max().unwrap_or_default(),
    }
}

fn max_verifier_slot_lag() -> u64 {
    parse_max_verifier_slot_lag(env::var("NCN_MAX_VERIFIER_SLOT_LAG").ok().as_deref())
}

/// A malformed override falls back to the default but says so: silently widening
/// the budget would keep verifiers in rotation that the configured policy meant
/// to drop, which is the failure this whole check exists to prevent.
fn parse_max_verifier_slot_lag(raw: Option<&str>) -> u64 {
    let Some(value) = raw else {
        return DEFAULT_MAX_VERIFIER_SLOT_LAG;
    };
    value.trim().parse().unwrap_or_else(|_| {
        eprintln!(
            "[ncn-meta-cron] ignoring NCN_MAX_VERIFIER_SLOT_LAG={value:?}: not a slot count; \
             using {DEFAULT_MAX_VERIFIER_SLOT_LAG}. Verifiers the configured value would have \
             dropped stay in rotation until this is corrected."
        );
        DEFAULT_MAX_VERIFIER_SLOT_LAG
    })
}

/// Demote `ok` verifiers whose snapshot trails `freshest_slot` by more than
/// `max_lag`, so the router stops sampling them.
///
/// `status == "ok"` only checks a verifier's root against the on-chain ballot at
/// the slot *it* reported, which an operator that stopped uploading months ago
/// still satisfies — it stays `ok` while 404ing every current proof request.
///
/// Measured against the freshest peer, not the chain tip: snapshots are only
/// produced when a proposal needs one, so between proposals the whole fleet
/// legitimately sits at the same older slot. `freshest_slot` is the max over
/// already-`ok` verifiers, so its holder has zero lag and always survives —
/// this narrows the routing pool but never empties it.
fn demote_stale_verifiers(verifiers: &mut [WhitelistVerifier], freshest_slot: u64, max_lag: u64) {
    for verifier in verifiers.iter_mut() {
        if verifier.status != "ok" {
            continue;
        }
        let lag = freshest_slot.saturating_sub(verifier.slot);
        if lag > max_lag {
            eprintln!(
                "[ncn-meta-cron] dropping stale verifier '{}' ({}): slot {} is {} slots behind the freshest verifier ({}), limit {}",
                verifier.name, verifier.domain, verifier.slot, lag, freshest_slot, max_lag
            );
            verifier.status = "stale".to_string();
            verifier.reason = Some(format!(
                "snapshot slot {} is {} slots behind freshest verifier ({}); limit {}",
                verifier.slot, lag, freshest_slot, max_lag
            ));
        }
    }
}

/// Canonical routing identity for a verifier origin. `ncn-router` redirects to
/// `domain` after ensuring a trailing slash, so `http://x` and `http://x/`
/// resolve to the same upstream. De-duplicating on this canonical form (rather
/// than the raw string) prevents slash variants from surviving as separate
/// whitelist rows that the router would later collapse to one upstream — i.e.
/// extra routing tickets for a single origin. Matches `canonical_domain` in
/// `ncn-router`'s `router.rs`.
fn canonical_domain(domain: &str) -> String {
    if domain.ends_with('/') {
        domain.to_string()
    } else {
        format!("{}/", domain)
    }
}

/// Keep one canonical whitelist record per verifier origin (canonical `domain`).
///
/// When a domain appears more than once (e.g. it is listed twice in the config,
/// possibly with trailing-slash variants), an `ok` record is preferred over a
/// non-`ok` one so a transient failure on one poll cannot shadow a successful
/// poll for the same origin. Among records of the same rank the first occurrence
/// wins, which keeps the output deterministic in config order. This still
/// collapses each origin to a single row, so a verifier can never hold extra
/// routing tickets — it just avoids demoting an origin that did verify
/// successfully. Mirrors the `status == "ok"`-first selection in `ncn-router`'s
/// `select_routable_verifiers`.
fn dedupe_verifiers_by_domain(verifiers: Vec<WhitelistVerifier>) -> Vec<WhitelistVerifier> {
    let mut index_by_domain: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut deduped: Vec<WhitelistVerifier> = Vec::new();
    for verifier in verifiers {
        let key = canonical_domain(&verifier.domain);
        match index_by_domain.get(&key) {
            None => {
                index_by_domain.insert(key, deduped.len());
                deduped.push(verifier);
            }
            Some(&idx) => {
                // Replace a previously kept non-ok record with an ok one; otherwise
                // keep what we already have (first-wins within the same rank).
                if deduped[idx].status != "ok" && verifier.status == "ok" {
                    eprintln!(
                        "[ncn-meta-cron] duplicate whitelist entry for domain '{}': preferring ok record (name='{}') over previously kept status '{}'",
                        verifier.domain, verifier.name, deduped[idx].status
                    );
                    deduped[idx] = verifier;
                } else {
                    eprintln!(
                        "[ncn-meta-cron] dropping duplicate whitelist entry for domain '{}' (name='{}', status='{}')",
                        verifier.domain, verifier.name, verifier.status
                    );
                }
            }
        }
    }
    deduped
}

fn fetch_ballot_box_state_cron(
    client: &RpcClient,
    program_id: &Pubkey,
    snapshot_slot: u64,
) -> Result<BallotBoxState, String> {
    let seeds: &[&[u8]] = &[b"BallotBox", &snapshot_slot.to_le_bytes()];
    let (ballot_box_pda, _bump) = Pubkey::find_program_address(seeds, program_id);

    match client.get_account(&ballot_box_pda) {
        Ok(account) => {
            if account.data.len() < 8 {
                return Err("Account data too short to contain discriminator.".to_string());
            }
            let (disc, rest) = account.data.split_at(8);
            if disc != BALLOT_BOX_DISCRIMINATOR {
                return Err(
                    "Account discriminator does not match BallotBox; wrong account type."
                        .to_string(),
                );
            }
            parse_ballot_box_state(rest)
        }
        Err(e) => Err(format!(
            "Failed to fetch BallotBox account: {}",
            e.to_string()
        )),
    }
}

/// Classify a verifier whose newest snapshot, at `entry.slot`, sits in a
/// ballot box that has not been voted through yet.
///
/// Nothing on chain says whether that snapshot is correct, so ask the verifier
/// for `reference_slot` (the newest slot that was voted through) and check its
/// answer against that ballot instead. A verifier that still serves the agreed
/// snapshot can still answer proof requests for the proposal being voted on,
/// so it stays `ok`.
///
/// The result is `pending`, which is not routable, whenever there is nothing
/// to check against:
/// - no slot has been voted through yet, as on a brand new deployment;
/// - the reference ballot box could not be read;
/// - the verifier no longer serves that slot, usually a 404 after pruning;
/// - the verifier answered with a different slot, which means verifier-service
///   older than 0.6: it ignores `slot` on `/meta` and returns its newest
///   snapshot regardless.
fn judge_pending_entry(
    fetch_meta_at: impl Fn(&str) -> Result<MetaResponse, String>,
    entry: &LogEntry,
    network: &str,
    reference_slot: Option<u64>,
    states: &std::collections::HashMap<u64, Result<BallotBoxState, String>>,
) -> WhitelistVerifier {
    let pending = |reason: String| WhitelistVerifier {
        name: entry.name.clone(),
        domain: entry.domain.clone(),
        slot: entry.slot,
        status: "pending".to_string(),
        reason: Some(reason),
    };

    let Some(reference_slot) = reference_slot else {
        return pending(format!(
            "slot {} has no consensus yet and no earlier finalized snapshot exists",
            entry.slot
        ));
    };
    let Some(Ok(reference)) = states.get(&reference_slot) else {
        return pending(format!(
            "slot {} has no consensus yet and the reference ballot box could not be read",
            entry.slot
        ));
    };

    let url = format!(
        "{}meta?network={}&slot={}",
        normalize_base_url(&entry.domain),
        network,
        reference_slot
    );
    let meta = match fetch_meta_at(&url) {
        Ok(meta) => meta,
        Err(e) => {
            return pending(format!(
                "slot {} has no consensus yet; /meta?slot={} {}",
                entry.slot, reference_slot, e
            ))
        }
    };
    // Same rule as `log_entry_from_meta`: trust the network we asked for, not
    // the one the answer claims, and say so when the two disagree. The row
    // below takes its network from `entry` either way. A snapshot from another
    // network cannot match this network's ballot, so a mislabelled answer can
    // only cost the verifier an `ok`, never earn it one.
    if meta.network != network {
        eprintln!(
            "[ncn-meta-cron] verifier '{}' ({}) reported network '{}' for requested network '{}' at slot {}; judging it as '{}'",
            entry.name, entry.domain, meta.network, network, reference_slot, network
        );
    }
    if meta.slot != reference_slot {
        return pending(format!(
            "slot {} has no consensus yet; verifier answered /meta?slot={} with slot {}",
            entry.slot, reference_slot, meta.slot
        ));
    }

    let reference_entry = LogEntry {
        slot: reference_slot,
        merkle_root: meta.merkle_root,
        snapshot_hash: meta.snapshot_hash,
        ..entry.clone()
    };
    let mut verifier = classify_entry_against_ballot(&reference_entry, &reference.winning_ballot);
    if verifier.status == "ok" {
        verifier.reason = Some(format!(
            "newest snapshot {} awaits consensus; judged on finalized slot {}",
            entry.slot, reference_slot
        ));
    }
    verifier
}

#[cfg(test)]
fn parse_winning_ballot(data: &[u8]) -> Result<Ballot, String> {
    parse_ballot_box_state(data).map(|state| state.winning_ballot)
}

/// The two `BallotBox` fields this cron reads. `slot_consensus_reached` stays
/// 0, and `winning_ballot` stays all-zero, until operators vote the box past
/// its threshold.
#[derive(Debug)]
struct BallotBoxState {
    slot_consensus_reached: u64,
    winning_ballot: Ballot,
}

fn parse_ballot_box_state(mut data: &[u8]) -> Result<BallotBoxState, String> {
    // BallotBox layout (after 8-byte discriminator), as per IDL:
    // bump: u8
    // epoch: u64
    // slot_created: u64
    // slot_consensus_reached: u64
    // min_consensus_threshold_bps: u16
    // winning_ballot: Ballot { [u8;32], [u8;32] }
    read_u8(&mut data)?;
    read_u64(&mut data)?;
    read_u64(&mut data)?;
    let slot_consensus_reached = read_u64(&mut data)?;
    read_u16(&mut data)?;

    let winning_ballot = Ballot::deserialize(&mut data).map_err(|e| e.to_string())?;
    Ok(BallotBoxState {
        slot_consensus_reached,
        winning_ballot,
    })
}

/// Compare what a verifier reported in `entry` against the ballot the chain
/// agreed on: `ok` when both the meta merkle root and the snapshot hash match,
/// `mismatch` otherwise, with `reason` recording which of the two failed.
fn classify_entry_against_ballot(entry: &LogEntry, ballot: &Ballot) -> WhitelistVerifier {
    let onchain_merkle_root = bytes32_base58(&ballot.meta_merkle_root);
    let onchain_snapshot_hash = bytes32_base58(&ballot.snapshot_hash);
    let merkle_match = onchain_merkle_root == entry.merkle_root;
    let snapshot_match = onchain_snapshot_hash == entry.snapshot_hash;
    let status = if merkle_match && snapshot_match {
        "ok".to_string()
    } else {
        "mismatch".to_string()
    };
    let reason = if status == "ok" {
        None
    } else {
        Some(format!(
            "merkle_match={}, snapshot_match={}",
            merkle_match, snapshot_match
        ))
    };
    WhitelistVerifier {
        name: entry.name.clone(),
        domain: entry.domain.clone(),
        slot: entry.slot,
        status,
        reason,
    }
}

fn take<const N: usize>(data: &mut &[u8]) -> Result<[u8; N], String> {
    if data.len() < N {
        return Err(format!("not enough bytes: need {}, have {}", N, data.len()));
    }
    let (head, tail) = data.split_at(N);
    *data = tail;
    let mut out = [0u8; N];
    out.copy_from_slice(head);
    Ok(out)
}

fn read_u8(data: &mut &[u8]) -> Result<u8, String> {
    Ok(take::<1>(data)?[0])
}

fn read_u16(data: &mut &[u8]) -> Result<u16, String> {
    Ok(u16::from_le_bytes(take::<2>(data)?))
}

fn read_u64(data: &mut &[u8]) -> Result<u64, String> {
    Ok(u64::from_le_bytes(take::<8>(data)?))
}

fn bytes32_base58(b: &[u8; 32]) -> String {
    solana_sdk::bs58::encode(b).into_string()
}

fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let s = std::fs::read_to_string(Path::new(path))?;
    let config: Config = toml::from_str(&s)?;
    Ok(config)
}

/// Ensure base URL ends with exactly one slash for appending "meta?network=..."
fn normalize_base_url(domain: &str) -> String {
    let s = domain.trim();
    if s.is_empty() {
        return "https://localhost/".to_string();
    }
    let mut s = s.to_string();
    if !s.ends_with('/') {
        s.push('/');
    }
    s
}

/// Build a successful log entry, binding it to the *requested* `network` rather
/// than the verifier-reported `meta.network`.
///
/// A malicious verifier could otherwise label a `testnet` response as `mainnet`
/// (or vice versa) so that a single origin lands twice in one network's whitelist
/// as `ok`, minting duplicate routing tickets in `ncn-router`. We always trust the
/// network we asked for and only warn when the verifier disagrees.
fn log_entry_from_meta(
    name: &str,
    domain: &str,
    network: &str,
    timestamp: &str,
    meta: MetaResponse,
) -> LogEntry {
    if meta.network != network {
        eprintln!(
            "[ncn-meta-cron] verifier '{}' ({}) reported network '{}' for requested network '{}'; binding entry to requested network",
            name, domain, meta.network, network
        );
    }
    LogEntry {
        timestamp: timestamp.to_string(),
        name: name.to_string(),
        domain: domain.to_string(),
        network: network.to_string(),
        slot: meta.slot,
        merkle_root: meta.merkle_root,
        snapshot_hash: meta.snapshot_hash,
        created_at: meta.created_at,
        error: None,
    }
}

/// Fetch and decode one verifier's `/meta` response.
fn fetch_meta_response(client: &Client, url: &str) -> Result<MetaResponse, String> {
    let resp = client.get(url).send().map_err(|e| format!("failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("returned HTTP {}", resp.status()));
    }
    resp.json::<MetaResponse>()
        .map_err(|e| format!("unparsable: {e}"))
}

fn fetch_meta(
    client: &Client,
    name: &str,
    domain: &str,
    url: &str,
    network: &str,
    timestamp: &str,
) -> LogEntry {
    match client.get(url).send() {
        Ok(resp) => {
            if !resp.status().is_success() {
                return LogEntry {
                    timestamp: timestamp.to_string(),
                    name: name.to_string(),
                    domain: domain.to_string(),
                    network: network.to_string(),
                    slot: 0,
                    merkle_root: String::new(),
                    snapshot_hash: String::new(),
                    created_at: None,
                    error: Some(format!("HTTP {}", resp.status())),
                };
            }
            match resp.json::<MetaResponse>() {
                Ok(meta) => log_entry_from_meta(name, domain, network, timestamp, meta),
                Err(e) => LogEntry {
                    timestamp: timestamp.to_string(),
                    name: name.to_string(),
                    domain: domain.to_string(),
                    network: network.to_string(),
                    slot: 0,
                    merkle_root: String::new(),
                    snapshot_hash: String::new(),
                    created_at: None,
                    error: Some(format!("JSON: {}", e)),
                },
            }
        }
        Err(e) => LogEntry {
            timestamp: timestamp.to_string(),
            name: name.to_string(),
            domain: domain.to_string(),
            network: network.to_string(),
            slot: 0,
            merkle_root: String::new(),
            snapshot_hash: String::new(),
            created_at: None,
            error: Some(e.to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(network: &str, slot: u64) -> MetaResponse {
        MetaResponse {
            network: network.to_string(),
            slot,
            merkle_root: "root".to_string(),
            snapshot_hash: "hash".to_string(),
            created_at: None,
        }
    }

    fn verifier(name: &str, domain: &str, status: &str) -> WhitelistVerifier {
        verifier_at(name, domain, status, 0)
    }

    fn verifier_at(name: &str, domain: &str, status: &str, slot: u64) -> WhitelistVerifier {
        WhitelistVerifier {
            name: name.to_string(),
            domain: domain.to_string(),
            slot,
            status: status.to_string(),
            reason: None,
        }
    }

    fn statuses(verifiers: &[WhitelistVerifier]) -> Vec<&str> {
        verifiers.iter().map(|v| v.status.as_str()).collect()
    }

    #[test]
    fn log_entry_binds_to_requested_network_not_reported() {
        // Malicious verifier mislabels a testnet response as "mainnet".
        let entry = log_entry_from_meta(
            "malicious",
            "http://evil",
            "testnet",
            "2026-01-01T00:00:00Z",
            meta("mainnet", 222),
        );
        // The entry is recorded under the network we actually asked for, so it
        // cannot leak into the mainnet whitelist.
        assert_eq!(entry.network, "testnet");
        assert_eq!(entry.slot, 222);
        assert!(entry.error.is_none());
    }

    #[test]
    fn log_entry_keeps_network_when_report_matches() {
        let entry = log_entry_from_meta(
            "honest",
            "http://good",
            "mainnet",
            "2026-01-01T00:00:00Z",
            meta("mainnet", 111),
        );
        assert_eq!(entry.network, "mainnet");
        assert_eq!(entry.slot, 111);
    }

    #[test]
    fn dedupe_collapses_duplicate_domains_keeping_first() {
        let verifiers = vec![
            verifier("first", "http://dup", "ok"),
            verifier("second", "http://dup", "ok"),
            verifier("other", "http://other", "ok"),
        ];
        let deduped = dedupe_verifiers_by_domain(verifiers);
        assert_eq!(deduped.len(), 2);
        // Among same-rank records the first occurrence wins; the duplicate origin
        // is collapsed to one record.
        assert_eq!(deduped[0].name, "first");
        assert_eq!(deduped[0].domain, "http://dup");
        assert_eq!(deduped[1].domain, "http://other");
        assert_eq!(
            deduped
                .iter()
                .filter(|v| v.domain == "http://dup")
                .count(),
            1
        );
    }

    #[test]
    fn dedupe_prefers_ok_over_non_ok_for_same_domain() {
        // A transient failure on the first poll must not shadow a later successful
        // poll for the same origin.
        let verifiers = vec![
            verifier("transient-fail", "http://dup", "error"),
            verifier("succeeded", "http://dup", "ok"),
            verifier("other", "http://other", "ok"),
        ];
        let deduped = dedupe_verifiers_by_domain(verifiers);
        assert_eq!(deduped.len(), 2);
        // Still one row per origin (no extra routing tickets), and the ok record
        // wins even though the error record came first. Order is preserved: the
        // duplicated domain keeps its first-seen position.
        let dup = deduped
            .iter()
            .find(|v| v.domain == "http://dup")
            .expect("dup domain present");
        assert_eq!(dup.status, "ok");
        assert_eq!(dup.name, "succeeded");
        assert_eq!(deduped[0].domain, "http://dup");
    }

    #[test]
    fn stale_ok_verifier_is_demoted() {
        // The reported failure mode: a verifier stopped uploading months ago, so
        // its data still matches the on-chain ballot for its own old slot and it
        // stays "ok" — while 404ing every request for a current snapshot.
        let mut verifiers = vec![
            verifier_at("fresh", "http://fresh", "ok", 436_919_878),
            verifier_at("frozen", "http://frozen", "ok", 422_497_000),
        ];
        demote_stale_verifiers(&mut verifiers, 436_919_878, DEFAULT_MAX_VERIFIER_SLOT_LAG);
        assert_eq!(statuses(&verifiers), vec!["ok", "stale"]);
        assert!(verifiers[1]
            .reason
            .as_deref()
            .unwrap()
            .contains("behind freshest verifier"));
    }

    #[test]
    fn verifier_within_lag_budget_stays_ok() {
        // Operators do not upload in lockstep; a verifier still inside the lag
        // budget must keep its routing ticket.
        let mut verifiers = vec![
            verifier_at("fresh", "http://fresh", "ok", 436_941_951),
            verifier_at("slightly-behind", "http://behind", "ok", 436_919_878),
        ];
        demote_stale_verifiers(&mut verifiers, 436_941_951, DEFAULT_MAX_VERIFIER_SLOT_LAG);
        assert_eq!(statuses(&verifiers), vec!["ok", "ok"]);
    }

    #[test]
    fn fleet_at_a_common_older_slot_is_not_demoted() {
        // Snapshots are only produced when a proposal needs one, so between
        // proposals the whole fleet legitimately sits at the same older slot.
        // Measuring against the freshest peer (not the chain tip) keeps everyone.
        let mut verifiers = vec![
            verifier_at("a", "http://a", "ok", 100_000),
            verifier_at("b", "http://b", "ok", 100_000),
            verifier_at("c", "http://c", "ok", 100_000),
        ];
        demote_stale_verifiers(&mut verifiers, 100_000, DEFAULT_MAX_VERIFIER_SLOT_LAG);
        assert_eq!(statuses(&verifiers), vec!["ok", "ok", "ok"]);
    }

    #[test]
    fn demotion_never_empties_the_routing_pool() {
        // `freshest_slot` is the max over ok verifiers, so its holder has zero lag
        // and always survives. Narrowing the pool is fine; emptying it is not.
        let mut verifiers = vec![
            verifier_at("frozen-1", "http://f1", "ok", 1),
            verifier_at("freshest", "http://f2", "ok", 10_000_000),
            verifier_at("frozen-2", "http://f3", "ok", 2),
        ];
        demote_stale_verifiers(&mut verifiers, 10_000_000, DEFAULT_MAX_VERIFIER_SLOT_LAG);
        assert_eq!(verifiers.iter().filter(|v| v.status == "ok").count(), 1);
        assert_eq!(verifiers[1].status, "ok");
    }

    #[test]
    fn non_ok_verifiers_keep_their_original_status() {
        // A verifier that failed the on-chain root comparison must stay
        // "mismatch"/"error" — staleness must not overwrite the real reason.
        let mut verifiers = vec![
            verifier_at("fresh", "http://fresh", "ok", 436_919_878),
            verifier_at("bad-root", "http://bad", "mismatch", 1),
            verifier_at("unreachable", "http://down", "error", 0),
        ];
        demote_stale_verifiers(&mut verifiers, 436_919_878, DEFAULT_MAX_VERIFIER_SLOT_LAG);
        assert_eq!(statuses(&verifiers), vec!["ok", "mismatch", "error"]);
    }

    #[test]
    fn no_ok_verifiers_demotes_nothing() {
        // `chosen_slot` stays 0 when nothing verified; every lag saturates to 0.
        let mut verifiers = vec![
            verifier_at("a", "http://a", "error", 500),
            verifier_at("b", "http://b", "mismatch", 600),
        ];
        demote_stale_verifiers(&mut verifiers, 0, DEFAULT_MAX_VERIFIER_SLOT_LAG);
        assert_eq!(statuses(&verifiers), vec!["error", "mismatch"]);
    }

    #[test]
    fn lag_override_is_parsed_and_falls_back_loudly() {
        assert_eq!(parse_max_verifier_slot_lag(Some("1000")), 1_000);
        assert_eq!(parse_max_verifier_slot_lag(Some("  1000  ")), 1_000);
        // 0 is a legitimate "any lag is stale" policy, not a parse failure.
        assert_eq!(parse_max_verifier_slot_lag(Some("0")), 0);
        assert_eq!(
            parse_max_verifier_slot_lag(None),
            DEFAULT_MAX_VERIFIER_SLOT_LAG
        );
        // Malformed values fall back rather than taking the cron down, but the
        // fallback is announced — silently widening the budget would keep stale
        // verifiers routable against the operator's intent.
        for bad in ["", "abc", "-1", "1.5", "1_000"] {
            assert_eq!(
                parse_max_verifier_slot_lag(Some(bad)),
                DEFAULT_MAX_VERIFIER_SLOT_LAG,
                "unexpected parse of {bad:?}"
            );
        }
    }

    #[test]
    fn lag_exactly_at_the_limit_is_allowed() {
        // The bound is inclusive: only a verifier strictly past the budget drops.
        let mut verifiers = vec![
            verifier_at("fresh", "http://fresh", "ok", DEFAULT_MAX_VERIFIER_SLOT_LAG),
            verifier_at("edge", "http://edge", "ok", 0),
        ];
        demote_stale_verifiers(
            &mut verifiers,
            DEFAULT_MAX_VERIFIER_SLOT_LAG,
            DEFAULT_MAX_VERIFIER_SLOT_LAG,
        );
        assert_eq!(statuses(&verifiers), vec!["ok", "ok"]);
    }

    #[test]
    fn dedupe_collapses_trailing_slash_domain_variants() {
        // `http://evil` and `http://evil/` resolve to the same upstream in the
        // router, so they must collapse to a single whitelist row even though the
        // raw strings differ.
        let verifiers = vec![
            verifier("evil", "http://evil", "ok"),
            verifier("evil-slash", "http://evil/", "ok"),
            verifier("other", "http://other", "ok"),
        ];
        let deduped = dedupe_verifiers_by_domain(verifiers);
        assert_eq!(deduped.len(), 2);
        assert_eq!(
            deduped
                .iter()
                .filter(|v| canonical_domain(&v.domain) == "http://evil/")
                .count(),
            1
        );
    }
}

#[cfg(test)]
mod pending_consensus {
    //! Verifiers whose newest snapshot is still waiting on a vote.
    //!
    //! `init_ballot_box` leaves `winning_ballot` all-zero until `cast_vote`
    //! carries the box past its threshold, and operators upload their snapshot
    //! before they vote. Checking a verifier against those zeros therefore
    //! marks every operator that uploaded on time as `mismatch` at the same
    //! moment, and `select_routable_verifiers` in `router.rs` is left with
    //! nobody to route to while the previous proposal is still being voted on.
    //!
    //! These tests pin down the behaviour that avoids that: such a verifier is
    //! checked against the newest snapshot that was voted through and that it
    //! still serves.
    use super::*;
    use std::collections::HashMap;

    const S1: u64 = 440_641_000; // voted through on mainnet
    const S2: u64 = 441_073_000; // uploaded, not voted on yet
    const S1_ROOT: &str = "2Jwjfi6KWQmqMraTkCcWqdj3VvyGcVgfNGYdzTQmaUj8";
    const S1_HASH: &str = "7TgfsjkYnyJ6DWKG8CTc7Yt9426fKRqs5Fh2xvunvXZ2";
    const S2_ROOT: &str = "3MpDoa4y8sNvuLd2ba9YJGzDeUnmA8b7fdrWYqW7QKf6";
    const S2_HASH: &str = "4fN6P4hwZXMAsn1KpTbHEk2rDdsyG8XzM9Dq8uwHzrEP";

    fn bs58_decode32(s: &str) -> [u8; 32] {
        let bytes = solana_sdk::bs58::decode(s).into_vec().unwrap();
        bytes.as_slice().try_into().unwrap()
    }

    /// Bytes of a `BallotBox` exactly as `init_ballot_box` leaves it. The
    /// handler sets bump, epoch, slot_created, threshold, expiry and
    /// snapshot_slot, and nothing else, so `winning_ballot` is still
    /// `Ballot::default()`.
    fn freshly_initialized_ballot_box(snapshot_slot: u64) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&BALLOT_BOX_DISCRIMINATOR);
        data.push(255);
        data.extend_from_slice(&1012u64.to_le_bytes());
        data.extend_from_slice(&437_227_093u64.to_le_bytes());
        data.extend_from_slice(&0u64.to_le_bytes()); // slot_consensus_reached
        data.extend_from_slice(&6000u16.to_le_bytes());
        data.extend_from_slice(&[0u8; 64]); // winning_ballot
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&0i64.to_le_bytes());
        data.extend_from_slice(&snapshot_slot.to_le_bytes());
        data
    }

    fn entry(name: &str, slot: u64, root: &str, hash: &str) -> LogEntry {
        LogEntry {
            timestamp: String::new(),
            name: name.to_string(),
            domain: format!("https://{name}.example"),
            network: "mainnet".to_string(),
            slot,
            merkle_root: root.to_string(),
            snapshot_hash: hash.to_string(),
            created_at: None,
            error: None,
        }
    }

    fn states() -> HashMap<u64, Result<BallotBoxState, String>> {
        let pending = parse_ballot_box_state(&freshly_initialized_ballot_box(S2)[8..]).unwrap();
        let finalized = BallotBoxState {
            slot_consensus_reached: 440_679_236,
            winning_ballot: Ballot {
                meta_merkle_root: bs58_decode32(S1_ROOT),
                snapshot_hash: bs58_decode32(S1_HASH),
            },
        };
        HashMap::from([(S1, Ok(finalized)), (S2, Ok(pending))])
    }

    fn meta(slot: u64, root: &str, hash: &str) -> MetaResponse {
        MetaResponse {
            network: "mainnet".to_string(),
            slot,
            merkle_root: root.to_string(),
            snapshot_hash: hash.to_string(),
            created_at: None,
        }
    }

    #[test]
    fn a_fresh_ballot_box_parses_to_a_zero_winning_ballot() {
        let state = parse_ballot_box_state(&freshly_initialized_ballot_box(S2)[8..]).unwrap();
        assert_eq!(state.slot_consensus_reached, 0);
        assert_eq!(state.winning_ballot.meta_merkle_root, [0u8; 32]);
        assert_eq!(state.winning_ballot.snapshot_hash, [0u8; 32]);
    }

    #[test]
    fn the_naive_comparison_demotes_every_up_to_date_verifier() {
        // How the old code behaved, kept to show what the fallback prevents.
        let winning = parse_winning_ballot(&freshly_initialized_ballot_box(S2)[8..]).unwrap();
        let verifiers: Vec<WhitelistVerifier> = ["a", "b", "c", "d"]
            .map(|n| entry(n, S2, S2_ROOT, S2_HASH))
            .iter()
            .map(|e| classify_entry_against_ballot(e, &winning))
            .collect();
        assert!(verifiers.iter().all(|v| v.status == "mismatch"));
        assert!(!verifiers.iter().any(|v| v.status == "ok"));
    }

    #[test]
    fn a_verifier_still_serving_the_finalized_snapshot_stays_routable() {
        let e = entry("a", S2, S2_ROOT, S2_HASH);
        let fetched = std::cell::RefCell::new(Vec::new());
        let v = judge_pending_entry(
            |url| {
                fetched.borrow_mut().push(url.to_string());
                Ok(meta(S1, S1_ROOT, S1_HASH))
            },
            &e,
            "mainnet",
            Some(S1),
            &states(),
        );
        assert_eq!(v.status, "ok", "{v:?}");
        // Recorded at the reference slot, so the staleness check later in the
        // run measures it there too.
        assert_eq!(v.slot, S1);
        assert!(v.reason.unwrap().contains("awaits consensus"));
        assert_eq!(
            fetched.borrow().as_slice(),
            [format!("https://a.example/meta?network=mainnet&slot={S1}")]
        );
    }

    #[test]
    fn a_verifier_whose_finalized_snapshot_disagrees_with_chain_is_a_mismatch() {
        let e = entry("a", S2, S2_ROOT, S2_HASH);
        let v = judge_pending_entry(
            |_| Ok(meta(S1, S2_ROOT, S1_HASH)),
            &e,
            "mainnet",
            Some(S1),
            &states(),
        );
        assert_eq!(v.status, "mismatch");
    }

    #[test]
    fn a_verifier_that_ignores_the_slot_parameter_is_pending_not_ok() {
        // verifier-service < 0.6 answers /meta?slot= with its newest snapshot.
        let e = entry("a", S2, S2_ROOT, S2_HASH);
        let v = judge_pending_entry(
            |_| Ok(meta(S2, S2_ROOT, S2_HASH)),
            &e,
            "mainnet",
            Some(S1),
            &states(),
        );
        assert_eq!(v.status, "pending", "{v:?}");
        assert!(v.reason.unwrap().contains("answered /meta?slot="));
    }

    #[test]
    fn a_verifier_that_pruned_the_finalized_snapshot_is_pending() {
        let e = entry("a", S2, S2_ROOT, S2_HASH);
        let v = judge_pending_entry(
            |_| Err("returned HTTP 404 Not Found".to_string()),
            &e,
            "mainnet",
            Some(S1),
            &states(),
        );
        assert_eq!(v.status, "pending");
        assert!(v.reason.unwrap().contains("HTTP 404"));
    }

    #[test]
    fn the_very_first_snapshot_has_nothing_to_fall_back_to() {
        let e = entry("a", S2, S2_ROOT, S2_HASH);
        let mut only_pending = states();
        only_pending.remove(&S1);
        let v = judge_pending_entry(
            |_| panic!("no reference slot, nothing to fetch"),
            &e,
            "mainnet",
            None,
            &only_pending,
        );
        assert_eq!(v.status, "pending");
    }

    #[test]
    fn the_previous_whitelist_slot_keeps_a_reference_once_the_whole_fleet_is_pending() {
        // Every verifier has uploaded S2 and none of them reports S1 any more,
        // so this run's own data names no voted-through slot. The slot the
        // previous run wrote to the whitelist file still does.
        let entries: Vec<LogEntry> = ["a", "b", "c", "d"]
            .iter()
            .map(|n| entry(n, S2, S2_ROOT, S2_HASH))
            .collect();
        assert_eq!(candidate_slots(&entries, None), vec![S2]);
        assert_eq!(candidate_slots(&entries, Some(S1)), vec![S1, S2]);

        let reference_slot = states()
            .iter()
            .filter_map(|(slot, state)| match state {
                Ok(s) if s.slot_consensus_reached != 0 => Some(*slot),
                _ => None,
            })
            .max();
        assert_eq!(reference_slot, Some(S1));

        let verifiers: Vec<WhitelistVerifier> = entries
            .iter()
            .map(|e| {
                judge_pending_entry(
                    |_| Ok(meta(S1, S1_ROOT, S1_HASH)),
                    e,
                    "mainnet",
                    reference_slot,
                    &states(),
                )
            })
            .collect();
        assert!(verifiers.iter().all(|v| v.status == "ok"), "{verifiers:?}");
    }

    #[test]
    fn the_previous_whitelist_slot_is_read_from_the_file_the_cron_writes() {
        let dir = std::env::temp_dir().join(format!(
            "ncn-router-whitelist-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ncn_whitelist.mainnet.json");
        let path = path.to_str().unwrap();

        assert_eq!(previous_whitelist_slot(path), None, "no file yet");

        let written = WhitelistSnapshot {
            network: "mainnet".to_string(),
            slot: S1,
            updated_at: "2026-09-16T00:00:00Z".to_string(),
            verifiers: [entry("a", S1, S1_ROOT, S1_HASH)]
                .iter()
                .map(|e| {
                    classify_entry_against_ballot(
                        e,
                        &states()[&S1].as_ref().unwrap().winning_ballot,
                    )
                })
                .collect(),
        };
        fs::write(path, serde_json::to_string_pretty(&written).unwrap()).unwrap();
        assert_eq!(previous_whitelist_slot(path), Some(S1));

        // Slot 0 means an older or half-written file, which is no reference.
        fs::write(
            path,
            r#"{"network":"mainnet","slot":0,"updated_at":"","verifiers":[]}"#,
        )
        .unwrap();
        assert_eq!(previous_whitelist_slot(path), None);

        fs::write(path, "not json").unwrap();
        assert_eq!(previous_whitelist_slot(path), None);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn finalized_reference_survives_consecutive_all_pending_runs() {
        let entries: Vec<LogEntry> = ["a", "b", "c", "d"]
            .iter()
            .map(|n| entry(n, S2, S2_ROOT, S2_HASH))
            .collect();

        let dir = std::env::temp_dir().join(format!(
            "ncn-router-reference-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ncn_whitelist.mainnet.json");
        let path = path.to_str().unwrap();

        let mut previous = Some(S1);
        for _ in 0..2 {
            let persisted = persisted_snapshot_slot(0, previous, &entries);
            assert_eq!(persisted, S1);
            let written = WhitelistSnapshot {
                network: "mainnet".to_string(),
                slot: persisted,
                updated_at: "2026-09-16T00:00:00Z".to_string(),
                verifiers: Vec::new(),
            };
            fs::write(path, serde_json::to_string_pretty(&written).unwrap()).unwrap();
            previous = previous_whitelist_slot(path);
            assert_eq!(previous, Some(S1));
        }

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn finalized_reference_survives_all_error_run_and_advances_when_new_slot_finalizes() {
        let mut errors: Vec<LogEntry> = ["a", "b"].iter().map(|n| entry(n, 0, "", "")).collect();
        for entry in &mut errors {
            entry.error = Some("unreachable".to_string());
        }
        assert_eq!(persisted_snapshot_slot(0, Some(S1), &errors), S1);

        // Once S2 is voted through and a verifier is routable on it,
        // `chosen_slot` takes precedence again and the file moves to S2.
        let s2_entries = vec![entry("a", S2, S2_ROOT, S2_HASH)];
        assert_eq!(persisted_snapshot_slot(S2, Some(S2), &s2_entries), S2);
    }

    #[test]
    fn an_ok_verifier_on_an_older_slot_does_not_drag_the_reference_backwards() {
        // One operator lags behind on S0 while the rest of the fleet has moved
        // to the still-unvoted S2 and pruned S1, so the only routable verifier
        // sits on a slot older than the reference. Recording S0 would throw
        // away S1, which is the newest slot anything can still be judged
        // against, and the next run would fall back to S0 instead.
        const S0: u64 = 440_209_000;
        let entries = vec![
            entry("lagging", S0, S1_ROOT, S1_HASH),
            entry("a", S2, S2_ROOT, S2_HASH),
        ];
        assert_eq!(persisted_snapshot_slot(S0, Some(S1), &entries), S1);

        // A routable verifier ahead of the reference still wins, so the file
        // keeps advancing once a newer slot is voted through.
        assert_eq!(persisted_snapshot_slot(S2, Some(S1), &entries), S2);
    }

    #[test]
    fn first_pending_run_without_any_finalized_reference_uses_freshest_reported_slot() {
        let entries = vec![
            entry("a", S1, S1_ROOT, S1_HASH),
            entry("b", S2, S2_ROOT, S2_HASH),
        ];
        assert_eq!(persisted_snapshot_slot(0, None, &entries), S2);
    }

    #[test]
    fn pending_entries_do_not_survive_as_routable_or_move_the_chosen_slot() {
        // Same steps as the tail of `compare_with_chain`. A `pending` entry is
        // not `ok` and does not raise `chosen_slot`, so the freshest slot that
        // `demote_stale_verifiers` measures against is the voted-through one.
        let ok_on_reference = judge_pending_entry(
            |_| Ok(meta(S1, S1_ROOT, S1_HASH)),
            &entry("a", S2, S2_ROOT, S2_HASH),
            "mainnet",
            Some(S1),
            &states(),
        );
        let pruned = judge_pending_entry(
            |_| Err("returned HTTP 404 Not Found".to_string()),
            &entry("b", S2, S2_ROOT, S2_HASH),
            "mainnet",
            Some(S1),
            &states(),
        );
        let mut verifiers = vec![ok_on_reference, pruned];
        let chosen_slot = verifiers
            .iter()
            .filter(|v| v.status == "ok")
            .map(|v| v.slot)
            .max()
            .unwrap_or(0);
        assert_eq!(chosen_slot, S1);
        demote_stale_verifiers(&mut verifiers, chosen_slot, DEFAULT_MAX_VERIFIER_SLOT_LAG);
        let routable: Vec<&str> = verifiers
            .iter()
            .filter(|v| v.status == "ok")
            .map(|v| v.name.as_str())
            .collect();
        assert_eq!(routable, ["a"]);
    }
}
