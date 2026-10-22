use super::{
    config::{Args, Case, Kind, Target},
    http::{hash, Record},
    sampling, Result,
};
use chrono::Utc;
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

const ROUTER_FAILURES_FILE: &str = "router-failures.json";

pub struct Journal {
    file: File,
    output: PathBuf,
    pub records: Vec<Record>,
}

impl Journal {
    pub fn new(args: &Args) -> Result<Self> {
        fs::create_dir_all(args.output.join("responses"))?;
        Ok(Self {
            file: File::create(args.output.join("requests.jsonl"))?,
            output: args.output.clone(),
            records: Vec::new(),
        })
    }

    pub fn consume(&mut self, mut record: Record) -> Result<()> {
        record.id = self.records.len() + 1;
        if let Some(body) = record.body.take() {
            let relative = format!("responses/{}.txt", hash(&body));
            fs::write(self.output.join(&relative), body)?;
            record.response_file = Some(relative);
        }
        serde_json::to_writer(&mut self.file, &record)?;
        self.file.write_all(b"\n")?;
        self.file.flush()?;
        self.records.push(record);
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub target: String,
    pub kind: Kind,
    pub request: String,
    pub attempts: usize,
    pub failures: usize,
    pub p50_ms: Option<f64>,
    pub p95_ms: Option<f64>,
    pub p99_ms: Option<f64>,
    pub statuses: BTreeMap<String, usize>,
    pub latency_comparison: String,
    pub slow: bool,
}

#[derive(Serialize)]
pub struct Report {
    pub schema_version: u8,
    pub started_at: String,
    pub network: String,
    pub config: Args,
    pub passed: bool,
    pub slot: Option<u64>,
    pub elapsed_seconds: f64,
    pub achieved_rps: f64,
    pub workload: Vec<Case>,
    pub verifiers: Vec<Target>,
    pub findings: Vec<Value>,
    pub targets: Vec<Summary>,
    pub router_selections: BTreeMap<String, usize>,
    pub router_required_samples: usize,
    pub router_attempts: usize,
    pub router_attributed_samples: usize,
    pub router_failed_requests: usize,
    pub router_sampling_seconds: f64,
    pub router_miss_probability_bound: Option<f64>,
    pub load_requests: usize,
    pub failed_requests: usize,
    pub http_attempts: usize,
}

impl Report {
    pub fn new(args: Args) -> Self {
        Self {
            schema_version: 1,
            started_at: Utc::now().to_rfc3339(),
            network: args.network.as_str().into(),
            config: args,
            passed: false,
            slot: None,
            elapsed_seconds: 0.0,
            achieved_rps: 0.0,
            workload: Vec::new(),
            verifiers: Vec::new(),
            findings: Vec::new(),
            targets: Vec::new(),
            router_selections: BTreeMap::new(),
            router_required_samples: 0,
            router_attempts: 0,
            router_attributed_samples: 0,
            router_failed_requests: 0,
            router_sampling_seconds: 0.0,
            router_miss_probability_bound: None,
            load_requests: 0,
            failed_requests: 0,
            http_attempts: 0,
        }
    }

