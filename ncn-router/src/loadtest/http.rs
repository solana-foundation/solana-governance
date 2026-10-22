use super::{
    config::{BodyFormat, Case, Kind, Target},
    Error, Result,
};
use chrono::Utc;
use reqwest::{Client, Url};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::time::Instant;

const MAX_BODY: usize = 2 * 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct Hop {
    pub url: String,
    pub method: &'static str,
    pub status: Option<u16>,
    pub elapsed_ms: f64,
    pub location: Option<String>,
    pub verifier_header: Option<String>,
    pub cache_status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Record {
    pub id: usize,
    pub phase: String,
    pub started_at: String,
    pub target: String,
    pub kind: Kind,
    pub request: String,
    pub path: String,
    pub url: String,
    pub method: &'static str,
    pub verifier: Option<String>,
    pub hops: Vec<Hop>,
    pub issues: Vec<String>,
    pub status: Option<u16>,
    pub response_sha256: Option<String>,
    pub response_file: Option<String>,
    pub final_url: Option<String>,
    pub elapsed_ms: f64,
    pub ok: bool,
    #[serde(skip)]
    pub body: Option<Vec<u8>>,
}

impl Record {
    fn new(target: &Target, case: &Case, phase: &str) -> Self {
        Self {
            id: 0,
            phase: phase.into(),
            started_at: Utc::now().to_rfc3339(),
            target: target.name.clone(),
            kind: target.kind,
            request: case.name.clone(),
            path: case.path.clone(),
            url: format!("{}{}", target.url, case.path.trim_start_matches('/')),
            method: "GET",
            verifier: (target.kind == Kind::Verifier).then(|| target.name.clone()),
            hops: Vec::new(),
            issues: Vec::new(),
            status: None,
            response_sha256: None,
            response_file: None,
            final_url: None,
            elapsed_ms: 0.0,
            ok: false,
            body: None,
        }
    }
}

async fn read_body(response: &mut reqwest::Response, limit: usize) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if body.len() + chunk.len() > limit {
            return Err(Error::Invalid(format!(
                "response exceeded {} MiB",
                limit / 1024 / 1024
            )));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

pub async fn fetch_vote_accounts(client: &Client, endpoint: &str) -> Result<Record> {
    let endpoint = Url::parse(endpoint)
        .ok()
        .filter(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
        .ok_or_else(|| Error::Invalid("RPC URL must use HTTP(S)".into()))?;
    // RPC credentials can live in the URL path or query; retain only the origin in evidence.
    let target = Target {
        name: "solana_rpc".into(),
        url: format!("{}/", endpoint.origin().ascii_serialization()),
        kind: Kind::Rpc,
    };
    let case = Case {
        name: "discover_vote_accounts".into(),
        path: "/".into(),
        expected_status: 200,
        format: BodyFormat::Json,
        expected_body: None,
        targets: vec![Kind::Rpc],
    };
    let mut record = Record::new(&target, &case, "rpc_discovery");
    record.method = "POST";
    let started = Instant::now();
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getVoteAccounts",
        "params": [{"commitment": "finalized", "keepUnstakedDelinquents": false}],
    });
    let result = async {
        let mut response = client.post(endpoint).json(&payload).send().await?;
        record.status = Some(response.status().as_u16());
        record.body = Some(read_body(&mut response, 16 * 1024 * 1024).await?);
        if record.status != Some(200) {
            return Err(Error::Invalid(format!(
                "RPC returned HTTP {}",
                response.status().as_u16()
            )));
        }
        let value: Value = serde_json::from_slice(record.body.as_ref().unwrap())?;
        if !value["error"].is_null() {
            return Err(Error::Invalid(
                "getVoteAccounts returned a JSON-RPC error".into(),
            ));
        }
        if !value["result"]["current"].is_array() || !value["result"]["delinquent"].is_array() {
            return Err(Error::Invalid(
                "getVoteAccounts did not return vote account arrays".into(),
            ));
        }
        record.response_sha256 = Some(hash(&serde_json::to_vec(&value)?));
        Ok(())
    }
    .await;
    if let Err(error) = result {
        record.issues.push(error.to_string());
    }
    record.elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    record.hops.push(Hop {
        url: record.url.clone(),
        method: "POST",
        status: record.status,
        elapsed_ms: record.elapsed_ms,
        location: None,
        verifier_header: None,
        cache_status: None,
    });
    record.ok = record.issues.is_empty();
    Ok(record)
}

pub fn hash(body: &[u8]) -> String {
    format!("{:x}", Sha256::digest(body))
}

fn identify(url: &Url, verifiers: &[Target]) -> Option<usize> {
    verifiers.iter().position(|verifier| {
        let base = Url::parse(&verifier.url).expect("validated verifier URL");
        base.origin() == url.origin() && url.path().starts_with(base.path())
    })
}

fn same_request(actual: &Url, expected: &Url) -> bool {
    let mut left: Vec<_> = actual.query_pairs().collect();
    let mut right: Vec<_> = expected.query_pairs().collect();
    left.sort();
    right.sort();
    actual.path() == expected.path() && left == right
}

fn validate(value: &Value, url: &Url) -> Result<()> {
    let path = url.path();
    if !matches!(path, "/meta" | "/metas")
        && !path.starts_with("/proof/")
        && !path.starts_with("/voter/")
    {
        return Ok(());
    }
    let values = if path == "/metas" {
        value
            .as_array()
            .filter(|items| items.len() == 1)
            .ok_or_else(|| Error::Invalid("expected metadata for exactly one snapshot".into()))?
            .iter()
            .collect::<Vec<_>>()
    } else {
        vec![value]
    };
    let query = |key: &str| {
        url.query_pairs()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.into_owned())
    };
    for item in values {
        if item.get("network").and_then(Value::as_str) != query("network").as_deref() {
            return Err(Error::Invalid(
                "response does not describe the requested network".into(),
            ));
        }
        let slot_key = if matches!(path, "/meta" | "/metas") {
            "slot"
        } else {
            "snapshot_slot"
        };
        let slot = item
            .get(slot_key)
            .and_then(Value::as_u64)
            .filter(|slot| *slot > 0)
            .ok_or_else(|| Error::Invalid("response is missing a valid snapshot slot".into()))?;
        if let Some(expected) = query("slot").or_else(|| query("slots")) {
            if expected.parse::<u64>().ok() != Some(slot) {
                return Err(Error::Invalid(
                    "response does not describe the requested snapshot".into(),
                ));
            }
        }
        if path.starts_with("/proof/") {
            let prefix = if path.contains("/vote_account/") {
                "meta_merkle"
            } else {
                "stake_merkle"
            };
            if !item
                .get(format!("{prefix}_leaf"))
                .is_some_and(Value::is_object)
                || !item
                    .get(format!("{prefix}_proof"))
                    .is_some_and(Value::is_array)
            {
                return Err(Error::Invalid(
                    "response is missing its Merkle leaf or proof".into(),
                ));
            }
        }
        if path.starts_with("/voter/")
            && (item.get("voting_wallet").and_then(Value::as_str) != path.rsplit('/').next()
                || !["vote_accounts", "stake_accounts"]
                    .iter()
                    .all(|key| item.get(key).is_some_and(Value::is_array)))
        {
            return Err(Error::Invalid(
                "response is missing the requested voter summary".into(),
            ));
        }
    }
    Ok(())
}

