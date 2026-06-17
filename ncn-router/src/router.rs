use rand::seq::SliceRandom;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::env;
use std::fs;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

use tiny_http::{Header, Method, Response, Server, StatusCode};

#[derive(Debug, Deserialize, Clone)]
struct WhitelistVerifier {
    name: String,
    domain: String,
    status: String,
    reason: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct WhitelistSnapshot {
    network: String,
    slot: u64,
    updated_at: String,
    verifiers: Vec<WhitelistVerifier>,
}

#[derive(Debug)]
struct CachedWhitelist {
    snapshot: Option<WhitelistSnapshot>,
    mtime: Option<SystemTime>,
}

impl CachedWhitelist {
    fn new() -> Self {
        Self {
            snapshot: None,
            mtime: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RouterMode {
    Redirect,
    Proxy,
}

fn router_mode_from_env() -> RouterMode {
    match env::var("NCN_ROUTER_MODE")
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Ok("proxy") => RouterMode::Proxy,
        _ => RouterMode::Redirect,
    }
}

fn proxy_timeout() -> Duration {
    let secs: u64 = env::var("NCN_ROUTER_PROXY_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
    Duration::from_secs(secs.max(1))
}

fn main() {
    let _ = dotenvy::dotenv();

    let bind_addr = env::var("NCN_ROUTER_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    let mainnet_path = env::var("NCN_WHITELIST_MAINNET_PATH")
        .unwrap_or_else(|_| "ncn_whitelist.mainnet.json".to_string());
    let testnet_path = env::var("NCN_WHITELIST_TESTNET_PATH")
        .unwrap_or_else(|_| "ncn_whitelist.testnet.json".to_string());

    let mode = router_mode_from_env();
    let http_client = Client::builder()
        .timeout(proxy_timeout())
        .build()
        .expect("reqwest client");

    let state = Arc::new(RouterState {
        mainnet_path,
        testnet_path,
        mainnet_cache: RwLock::new(CachedWhitelist::new()),
        testnet_cache: RwLock::new(CachedWhitelist::new()),
        mode,
        http_client,
    });

    let server =
        Server::http(&bind_addr).expect("failed to bind router address / create tiny_http server");
    eprintln!(
        "[ncn-router] Listening on {} (mode={})",
        bind_addr,
        if mode == RouterMode::Proxy {
            "proxy"
        } else {
            "redirect"
        }
    );

    for request in server.incoming_requests() {
        let state = Arc::clone(&state);
        std::thread::spawn(move || handle_request(state, request));
    }
}

struct RouterState {
    mainnet_path: String,
    testnet_path: String,
    mainnet_cache: RwLock<CachedWhitelist>,
    testnet_cache: RwLock<CachedWhitelist>,
    mode: RouterMode,
    http_client: Client,
}

fn handle_request(state: Arc<RouterState>, request: tiny_http::Request) {
    if request.method() != &Method::Get {
        let _ = request.respond(Response::from_string("Method Not Allowed").with_status_code(405));
        return;
    }

    let url = request.url().to_string();
    let (path, query) = split_path_and_query(&url);

    let network = match select_network(&query) {
        NetworkSelection::Supported(n) => n,
        NetworkSelection::Duplicate => {
            let body = serde_json::json!({ "error": "duplicate_network" }).to_string();
            let _ = request.respond(
                Response::from_string(body)
                    .with_status_code(StatusCode(400))
                    .with_header(json_header()),
            );
            return;
        }
        NetworkSelection::Invalid(other) => {
            let body =
                serde_json::json!({ "error": "invalid_network", "network": other }).to_string();
            let _ = request.respond(
                Response::from_string(body)
                    .with_status_code(StatusCode(400))
                    .with_header(json_header()),
            );
            return;
        }
    };

    let (path_prefix, cache) = match network {
        "testnet" => (&state.testnet_path, &state.testnet_cache),
        _ => (&state.mainnet_path, &state.mainnet_cache),
    };

    let snapshot_opt = load_whitelist(path_prefix, cache);
    let snapshot = match snapshot_opt {
        Some(s) => s,
        None => {
            let body = format!(r#"{{"error":"no_whitelist_data","network":"{}"}}"#, network);
            let _ = request.respond(
                Response::from_string(body)
                    .with_status_code(StatusCode(503))
                    .with_header(json_header()),
            );
            return;
        }
    };

    // Defense in depth: the snapshot we loaded must actually describe the
    // network we selected. A mislabeled or misconfigured whitelist file would
    // otherwise let us serve traffic against the wrong trust boundary.
    if snapshot.network != network {
        eprintln!(
            "[ncn-router] whitelist network mismatch: selected={}, snapshot={}",
            network, snapshot.network
        );
        let body = serde_json::json!({
            "error": "whitelist_network_mismatch",
            "selected": network,
            "snapshot": snapshot.network,
        })
        .to_string();
        let _ = request.respond(
            Response::from_string(body)
                .with_status_code(StatusCode(500))
                .with_header(json_header()),
        );
        return;
    }

    let mut rng = rand::thread_rng();
    let ok_verifiers = select_routable_verifiers(&snapshot.verifiers);

    if ok_verifiers.is_empty() {
        let body = format!(
            r#"{{"error":"no_ok_verifiers","network":"{}","slot":{}}}"#,
            snapshot.network, snapshot.slot
        );
        let _ = request.respond(
            Response::from_string(body)
                .with_status_code(StatusCode(503))
                .with_header(json_header()),
        );
        return;
    }

    let chosen = ok_verifiers.choose(&mut rng).unwrap();

    // Build the upstream base from the same canonical form used for de-duplication,
    // so the routing identity is consistent end to end.
    let mut target = canonical_domain(&chosen.domain);

    let relative_path = path.trim_start_matches('/');
    target.push_str(relative_path);

    if !query.is_empty() {
        let qs = encode_query(&query);
        if !qs.is_empty() {
            target.push('?');
            target.push_str(&qs);
        }
    }

    eprintln!(
        "[ncn-router] {} -> {} (network={}, slot={})",
        url, target, snapshot.network, snapshot.slot
    );

    if state.mode == RouterMode::Proxy {
        respond_proxy(&state.http_client, request, &target);
    } else {
        let response = Response::empty(302)
            .with_header(Header::from_bytes(&b"Location"[..], target.as_bytes()).expect("header"));
        let _ = request.respond(response);
    }
}

fn respond_proxy(client: &Client, request: tiny_http::Request, target: &str) {
    match client.get(target).send() {
        Ok(upstream) => {
            let code = upstream.status().as_u16();
            let content_type = upstream
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            let body = match upstream.bytes() {
                Ok(b) => b.to_vec(),
                Err(e) => {
                    eprintln!("[ncn-router] proxy read body failed: {}", e);
                    let _ = request.respond(
                        Response::from_string(
                            r#"{"error":"proxy_upstream_body_failed"}"#.to_string(),
                        )
                        .with_status_code(StatusCode(502))
                        .with_header(json_header()),
                    );
                    return;
                }
            };

            let mut response = Response::from_data(body).with_status_code(StatusCode(code));
            if let Some(ct) = content_type {
                if let Ok(h) = Header::from_bytes(&b"Content-Type"[..], ct.as_bytes()) {
                    response = response.with_header(h);
                }
            }
            let _ = request.respond(response);
        }
        Err(e) => {
            eprintln!("[ncn-router] proxy request failed: {}", e);
            let body = serde_json::json!({ "error": "proxy_upstream_failed" }).to_string();
            let _ = request.respond(
                Response::from_string(body)
                    .with_status_code(StatusCode(502))
                    .with_header(json_header()),
            );
        }
    }
}

/// The outcome of resolving the effective network from the decoded query.
#[derive(Debug, PartialEq, Eq)]
enum NetworkSelection {
    /// A supported network that has a configured whitelist snapshot.
    Supported(&'static str),
    /// The `network` parameter appeared more than once.
    Duplicate,
    /// The `network` parameter was present but not a supported value.
    Invalid(String),
}

/// Split a request URL into its path and the list of percent-decoded query
/// parameters.
///
/// The query is parsed with the `application/x-www-form-urlencoded` rules used
/// by the downstream Axum verifiers (`form_urlencoded`), so the router and the
/// verifier agree on the canonical meaning of every parameter. This closes the
/// parser differential that let percent-encoding tricks such as
/// `?%6eetwork=testnet` or `?network=%74estnet` be read as `network=testnet`
/// downstream while the router's naive byte matching fell back to mainnet.
///
/// The pairs are kept in their original order (rather than a map) so duplicate
/// `network` parameters can be detected and the forwarded query can be
/// re-serialized deterministically.
fn split_path_and_query(url: &str) -> (&str, Vec<(String, String)>) {
    let mut parts = url.splitn(2, '?');
    let path = parts.next().unwrap_or("/");
    let pairs = match parts.next() {
        Some(qs) => form_urlencoded::parse(qs.as_bytes()).into_owned().collect(),
        None => Vec::new(),
    };
    (path, pairs)
}

/// Canonical routing identity for a verifier origin. The router redirects to
/// `domain` after ensuring a trailing slash, so `http://x` and `http://x/`
/// resolve to the same upstream. Using this as the de-dup key (and to build the
/// redirect target) keeps the routing identity consistent and prevents slash
/// variants from surviving as separate tickets that collapse to one upstream.
fn canonical_domain(domain: &str) -> String {
    if domain.ends_with('/') {
        domain.to_string()
    } else {
        format!("{}/", domain)
    }
}

/// Select the verifiers eligible for routing: `status == "ok"`, de-duplicated by
/// canonical domain. The whitelist is sampled uniformly, so a single origin
/// appearing in multiple rows would otherwise be sampled as extra routing
/// tickets. Keeping one row per canonical origin enforces "one ticket per
/// origin" at the sampling site, independent of how the whitelist file was
/// produced and regardless of trailing-slash variants.
fn select_routable_verifiers(verifiers: &[WhitelistVerifier]) -> Vec<&WhitelistVerifier> {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    verifiers
        .iter()
        .filter(|v| v.status == "ok")
        .filter(|v| seen.insert(canonical_domain(&v.domain)))
        .collect()
}

/// Derive the effective network from the canonical (percent-decoded) query
/// parameters.
///
/// Only `mainnet` and `testnet` have whitelist snapshots, so any other value is
/// rejected rather than silently falling back to mainnet. A missing `network`
/// parameter defaults to mainnet (matching the verifier's `DEFAULT_NETWORK`),
/// and a duplicated `network` parameter is rejected outright.
fn select_network(query: &[(String, String)]) -> NetworkSelection {
    let mut value: Option<&str> = None;
    for (k, v) in query {
        if k == "network" {
            if value.is_some() {
                return NetworkSelection::Duplicate;
            }
            value = Some(v.as_str());
        }
    }
    match value.unwrap_or("mainnet") {
        "mainnet" => NetworkSelection::Supported("mainnet"),
        "testnet" => NetworkSelection::Supported("testnet"),
        other => NetworkSelection::Invalid(other.to_string()),
    }
}

/// Re-serialize the decoded query parameters into a canonical
/// `application/x-www-form-urlencoded` string.
///
/// Forwarding the canonical form (rather than the raw inbound bytes) guarantees
/// the downstream verifier decodes exactly the parameters the router used to
/// pick the whitelist, so the selected trust boundary and the serviced request
/// can never disagree.
fn encode_query(query: &[(String, String)]) -> String {
    let mut serializer = form_urlencoded::Serializer::new(String::new());
    for (k, v) in query {
        serializer.append_pair(k, v);
    }
    serializer.finish()
}

fn load_whitelist(path: &str, cache_lock: &RwLock<CachedWhitelist>) -> Option<WhitelistSnapshot> {
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return None,
    };
    let mtime = metadata.modified().ok();

    {
        let cache = cache_lock.read().unwrap();
        if cache.mtime == mtime {
            return cache.snapshot.clone();
        }
    }

    let contents = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[ncn-router] Failed to read whitelist {}: {}", path, e);
            return None;
        }
    };

    let snapshot: WhitelistSnapshot = match serde_json::from_str(&contents) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[ncn-router] Failed to parse whitelist {}: {}", path, e);
            return None;
        }
    };

    {
        let mut cache = cache_lock.write().unwrap();
        cache.snapshot = Some(snapshot.clone());
        cache.mtime = mtime;
    }

    Some(snapshot)
}

fn json_header() -> Header {
    Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).expect("header")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verifier(name: &str, domain: &str, status: &str) -> WhitelistVerifier {
        WhitelistVerifier {
            name: name.to_string(),
            domain: domain.to_string(),
            status: status.to_string(),
            reason: None,
        }
    }

