use super::{Error, Result};
use clap::{Parser, ValueEnum};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::PathBuf, time::Duration};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Router,
    Verifier,
    Rpc,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BodyFormat {
    Json,
    Text,
}

#[derive(Clone, Copy, Debug, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Testnet,
}

impl Network {
    pub fn rpc_url(self) -> &'static str {
        match self {
            Self::Mainnet => "https://api.mainnet-beta.solana.com",
            Self::Testnet => "https://api.testnet.solana.com",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mainnet => "mainnet",
            Self::Testnet => "testnet",
        }
    }
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .into()
}

fn positive(value: &str) -> std::result::Result<f64, String> {
    let number: f64 = value.parse().map_err(|_| "must be a positive number")?;
    if !number.is_finite() || number <= 0.0 {
        return Err("must be a finite positive number".into());
    }
    Ok(number)
}

fn positive_usize(value: &str) -> std::result::Result<usize, String> {
    value
        .parse()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "must be a positive integer".into())
}

fn probability(value: &str) -> std::result::Result<f64, String> {
    let number = positive(value)?;
    if number >= 1.0 {
        return Err("must be greater than zero and less than one".into());
    }
    Ok(number)
}

#[derive(Clone, Debug, Parser, Serialize)]
#[command(about = "Load the NCN router and each verifier, writing request evidence and reports")]
pub struct Args {
    /// Router base URL to exercise.
    #[arg(long, default_value = "https://ncn-governance.solana.com")]
    pub router_url: String,
    /// TOML file containing the expected verifier names and base URLs.
    #[arg(long, default_value_os_t = root().join("ncn-router/config.toml"))]
    pub verifiers: PathBuf,
    /// TOML request corpus with network and slot placeholders.
    #[arg(long, default_value_os_t = root().join("ncn-router/loadtest/requests.toml"))]
    pub requests: PathBuf,
    /// Network namespace used in requests and response validation.
    #[arg(long, value_enum, default_value = "mainnet")]
    pub network: Network,
    /// Solana RPC for latest account discovery; defaults to the selected network's public RPC.
    #[arg(long, env = "NCN_LOADTEST_RPC_URL", hide_env_values = true)]
    #[serde(skip_serializing)]
    pub rpc_url: Option<String>,
    /// Snapshot slot; otherwise discover the most common latest slot.
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
    pub slot: Option<u64>,
    /// Load phase seconds, excluding discovery, additional router sampling, and drain.
    #[arg(long, default_value = "90", value_parser = positive)]
    pub duration: f64,
    /// Total logical requests per second across router and direct probes.
    #[arg(long, default_value = "2", value_parser = positive)]
    pub rps: f64,
    /// Maximum logical requests in flight.
    #[arg(long, default_value = "8", value_parser = positive_usize)]
    pub concurrency: usize,
    /// Target probability of selecting every verifier with independent, uniform routing.
    #[arg(long, default_value = "0.999", value_parser = probability)]
    pub router_confidence: f64,
    /// Deadline in seconds for each HTTP hop, including reading its body.
    #[arg(long, default_value = "10", value_parser = positive)]
    pub timeout: f64,
    /// Relative p50 excess required to flag a verifier.
    #[arg(long, default_value = "3", value_parser = positive)]
    pub slow_factor: f64,
    /// Minimum p50 excess above the peer median, in milliseconds.
    #[arg(long, default_value = "250", value_parser = positive)]
    pub slow_ms: f64,
    /// Successful samples per verifier and case for latency comparisons.
    #[arg(long, default_value = "3", value_parser = positive_usize)]
    pub min_samples: usize,
    /// Destination for the summary, JSON report, journal, and response bodies.
    #[arg(long, default_value_os_t = root().join("loadtest-results"))]
    pub output: PathBuf,
}

pub fn seconds(value: f64) -> Result<Duration> {
    Duration::try_from_secs_f64(value)
        .ok()
        .filter(|duration| !duration.is_zero())
        .ok_or_else(|| Error::Invalid("duration or rate is outside the supported range".into()))
}