    /// Aggregate request failures, response variants, coverage, and peer-relative latency.
    pub fn analyze(&mut self, records: &[Record], targets: &[Target], cases: &[Case]) {
        let mut variants: BTreeMap<&str, BTreeMap<&str, Vec<usize>>> = BTreeMap::new();
        for record in records {
            self.http_attempts += record.hops.len();
            // A current RPC account can legitimately be absent from an older snapshot.
            let candidate_miss = record.phase == "account_discovery"
                && record.status == Some(404)
                && record.issues == ["expected HTTP 200, got 404"];
            if !record.ok && !candidate_miss {
                self.findings.push(
                    json!({"type":"request_failed", "id":record.id, "target":record.target,
                    "verifier":record.verifier, "request":record.request, "status":record.status,
                    "elapsed_ms":record.elapsed_ms, "issues":record.issues}),
                );
            }
            if record.phase != "load" && record.phase != "router_sampling" {
                continue;
            }
            if record.ok {
                if let Some(hash) = &record.response_sha256 {
                    variants
                        .entry(&record.request)
                        .or_default()
                        .entry(hash)
                        .or_default()
                        .push(record.id);
                }
            }
            if record.kind == Kind::Router {
                self.router_attempts += 1;
                self.router_failed_requests += usize::from(!record.ok);
                if let Some(verifier) = &record.verifier {
                    *self.router_selections.entry(verifier.clone()).or_default() += 1;
                    self.router_attributed_samples += 1;
                }
            }
            if record.phase == "load" {
                self.load_requests += 1;
                self.failed_requests += usize::from(!record.ok);
            }
        }
        for (request, variants) in variants {
            if variants.len() > 1 {
                self.findings.push(
                    json!({"type":"response_mismatch", "request":request, "variants":variants}),
                );
            }
        }
        for target in targets {
            if target.kind == Kind::Verifier {
                self.router_selections
                    .entry(target.name.clone())
                    .or_default();
            }
            for case in cases
                .iter()
                .filter(|case| case.targets.contains(&target.kind))
            {
                let rows: Vec<_> = records
                    .iter()
                    .filter(|row| {
                        row.phase == "load" && row.target == target.name && row.request == case.name
                    })
                    .collect();
                let successful: Vec<_> = rows
                    .iter()
                    .filter(|row| row.ok)
                    .map(|row| row.elapsed_ms)
                    .collect();
                let mut statuses = BTreeMap::new();
                for row in &rows {
                    *statuses
                        .entry(
                            row.status
                                .map_or_else(|| "no_response".into(), |status| status.to_string()),
                        )
                        .or_default() += 1;
                }
                if rows.is_empty() {
                    self.findings.push(json!({"type":"missing_coverage", "target":target.name, "request":case.name}));
                }
                self.targets.push(Summary {
                    target: target.name.clone(),
                    kind: target.kind,
                    request: case.name.clone(),
                    attempts: rows.len(),
                    failures: rows.len() - successful.len(),
                    p50_ms: percentile(&successful, 0.5),
                    p95_ms: percentile(&successful, 0.95),
                    p99_ms: percentile(&successful, 0.99),
                    statuses,
                    latency_comparison: "insufficient_samples".into(),
                    slow: false,
                });
            }
        }
        for index in 0..self.targets.len() {
            let summary = &self.targets[index];
            if summary.kind != Kind::Verifier
                || summary.attempts - summary.failures < self.config.min_samples
            {
                continue;
            }
            let mut peers: Vec<_> = self
                .targets
                .iter()
                .filter(|peer| {
                    peer.kind == Kind::Verifier
                        && peer.target != summary.target
                        && peer.request == summary.request
                        && peer.attempts - peer.failures >= self.config.min_samples
                })
                .filter_map(|peer| peer.p50_ms)
                .collect();
            peers.sort_by(f64::total_cmp);
            let summary = &mut self.targets[index];
            summary.latency_comparison = if peers.is_empty() {
                "insufficient_peers"
            } else {
                "compared"
            }
            .into();
            if peers.is_empty() {
                continue;
            }
            let median = (peers[(peers.len() - 1) / 2] + peers[peers.len() / 2]) / 2.0;
            let p50 = summary.p50_ms.expect("successful samples have latency");
            if p50 > (median * self.config.slow_factor).max(median + self.config.slow_ms) {
                summary.slow = true;
                self.findings.push(json!({"type":"slow_verifier", "target":summary.target, "request":summary.request,
                    "p50_ms":p50, "peer_median_ms":median}));
            }
        }
        if targets.iter().any(|target| target.kind == Kind::Router) {
            let verifiers: Vec<_> = targets
                .iter()
                .filter(|target| target.kind == Kind::Verifier)
                .collect();
            self.router_required_samples =
                sampling::required_samples(verifiers.len(), self.config.router_confidence);
            self.router_miss_probability_bound = Some(sampling::miss_probability_bound(
                verifiers.len(),
                self.router_attributed_samples,
            ));
            if self.router_attributed_samples < self.router_required_samples {
                self.findings.push(json!({"type":"insufficient_router_samples", "required":self.router_required_samples,
                    "attributed":self.router_attributed_samples, "attempts":self.router_attempts}));
            }
            for verifier in verifiers {
                if self
                    .router_selections
                    .get(&verifier.name)
                    .copied()
                    .unwrap_or(0)
                    == 0
                {
                    self.findings.push(
                        json!({"type":"router_verifier_not_selected", "target":verifier.name,
                        "domain":verifier.url, "router_samples":self.router_attributed_samples}),
                    );
                }
            }
        }
        self.passed = self.findings.is_empty();
        if self.elapsed_seconds > 0.0 {
            self.achieved_rps = self.load_requests as f64 / self.elapsed_seconds;
        }
    }