    fn selection(url: &str) -> NetworkSelection {
        let (_, query) = split_path_and_query(url);
        select_network(&query)
    }

    fn canonical_query(url: &str) -> String {
        let (_, query) = split_path_and_query(url);
        encode_query(&query)
    }

    // --- Regression: legitimate, previously-accepted traffic still works. ---

    #[test]
    fn plain_networks_select_their_whitelist() {
        assert_eq!(
            selection("/meta?network=testnet"),
            NetworkSelection::Supported("testnet")
        );
        assert_eq!(
            selection("/meta?network=mainnet"),
            NetworkSelection::Supported("mainnet")
        );
    }

    #[test]
    fn missing_network_defaults_to_mainnet() {
        assert_eq!(selection("/meta"), NetworkSelection::Supported("mainnet"));
        assert_eq!(
            selection("/meta?slot=10"),
            NetworkSelection::Supported("mainnet")
        );
    }

    // --- Security: the percent-encoding bypass variants are decoded first. ---

    #[test]
    fn percent_encoded_value_is_decoded_before_selection() {
        // `%74estnet` decodes to `testnet`; it must NOT fall back to mainnet.
        assert_eq!(
            selection("/meta?network=%74estnet"),
            NetworkSelection::Supported("testnet")
        );
    }

    #[test]
    fn percent_encoded_key_is_decoded_before_selection() {
        // `%6eetwork` decodes to `network` (the PoC payload).
        assert_eq!(
            selection("/meta?%6eetwork=testnet"),
            NetworkSelection::Supported("testnet")
        );
    }

