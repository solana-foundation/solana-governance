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

#[derive(Debug, Deserialize, serde::Serialize)]
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

#[derive(Debug, serde::Serialize)]
struct WhitelistVerifier {
    name: String,
    domain: String,
    status: String,
    reason: Option<String>,
}

#[derive(Debug, serde::Serialize)]
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

    for entry in &entries {
        if !verification_domain_is_https(&entry.domain) {
            return Err(format!(
                "verifier '{}' domain '{}' is not https; refusing to generate whitelist over plaintext transport",
                entry.name, entry.domain
            )
            .into());
        }
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

    println!("network | name | meta_merkle_root | snapshot_hash | (domain)");

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
                status: "error".to_string(),
                reason: entry.error.clone(),
            });
            continue;
        }

        match fetch_winning_ballot_cron(&client, &program_id, entry.slot) {
            Ok(ballot) => {
                let onchain_merkle_root = bytes32_base58(&ballot.meta_merkle_root);
                let onchain_snapshot_hash = bytes32_base58(&ballot.snapshot_hash);

                let merkle_match = onchain_merkle_root == entry.merkle_root;
                let snapshot_match = onchain_snapshot_hash == entry.snapshot_hash;

                println!(
                    "{} | {} | meta_merkle_root ({}) | snapshot_hash ({}) | ({})",
                    network,
                    entry.name,
                    if merkle_match { "matched" } else { "mismatch" },
                    if snapshot_match { "matched" } else { "mismatch" },
                    entry.domain
                );

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

                if status == "ok" && entry.slot > chosen_slot {
                    chosen_slot = entry.slot;
                }

                whitelist_verifiers.push(WhitelistVerifier {
                    name: entry.name.clone(),
                    domain: entry.domain.clone(),
                    status,
                    reason,
                });
            }
            Err(e) => {
                println!(
                    "{} | {} | meta_merkle_root (fetch_failed) | snapshot_hash (fetch_failed) | ({})",
                    network, entry.name, entry.domain
                );
                eprintln!(
                    "[ncn-meta-cron] fetch failed for {} (slot {}): {}",
                    entry.name, entry.slot, e
                );
                whitelist_verifiers.push(WhitelistVerifier {
                    name: entry.name.clone(),
                    domain: entry.domain.clone(),
                    status: "error".to_string(),
                    reason: Some(e),
                });
            }
        }
    }

    // Build and write whitelist snapshot for this network.
    let snapshot_slot = if chosen_slot != 0 {
        chosen_slot
    } else {
        // Fallback to max slot seen in log entries (even if none were fully ok).
        entries
            .iter()
            .map(|e| e.slot)
            .max()
            .unwrap_or_default()
    };

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

    let whitelist_path = match network {
        "testnet" => env::var("NCN_WHITELIST_TESTNET_PATH")
            .unwrap_or_else(|_| "ncn_whitelist.testnet.json".to_string()),
        _ => env::var("NCN_WHITELIST_MAINNET_PATH")
            .unwrap_or_else(|_| "ncn_whitelist.mainnet.json".to_string()),
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

/// Keep one canonical whitelist record per verifier origin (`domain`).
///
/// When a domain appears more than once (e.g. it is listed twice in the config),
/// an `ok` record is preferred over a non-`ok` one so a transient failure on one
/// poll cannot shadow a successful poll for the same origin. Among records of the
/// same rank the first occurrence wins, which keeps the output deterministic in
/// config order. This still collapses each origin to a single row, so a verifier
/// can never hold extra routing tickets — it just avoids demoting an origin that
/// did verify successfully. Mirrors the `status == "ok"`-first selection in
/// `ncn-router`'s `select_routable_verifiers`.
fn dedupe_verifiers_by_domain(verifiers: Vec<WhitelistVerifier>) -> Vec<WhitelistVerifier> {
    let mut index_by_domain: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut deduped: Vec<WhitelistVerifier> = Vec::new();
    for verifier in verifiers {
        match index_by_domain.get(&verifier.domain) {
            None => {
                index_by_domain.insert(verifier.domain.clone(), deduped.len());
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

fn fetch_winning_ballot_cron(
    client: &RpcClient,
    program_id: &Pubkey,
    snapshot_slot: u64,
) -> Result<Ballot, String> {
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
            parse_winning_ballot(rest)
        }
        Err(e) => Err(format!(
            "Failed to fetch BallotBox account: {}",
            e.to_string()
        )),
    }
}

fn parse_winning_ballot(mut data: &[u8]) -> Result<Ballot, String> {
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
    read_u64(&mut data)?;
    read_u16(&mut data)?;

    Ballot::deserialize(&mut data).map_err(|e| e.to_string())
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
    for verifier in &config.verifiers {
        if !verification_domain_is_https(&verifier.verification_domain) {
            return Err(format!(
                "verifier '{}' verification_domain '{}' must use https; plaintext transport is not trusted",
                verifier.name, verifier.verification_domain
            )
            .into());
        }
    }
    Ok(config)
}

/// Verifier trust depends on an authenticated transport, so only https origins
/// are accepted; unparsable or non-https domains are treated as untrusted.
fn verification_domain_is_https(domain: &str) -> bool {
    reqwest::Url::parse(domain.trim())
        .map(|url| url.scheme() == "https")
        .unwrap_or(false)
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
        WhitelistVerifier {
            name: name.to_string(),
            domain: domain.to_string(),
            status: status.to_string(),
            reason: None,
        }
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
}
