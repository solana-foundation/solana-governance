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

    let network = match query.get("network").map(|s| s.as_str()).unwrap_or("mainnet") {
        "mainnet" => "mainnet",
        "testnet" => "testnet",
        other => {
            let body = format!(r#"{{"error":"invalid_network","network":"{}"}}"#, other);
            let _ = request.respond(
                Response::from_string(body)
                    .with_status_code(StatusCode(400))
                    .with_header(json_header()),
            );
            return;
        },
    };

    let (path_prefix, cache) = match network {
        "testnet" => (&state.testnet_path, &state.testnet_cache),
        _ => (&state.mainnet_path, &state.mainnet_cache),
    };

    let snapshot_opt = load_whitelist(path_prefix, cache);
    let snapshot = match snapshot_opt {
        Some(s) => s,
        None => {
            let body = format!(
                r#"{{"error":"no_whitelist_data","network":"{}"}}"#,
                network
            );
            let _ = request.respond(
                Response::from_string(body)
                    .with_status_code(StatusCode(503))
                    .with_header(json_header()),
            );
            return;
        }
    };

    let mut rng = rand::thread_rng();
    let ok_verifiers: Vec<&WhitelistVerifier> = snapshot
        .verifiers
        .iter()
        .filter(|v| v.status == "ok")
        .collect();

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

    let mut target = chosen.domain.clone();
    if !target.ends_with('/') {
        target.push('/');
    }

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
        let response = Response::empty(302).with_header(
            Header::from_bytes(&b"Location"[..], target.as_bytes()).expect("header"),
        );
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

fn split_path_and_query(url: &str) -> (&str, std::collections::HashMap<String, String>) {
    let mut parts = url.splitn(2, '?');
    let path = parts.next().unwrap_or("/");
    let mut query_map = std::collections::HashMap::new();
    if let Some(qs) = parts.next() {
        for pair in qs.split('&') {
            if pair.is_empty() {
                continue;
            }
            let mut kv = pair.splitn(2, '=');
            let k = kv.next().unwrap_or("").to_string();
            let v = kv.next().unwrap_or("").to_string();
            if !k.is_empty() {
                query_map.insert(k, v);
            }
        }
    }
    (path, query_map)
}

fn encode_query(query: &std::collections::HashMap<String, String>) -> String {
    let mut pairs: Vec<String> = Vec::new();
    for (k, v) in query {
        if v.is_empty() {
            pairs.push(k.clone());
        } else {
            pairs.push(format!("{}={}", k, v));
        }
    }
    pairs.join("&")
}

fn load_whitelist(
    path: &str,
    cache_lock: &RwLock<CachedWhitelist>,
) -> Option<WhitelistSnapshot> {
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