    #[test]
    fn fully_encoded_pair_is_decoded() {
        assert_eq!(
            selection("/meta?%6eetwork=%74estnet"),
            NetworkSelection::Supported("testnet")
        );
    }

    // --- Negative cases: malformed / duplicated / unsupported are rejected. ---

    #[test]
    fn duplicate_network_is_rejected() {
        assert_eq!(
            selection("/meta?network=mainnet&network=testnet"),
            NetworkSelection::Duplicate
        );
        // Duplicates that hide behind percent-encoding are still caught,
        // because detection happens on the decoded keys.
        assert_eq!(
            selection("/meta?network=mainnet&%6eetwork=testnet"),
            NetworkSelection::Duplicate
        );
    }

    #[test]
    fn unsupported_network_is_rejected() {
        // devnet has no whitelist snapshot, so it must not be routed to mainnet.
        assert_eq!(
            selection("/meta?network=devnet"),
            NetworkSelection::Invalid("devnet".to_string())
        );
        assert_eq!(
            selection("/meta?network=evil"),
            NetworkSelection::Invalid("evil".to_string())
        );
    }

    // --- The forwarded query is the canonical representation. ---

    #[test]
    fn forwarded_query_is_canonicalized() {
        // Both bypass variants forward the canonical `network=testnet`, so the
        // downstream verifier sees exactly what the router whitelisted.
        assert_eq!(
            canonical_query("/meta?%6eetwork=testnet"),
            "network=testnet"
        );
        assert_eq!(
            canonical_query("/meta?network=%74estnet"),
            "network=testnet"
        );
    }