    pub fn write(&self, records: &[Record]) -> Result<()> {
        let failures: Vec<_> = records
            .iter()
            .filter(|record| record.kind == Kind::Router && !record.ok)
            .collect();
        let failures_path = self.config.output.join(ROUTER_FAILURES_FILE);
        if failures.is_empty() {
            match fs::remove_file(failures_path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        } else {
            let mut by_status = BTreeMap::new();
            let mut by_verifier = BTreeMap::new();
            for record in &failures {
                *by_status.entry(record.status).or_insert(0usize) += 1;
                *by_verifier
                    .entry(record.verifier.as_deref())
                    .or_insert(0usize) += 1;
            }
            let by_status: Vec<_> = by_status
                .into_iter()
                .map(|(status, count)| json!({"status":status, "count":count}))
                .collect();
            let by_verifier: Vec<_> = by_verifier
                .into_iter()
                .map(|(verifier, count)| json!({"verifier":verifier, "count":count}))
                .collect();
            fs::write(
                failures_path,
                serde_json::to_vec_pretty(&json!({"total":failures.len(),
                "by_status":by_status, "by_verifier":by_verifier, "failures":failures}))?,
            )?;
        }
        fs::write(
            self.config.output.join("report.json"),
            serde_json::to_vec_pretty(self)?,
        )?;
        let markdown = self.markdown();
        fs::write(self.config.output.join("summary.md"), &markdown)?;
        std::io::stdout().lock().write_all(markdown.as_bytes())?;
        Ok(())
    }

    pub fn markdown(&self) -> String {
        let mut lines = vec!["# NCN network load test".into(), String::new(),
            format!("Result: **{}**", if self.passed { "PASS" } else { "FAIL" }), String::new(),
            format!("Network: {}; snapshot slot: {}.", self.network, self.slot.map_or_else(|| "unavailable".into(), |slot| slot.to_string())),
            String::new(), format!("{} load requests; {} failed. Requested rate: {} logical requests/s; achieved: {:.2}/s; elapsed including drain: {:.1}s.",
                self.load_requests, self.failed_requests, self.config.rps, self.achieved_rps, self.elapsed_seconds)];
        lines.extend([String::new(), "## Router selections".into(), String::new()]);
        if self.router_required_samples > 0 {
            lines.push(format!("{} attributed selections from {} router requests; required samples: {}; failed router requests: {}.",
                self.router_attributed_samples, self.router_attempts, self.router_required_samples, self.router_failed_requests));
            lines.push(String::new());
            lines.push(format!("Coverage confidence target: {:.3}%, assuming independent, uniform selections across configured verifiers. Additional sampling took {:.1}s.",
                self.config.router_confidence * 100.0, self.router_sampling_seconds));
            lines.push(String::new());
        }
        if self.router_failed_requests > 0 {
            lines.push(format!(
                "[Router failure details ({} requests, JSON)]({ROUTER_FAILURES_FILE})",
                self.router_failed_requests
            ));
            lines.push(String::new());
        }
        for (verifier, count) in &self.router_selections {
            lines.push(format!(
                "- {}: {count}{}",
                cell(verifier),
                if *count == 0 {
                    " (ERROR: never selected)"
                } else {
                    ""
                }
            ));
        }
        let ms =
            |value: Option<f64>| value.map_or_else(|| "—".into(), |value| format!("{value:.1}"));
        let mut requests = BTreeMap::<&str, Vec<&Summary>>::new();
        for row in &self.targets {
            requests.entry(&row.request).or_default().push(row);
        }
        for (request, rows) in requests {
            lines.extend([
                String::new(),
                format!("## Request: {}", cell(request)),
                String::new(),
                "| Target | Attempts | Failures | p50 ms | p95 ms | p99 ms | Latency comparison |"
                    .into(),
                "| --- | ---: | ---: | ---: | ---: | ---: | --- |".into(),
            ]);
            for row in rows {
                let comparison = if row.kind == Kind::Router {
                    "router"
                } else if row.slow {
                    "slow"
                } else {
                    &row.latency_comparison
                };
                lines.push(format!(
                    "| {} | {} | {} | {} | {} | {} | {} |",
                    cell(&row.target),
                    row.attempts,
                    row.failures,
                    ms(row.p50_ms),
                    ms(row.p95_ms),
                    ms(row.p99_ms),
                    comparison
                ));
            }
        }
        lines.extend([String::new(), "## Findings".into(), String::new()]);
        let mut counts = BTreeMap::new();
        for finding in &self.findings {
            *counts
                .entry(finding["type"].as_str().unwrap_or("unknown"))
                .or_insert(0) += 1;
        }
        for (kind, count) in counts {
            lines.push(format!("- {kind}: {count}"));
        }
        if self.findings.is_empty() {
            lines.push("No failures, response differences, or latency outliers detected.".into());
        }
        for finding in self.findings.iter().take(50) {
            lines.extend([
                String::new(),
                "```json".into(),
                serde_json::to_string_pretty(finding).expect("finding contains valid JSON"),
                "```".into(),
            ]);
        }
        lines.extend([String::new(), "Full findings: `report.json`. Every request and redirect: `requests.jsonl`. Response evidence: `responses/`.".into(),
            String::new(), "Latency percentiles use successful requests. Peer comparisons use direct verifier requests for the same case. A configured verifier with zero router selections fails the run.".into(), String::new()]);
        lines.join("\n")
    }
}

fn cell(value: &str) -> String {
    value
        .replace('|', "\\|")
        .replace(['\n', '\r'], " ")
        .replace('`', "'")
}

fn percentile(values: &[f64], quantile: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    Some(sorted[(sorted.len() as f64 * quantile).ceil() as usize - 1])
}