/// Return a stable response hash, ignoring only verifier-local metadata upload times.
pub fn response_hash(body: &[u8], case: &Case, validate_status: bool) -> Result<String> {
    if case.format == BodyFormat::Text {
        return Ok(hash(body));
    }
    let mut value: Value = serde_json::from_slice(body)?;
    let url = Url::parse(&format!("https://loadtest.invalid{}", case.path))
        .map_err(|_| Error::Invalid("invalid request path".into()))?;
    if validate_status {
        validate(&value, &url)?;
    }
    if url.path() == "/meta" {
        if let Some(item) = value.as_object_mut() {
            item.remove("created_at");
        }
    } else if url.path() == "/metas" {
        if let Some(items) = value.as_array_mut() {
            for item in items {
                if let Some(item) = item.as_object_mut() {
                    item.remove("created_at");
                }
            }
        }
    }
    value.sort_all_objects();
    Ok(hash(&serde_json::to_vec(&value)?))
}

pub async fn fetch(
    client: &Client,
    target: &Target,
    case: &Case,
    verifiers: &[Target],
    phase: &str,
) -> Record {
    let mut record = Record::new(target, case, phase);
    let started = Instant::now();
    let mut url = Url::parse(&record.url).expect("validated request URL");
    for index in 0..6 {
        let hop_started = Instant::now();
        let mut hop = Hop {
            url: url.to_string(),
            method: "GET",
            status: None,
            elapsed_ms: 0.0,
            location: None,
            verifier_header: None,
            cache_status: None,
        };
        record.status = None;
        let result = async {
            let mut response = client
                .get(url.clone())
                .header("Cache-Control", "no-cache")
                .header("Accept", "application/json")
                .send()
                .await?;
            let status = response.status();
            let header = |name: &str| {
                response
                    .headers()
                    .get(name)
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_owned)
            };
            hop.status = Some(status.as_u16());
            hop.location = header("Location");
            hop.verifier_header = header("X-NCN-Verifier");
            hop.cache_status = header("CF-Cache-Status");
            record.status = hop.status;
            let redirected = matches!(status.as_u16(), 301 | 302 | 303 | 307 | 308);
            if index == 0 && target.kind == Kind::Router {
                let selected = if redirected {
                    hop.location
                        .as_ref()
                        .and_then(|location| url.join(location).ok())
                } else {
                    hop.verifier_header
                        .as_ref()
                        .and_then(|identity| Url::parse(identity).ok())
                };
                let identity = selected
                    .as_ref()
                    .and_then(|selected| identify(selected, verifiers));
                record.verifier = identity.map(|idx| verifiers[idx].name.clone());
                if let Some(idx) = identity {
                    let expected = Url::parse(&format!(
                        "{}{}",
                        verifiers[idx].url,
                        case.path.trim_start_matches('/')
                    ))
                    .expect("validated request URL");
                    if redirected && !same_request(selected.as_ref().unwrap(), &expected) {
                        record
                            .issues
                            .push("router changed the request path or query".into());
                    }
                } else {
                    record
                        .issues
                        .push("router did not identify a configured verifier".into());
                }
            }
            let body = read_body(&mut response, MAX_BODY).await?;
            if redirected {
                let next = hop
                    .location
                    .as_ref()
                    .and_then(|location| url.join(location).ok())
                    .filter(|next| matches!(next.scheme(), "http" | "https"))
                    .ok_or_else(|| {
                        Error::Invalid("missing or invalid HTTP(S) redirect location".into())
                    })?;
                if index == 5 {
                    return Err(Error::Invalid("more than five redirects".into()));
                }
                return Ok(Some(next));
            }
            record.final_url = Some(url.to_string());
            record.body = Some(body);
            let body = record.body.as_ref().unwrap();
            if status.as_u16() != case.expected_status {
                record.issues.push(format!(
                    "expected HTTP {}, got {}",
                    case.expected_status,
                    status.as_u16()
                ));
            } else {
                if case
                    .expected_body
                    .as_ref()
                    .is_some_and(|expected| expected.as_bytes() != body)
                {
                    record
                        .issues
                        .push("response body differs from expected_body".into());
                }
                record.response_sha256 = Some(response_hash(body, case, status.as_u16() == 200)?);
            }
            Ok(None)
        }
        .await;
        hop.elapsed_ms = hop_started.elapsed().as_secs_f64() * 1000.0;
        record.hops.push(hop);
        match result {
            Ok(Some(next)) => url = next,
            Ok(None) => break,
            Err(error) => {
                record.issues.push(error.to_string());
                break;
            }
        }
    }
    record.elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    record.ok = record.issues.is_empty();
    record
}