    #[test]
    fn canonicalization_preserves_other_params() {
        assert_eq!(
            canonical_query("/voter/abc?network=testnet&slot=42"),
            "network=testnet&slot=42"
        );
    }

    #[test]
    fn path_is_preserved() {
        let (path, _) = split_path_and_query("/proof/vote_account/xyz?network=mainnet");
        assert_eq!(path, "/proof/vote_account/xyz");
    }

    #[test]
    fn routable_verifiers_dedupe_duplicate_domains() {
        // A single origin listed twice as "ok" must yield one routing ticket.
        let verifiers = vec![
            verifier("malicious", "http://evil", "ok"),
            verifier("malicious-dup", "http://evil", "ok"),
            verifier("honest", "http://good", "ok"),
        ];
        let routable = select_routable_verifiers(&verifiers);
        assert_eq!(routable.len(), 2);
        assert_eq!(
            routable
                .iter()
                .filter(|v| v.domain == "http://evil")
                .count(),
            1
        );
        let domains: Vec<&str> = routable.iter().map(|v| v.domain.as_str()).collect();
        assert!(domains.contains(&"http://evil"));
        assert!(domains.contains(&"http://good"));
    }

    #[test]
    fn routable_verifiers_exclude_non_ok() {
        let verifiers = vec![
            verifier("a", "http://a", "ok"),
            verifier("b", "http://b", "error"),
            verifier("c", "http://c", "mismatch"),
        ];
        let routable = select_routable_verifiers(&verifiers);
        assert_eq!(routable.len(), 1);
        assert_eq!(routable[0].domain, "http://a");
    }

    #[test]
    fn routable_verifiers_dedupe_trailing_slash_variants() {
        // `http://evil` and `http://evil/` redirect to the same upstream, so they
        // must not survive as two separate routing tickets.
        let verifiers = vec![
            verifier("evil", "http://evil", "ok"),
            verifier("evil-slash", "http://evil/", "ok"),
            verifier("honest", "http://good/", "ok"),
        ];
        let routable = select_routable_verifiers(&verifiers);
        assert_eq!(routable.len(), 2);
        assert_eq!(
            routable
                .iter()
                .filter(|v| canonical_domain(&v.domain) == "http://evil/")
                .count(),
            1
        );
    }
}