#[derive(Clone, Debug, Serialize)]
pub struct Target {
    pub name: String,
    pub url: String,
    pub kind: Kind,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Case {
    pub name: String,
    pub path: String,
    #[serde(default = "expected_status")]
    pub expected_status: u16,
    #[serde(default = "json_format")]
    pub format: BodyFormat,
    pub expected_body: Option<String>,
    #[serde(default = "all_targets")]
    pub targets: Vec<Kind>,
}

fn expected_status() -> u16 {
    200
}
fn json_format() -> BodyFormat {
    BodyFormat::Json
}
fn all_targets() -> Vec<Kind> {
    vec![Kind::Router, Kind::Verifier]
}

pub fn base_url(value: &str) -> Result<String> {
    let mut url = Url::parse(value).map_err(|_| Error::Invalid("invalid base URL".into()))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(Error::Invalid(
            "base URLs must use HTTP(S) without credentials, query or fragment".into(),
        ));
    }
    url.set_path(&format!("{}/", url.path().trim_end_matches('/')));
    Ok(url.into())
}

pub fn load(args: &Args) -> Result<(Vec<Target>, Vec<Case>)> {
    #[derive(Deserialize)]
    struct Verifier {
        name: String,
        verification_domain: String,
    }
    #[derive(Deserialize)]
    struct Verifiers {
        verifiers: Vec<Verifier>,
    }
    #[derive(Deserialize)]
    struct Requests {
        requests: Vec<Case>,
    }

    seconds(args.duration)?;
    seconds(args.timeout)?;
    seconds(1.0 / args.rps)?;
    let config: Verifiers = toml::from_str(&fs::read_to_string(&args.verifiers)?)?;
    let mut urls = HashSet::new();
    let mut names = HashSet::from(["router".to_owned()]);
    let mut verifiers = Vec::new();
    for verifier in config.verifiers {
        let url = base_url(&verifier.verification_domain)?;
        if !urls.insert(url.clone()) {
            continue;
        }
        if verifier.name.is_empty() || !names.insert(verifier.name.clone()) {
            return Err(Error::Invalid(
                "verifier names must be distinct and cannot be 'router'".into(),
            ));
        }
        verifiers.push(Target {
            name: verifier.name,
            url,
            kind: Kind::Verifier,
        });
    }
    let corpus: Requests = toml::from_str(&fs::read_to_string(&args.requests)?)?;
    names.clear();
    if verifiers.is_empty() || corpus.requests.is_empty() {
        return Err(Error::Invalid(
            "configure at least one verifier and request".into(),
        ));
    }
    for case in &corpus.requests {
        if case.name.is_empty()
            || !names.insert(case.name.clone())
            || case.targets.is_empty()
            || case.targets.contains(&Kind::Rpc)
            || !(100..=599).contains(&case.expected_status)
        {
            return Err(Error::Invalid(
                "requests need unique names, targets, and a valid expected_status".into(),
            ));
        }
        if case.path == "latest"
            && (!matches!(case.name.as_str(), "vote_proof" | "voter")
                || case.expected_status != 200
                || case.format != BodyFormat::Json)
        {
            return Err(Error::Invalid(
                "path = 'latest' requires a vote_proof or voter JSON request expecting HTTP 200"
                    .into(),
            ));
        }
    }
    Ok((verifiers, corpus.requests))
}

pub fn render(cases: &mut [Case], network: &str, slot: u64) -> Result<()> {
    for case in cases {
        case.path = case
            .path
            .replace("{network}", network)
            .replace("{slot}", &slot.to_string());
        let url = Url::parse(&format!("https://loadtest.invalid{}", case.path))
            .map_err(|_| Error::Invalid("invalid request path".into()))?;
        if !case.path.starts_with('/')
            || case.path.starts_with("//")
            || url.fragment().is_some()
            || case.path.contains(['{', '}'])
        {
            return Err(Error::Invalid(
                "request paths must be relative paths with supported placeholders".into(),
            ));
        }
        let pairs: Vec<_> = url.query_pairs().collect();
        let matches = |key: &str, value: &str| {
            pairs.iter().filter(|(k, _)| k == key).count() == 1
                && pairs.iter().any(|(k, v)| k == key && v == value)
        };
        let key = if url.path() == "/metas" {
            "slots"
        } else {
            "slot"
        };
        if url.path() != "/healthz"
            && (!matches("network", network) || !matches(key, &slot.to_string()))
        {
            return Err(Error::Invalid(
                "data requests must use the selected network and pinned slot".into(),
            ));
        }
    }
    Ok(())
}
