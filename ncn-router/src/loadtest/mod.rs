mod config;
mod http;
mod report;
mod resolve;
mod sampling;

pub use config::Args;
use config::{BodyFormat, Case, Kind, Target};
use http::fetch;
use report::{Journal, Report};
use reqwest::Client;
use serde_json::json;
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
use tokio::{task::JoinSet, time::sleep};
use tokio_util::sync::CancellationToken;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("report I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid TOML: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("HTTP request failed: {0}")]
    Http(reqwest::Error),
    #[error("{0}")]
    Invalid(String),
    #[error("request worker failed: {0}")]
    Worker(#[from] tokio::task::JoinError),
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            Self::Invalid("timeout".into())
        } else {
            Self::Http(error.without_url())
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

pub async fn run(args: Args) -> Result<bool> {
    let mut journal = Journal::new(&args)?;
    let mut report = Report::new(args.clone());
    let mut targets = Vec::new();
    let mut cases = Vec::new();
    let cancellation = CancellationToken::new();
    let result = tokio::select! {
        result = prepare(&args) => result,
        _ = tokio::signal::ctrl_c() => Err(Error::Invalid("interrupted during setup".into())),
    };
    let result = match result {
        Ok((client, verifiers, mut corpus)) => {
            let slot = if let Some(slot) = args.slot {
                Ok(slot)
            } else {
                tokio::select! {
                    result = discover(&args, &client, &verifiers, &mut journal) => result,
                    _ = tokio::signal::ctrl_c() => Err(Error::Invalid("interrupted during discovery".into())),
                }
            };
            let resolved = match slot {
                Ok(slot) => {
                    report.slot = Some(slot);
                    tokio::select! {
                        result = resolve::resolve(&args, &client, &verifiers, &mut corpus, slot, &mut journal) => result,
                        _ = tokio::signal::ctrl_c() => Err(Error::Invalid("interrupted during account discovery".into())),
                    }
                }
                Err(error) => Err(error),
            };
            match resolved {
                Ok(()) => {
                    report.workload = corpus.clone();
                    report.verifiers = verifiers.clone();
                    targets.push(Target {
                        name: "router".into(),
                        url: config::base_url(&args.router_url)?,
                        kind: Kind::Router,
                    });
                    targets.extend(verifiers.iter().cloned());
                    cases = corpus;
                    tracing::info!(
                        slot = report.slot,
                        duration = args.duration,
                        rps = args.rps,
                        "Starting load phase"
                    );
                    let result = load(
                        &args,
                        &client,
                        &targets,
                        &cases,
                        &mut journal,
                        &cancellation,
                    )
                    .await;
                    match result {
                        Ok(elapsed) => {
                            report.elapsed_seconds = elapsed;
                            if cancellation.is_cancelled() {
                                Ok(())
                            } else {
                                sample_router(
                                    &args,
                                    &client,
                                    &targets,
                                    report.slot.unwrap(),
                                    &mut journal,
                                    &cancellation,
                                )
                                .await
                                .map(|elapsed| report.router_sampling_seconds = elapsed)
                            }
                        }
                        Err(error) => Err(error),
                    }
                }
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    };
    if let Err(error) = result {
        report
            .findings
            .push(json!({"type":"setup_failed", "message":error.to_string()}));
    }
    if cancellation.is_cancelled() {
        report.findings.push(
            json!({"type":"interrupted", "message":"issuance stopped; in-flight requests drained"}),
        );
    }
    report.analyze(&journal.records, &targets, &cases);
    report.write(&journal.records)?;
    Ok(report.passed)
}

async fn prepare(args: &Args) -> Result<(Client, Vec<Target>, Vec<Case>)> {
    let (verifiers, cases) = config::load(args)?;
    config::base_url(&args.router_url)?;
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(config::seconds(args.timeout)?)
        .user_agent("ncn-network-loadtest/1")
        .build()?;
    Ok((client, verifiers, cases))
}

async fn discover(
    args: &Args,
    client: &Client,
    verifiers: &[Target],
    journal: &mut Journal,
) -> Result<u64> {
    let case = Case {
        name: "discover_snapshot".into(),
        path: format!("/meta?network={}", args.network.as_str()),
        expected_status: 200,
        format: BodyFormat::Json,
        expected_body: None,
        targets: vec![Kind::Verifier],
    };
    let mut counts = BTreeMap::new();
    for verifier in verifiers {
        let started = Instant::now();
        let row = fetch(client, verifier, &case, verifiers, "discovery").await;
        if row.ok {
            let body: serde_json::Value = serde_json::from_slice(row.body.as_ref().unwrap())?;
            *counts
                .entry(body["slot"].as_u64().expect("validated snapshot"))
                .or_insert(0usize) += 1;
        }
        journal.consume(row)?;
        sleep(config::seconds(1.0 / args.rps)?.saturating_sub(started.elapsed())).await;
    }
    counts
        .into_iter()
        .max_by_key(|(slot, count)| (*count, *slot))
        .map(|(slot, _)| slot)
        .ok_or_else(|| {
            Error::Invalid("no verifier returned a usable snapshot; see discovery requests".into())
        })
}

async fn load(
    args: &Args,
    client: &Client,
    targets: &[Target],
    cases: &[Case],
    journal: &mut Journal,
    cancellation: &CancellationToken,
) -> Result<f64> {
    issue_requests(
        args,
        client,
        Workload {
            targets,
            cases,
            phase: "load",
            limit: Limit::Duration(config::seconds(args.duration)?),
        },
        journal,
        cancellation,
    )
    .await
}

async fn sample_router(
    args: &Args,
    client: &Client,
    targets: &[Target],
    slot: u64,
    journal: &mut Journal,
    cancellation: &CancellationToken,
) -> Result<f64> {
    let count = targets
        .iter()
        .filter(|target| target.kind == Kind::Verifier)
        .count();
    let required = sampling::required_samples(count, args.router_confidence);
    let attributed = journal
        .records
        .iter()
        .filter(|row| row.phase == "load" && row.kind == Kind::Router && row.verifier.is_some())
        .count();
    let remaining = required.saturating_sub(attributed);
    if remaining == 0 {
        return Ok(0.0);
    }
    tracing::info!(
        required,
        attributed,
        additional_requests = remaining,
        "Sampling router selections"
    );
    let case = Case {
        name: "router_selection".into(),
        path: format!("/meta?network={}&slot={slot}", args.network.as_str()),
        expected_status: 200,
        format: BodyFormat::Json,
        expected_body: None,
        targets: vec![Kind::Router],
    };
    issue_requests(
        args,
        client,
        Workload {
            targets,
            cases: &[case],
            phase: "router_sampling",
            limit: Limit::Requests(remaining),
        },
        journal,
        cancellation,
    )
    .await
}

enum Limit {
    Duration(Duration),
    Requests(usize),
}

impl Limit {
    fn reached(&self, issued: usize, elapsed: Duration) -> bool {
        match self {
            Self::Duration(duration) => elapsed >= *duration,
            Self::Requests(count) => issued >= *count,
        }
    }

    fn delay(&self, until_next: Duration, elapsed: Duration) -> Duration {
        match self {
            Self::Duration(duration) => until_next.min(duration.saturating_sub(elapsed)),
            Self::Requests(_) => until_next,
        }
    }
}

struct Workload<'a> {
    targets: &'a [Target],
    cases: &'a [Case],
    phase: &'static str,
    limit: Limit,
}

async fn issue_requests(
    args: &Args,
    client: &Client,
    workload: Workload<'_>,
    journal: &mut Journal,
    cancellation: &CancellationToken,
) -> Result<f64> {
    let Workload {
        targets,
        cases,
        phase,
        limit,
    } = workload;
    let verifiers: Vec<_> = targets
        .iter()
        .filter(|target| target.kind == Kind::Verifier)
        .cloned()
        .collect();
    let jobs: Vec<_> = cases
        .iter()
        .flat_map(|case| {
            targets
                .iter()
                .filter(move |target| case.targets.contains(&target.kind))
                .map(move |target| (target, case))
        })
        .collect();
    let mut pending = JoinSet::new();
    let started = Instant::now();
    let interval = config::seconds(1.0 / args.rps)?;
    let mut next_at = if phase == "router_sampling" {
        started + interval
    } else {
        started
    };
    let mut issued = 0;
    let mut interrupted = Box::pin(tokio::signal::ctrl_c());
    let mut result = Ok(());
    loop {
        if limit.reached(issued, started.elapsed())
            || result.is_err()
            || cancellation.is_cancelled()
        {
            break;
        }
        let delay = limit.delay(
            next_at.saturating_duration_since(Instant::now()),
            started.elapsed(),
        );
        if pending.len() >= args.concurrency {
            tokio::select! {
                _ = &mut interrupted => { cancellation.cancel(); },
                row = pending.join_next() => { if let Some(row) = row { result = row.map_err(Error::from).and_then(|row| journal.consume(row)); } },
            }
            continue;
        }
        tokio::select! {
            _ = &mut interrupted => { cancellation.cancel(); continue; },
            _ = sleep(delay) => {},
        }
        if limit.reached(issued, started.elapsed()) {
            break;
        }
        let (target, case) = jobs[issued % jobs.len()];
        let (client, target, case, verifiers) = (
            client.clone(),
            target.clone(),
            case.clone(),
            verifiers.to_vec(),
        );
        pending.spawn(async move { fetch(&client, &target, &case, &verifiers, phase).await });
        issued += 1;
        // Schedule from the actual issue time so backpressure never creates a catch-up burst.
        next_at = Instant::now() + interval;
        while let Some(row) = pending.try_join_next() {
            if let Err(error) = row
                .map_err(Error::from)
                .and_then(|row| journal.consume(row))
            {
                result = Err(error);
            }
        }
    }
    while let Some(row) = pending.join_next().await {
        if let Err(error) = row
            .map_err(Error::from)
            .and_then(|row| journal.consume(row))
        {
            result = Err(error);
        }
    }
    result?;
    Ok(started.elapsed().as_secs_f64())
}

#[cfg(test)]
mod tests;
