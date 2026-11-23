use super::*;
use clap::Parser;
use serde_json::Value;
use std::{
    fs,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
use tiny_http::{Header, Response, Server};

const META: &str =
    r#"{"network":"mainnet","slot":42,"merkle_root":"root","active_stake":18446744073709551615}"#;

struct Fixture {
    url: String,
    stopped: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
    paths: Arc<Mutex<Vec<String>>>,
}

impl Fixture {
    fn new(handler: impl Fn(tiny_http::Request) + Send + Sync + 'static) -> Self {
        let server = Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", server.server_addr());
        let stopped = Arc::new(AtomicBool::new(false));
        let paths = Arc::new(Mutex::new(Vec::new()));
        let (stop, requests) = (stopped.clone(), paths.clone());
        let handler = Arc::new(handler);
        let worker = thread::spawn(move || {
            let mut handlers = Vec::new();
            while !stop.load(Ordering::Relaxed) {
                if let Some(request) = server.recv_timeout(Duration::from_millis(10)).unwrap() {
                    requests.lock().unwrap().push(request.url().into());
                    let handler = handler.clone();
                    handlers.push(thread::spawn(move || handler(request)));
                }
            }
            for handler in handlers {
                handler.join().unwrap();
            }
        });
        Self {
            url,
            stopped,
            worker: Some(worker),
            paths,
        }
    }

    fn response(status: u16, body: &'static str) -> Self {
        Self::new(move |request| {
            let _ = request.respond(Response::from_string(body).with_status_code(status));
        })
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        self.worker.take().unwrap().join().unwrap();
    }
}

fn args(directory: &tempfile::TempDir) -> Args {
    Args::parse_from([
        "loadtest",
        "--output",
        directory.path().to_str().unwrap(),
        "--duration",
        ".3",
        "--rps",
        "40",
    ])
}

fn verifier(name: &str, url: &str) -> Target {
    Target {
        name: name.into(),
        url: url.into(),
        kind: Kind::Verifier,
    }
}

fn case() -> Case {
    Case {
        name: "meta".into(),
        path: "/meta?network=mainnet&slot=42".into(),
        expected_status: 200,
        format: BodyFormat::Json,
        expected_body: None,
        targets: vec![Kind::Router, Kind::Verifier],
    }
}

fn client(timeout: f64) -> Client {
    Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs_f64(timeout))
        .build()
        .unwrap()
}

#[tokio::test]
async fn discovers_snapshot_follows_router_and_saves_evidence_without_routing_health() {
    let upstream = Fixture::new(|request| {
        let body = if request.url() == "/healthz" {
            "ok"
        } else {
            META
        };
        request.respond(Response::from_string(body)).unwrap();
    });
    let upstream_url = upstream.url.clone();
    let router = Fixture::new(move |request| {
        let location = format!("{}{}", upstream_url, request.url().trim_start_matches('/'));
        request
            .respond(
                Response::empty(302).with_header(Header::from_bytes("Location", location).unwrap()),
            )
            .unwrap();
    });
    let directory = tempfile::tempdir().unwrap();
    let mut args = args(&directory);
    args.router_url = router.url.clone();
    args.verifiers = directory.path().join("verifiers.toml");
    args.requests = directory.path().join("requests.toml");
    fs::write(
        &args.verifiers,
        format!(
            "[[verifiers]]\nname='alice'\nverification_domain='{}'\n",
            upstream.url
        ),
    )
    .unwrap();
    fs::write(&args.requests, "[[requests]]\nname='meta'\npath='/meta?network={network}&slot={slot}'\n\n[[requests]]\nname='health'\npath='/healthz'\nformat='text'\nexpected_body='ok'\ntargets=['verifier']\n").unwrap();
    assert!(run(args.clone()).await.unwrap());
    let report: Value =
        serde_json::from_slice(&fs::read(args.output.join("report.json")).unwrap()).unwrap();
    assert_eq!(report["slot"], 42);
    assert!(report["router_selections"]["alice"].as_u64().unwrap() > 0);
    let journal = fs::read_to_string(args.output.join("requests.jsonl")).unwrap();
    let rows: Vec<Value> = journal
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let routed = rows.iter().find(|row| row["kind"] == "router").unwrap();
    assert_eq!(routed["hops"][0]["status"], 302);
    assert_eq!(routed["hops"][1]["status"], 200);
    assert!(args
        .output
        .join(routed["response_file"].as_str().unwrap())
        .is_file());
    assert!(!router
        .paths
        .lock()
        .unwrap()
        .iter()
        .any(|path| path == "/healthz"));
    assert!(upstream
        .paths
        .lock()
        .unwrap()
        .iter()
        .any(|path| path == "/healthz"));
}

#[tokio::test]
async fn proxy_identity_is_retained_on_http_errors() {
    let proxy = Fixture::new(|request| {
        request
            .respond(
                Response::from_string("unavailable")
                    .with_status_code(503)
                    .with_header(
                        Header::from_bytes("X-NCN-Verifier", "https://alice.example/").unwrap(),
                    ),
            )
            .unwrap();
    });
    let router = Target {
        name: "router".into(),
        url: proxy.url.clone(),
        kind: Kind::Router,
    };
    let row = fetch(
        &client(1.0),
        &router,
        &case(),
        &[verifier("alice", "https://alice.example/")],
        "load",
    )
    .await;
    assert_eq!(row.verifier.as_deref(), Some("alice"));
    assert_eq!(row.status, Some(503));
    assert_eq!(row.body.as_deref(), Some(b"unavailable".as_slice()));
    assert!(!row.ok);
}

#[tokio::test]
async fn timeout_keeps_selected_verifier_and_failed_hop() {
    let upstream = Fixture::new(|request| {
        thread::sleep(Duration::from_millis(150));
        let _ = request.respond(Response::from_string(META));
    });
    let url = upstream.url.clone();
    let router = Fixture::new(move |request| {
        let location = format!("{}{}", url, request.url().trim_start_matches('/'));
        request
            .respond(
                Response::empty(302).with_header(Header::from_bytes("Location", location).unwrap()),
            )
            .unwrap();
    });
    let target = Target {
        name: "router".into(),
        url: router.url.clone(),
        kind: Kind::Router,
    };
    let row = fetch(
        &client(0.03),
        &target,
        &case(),
        &[verifier("alice", &upstream.url)],
        "load",
    )
    .await;
    assert!(!row.ok);
    assert_eq!(row.verifier.as_deref(), Some("alice"));
    assert_eq!(row.hops.len(), 2);
    assert!(row.issues.iter().any(|issue| issue == "timeout"));
    assert!(row.hops[1].elapsed_ms > 0.0);
}

#[tokio::test]
async fn changed_query_and_unknown_redirect_are_failures() {
    let upstream = Fixture::response(200, META);
    let url = upstream.url.clone();
    let router = Fixture::new(move |request| {
        request
            .respond(
                Response::empty(302).with_header(
                    Header::from_bytes("Location", format!("{url}meta?network=testnet&slot=42"))
                        .unwrap(),
                ),
            )
            .unwrap();
    });
    let target = Target {
        name: "router".into(),
        url: router.url.clone(),
        kind: Kind::Router,
    };
    let changed = fetch(
        &client(1.0),
        &target,
        &case(),
        &[verifier("alice", &upstream.url)],
        "load",
    )
    .await;
    assert!(changed
        .issues
        .iter()
        .any(|issue| issue == "router changed the request path or query"));
    let unknown = fetch(
        &client(1.0),
        &target,
        &case(),
        &[verifier("bob", "https://bob.example/")],
        "load",
    )
    .await;
    assert!(unknown
        .issues
        .iter()
        .any(|issue| issue == "router did not identify a configured verifier"));
}

#[test]
fn normalization_preserves_proof_order_and_integer_precision() {
    let mut case = case();
    let a = br#"{"network":"mainnet","slot":42,"created_at":"a","amount":18446744073709551615,"proof":["a","b"]}"#;
    let b = br#"{"proof":["a","b"],"amount":18446744073709551615,"created_at":"b","slot":42,"network":"mainnet"}"#;
    assert_eq!(
        http::response_hash(a, &case, true).unwrap(),
        http::response_hash(b, &case, true).unwrap()
    );
    let changed = String::from_utf8(b.to_vec())
        .unwrap()
        .replace("18446744073709551615", "18446744073709551614");
    assert_ne!(
        http::response_hash(a, &case, true).unwrap(),
        http::response_hash(changed.as_bytes(), &case, true).unwrap()
    );
    let reversed = String::from_utf8(b.to_vec())
        .unwrap()
        .replace(r#"["a","b"]"#, r#"["b","a"]"#);
    assert_ne!(
        http::response_hash(a, &case, true).unwrap(),
        http::response_hash(reversed.as_bytes(), &case, true).unwrap()
    );
    case.path = "/proof/vote_account/key?network=mainnet&slot=42".into();
    assert_ne!(
        http::response_hash(a, &case, false).unwrap(),
        http::response_hash(b, &case, false).unwrap()
    );
}

#[tokio::test]
async fn status_only_negative_cases_accept_different_bodies_and_preserve_evidence() {
    let request: Case = toml::from_str(
        "name='missing_proof'\npath='/proof/vote_account/absent?network=mainnet&slot=42'\nexpected_status=404\nformat='status'\ntargets=['verifier']",
    ).unwrap();
    let servers = [
        Fixture::response(404, "not found on alice"),
        Fixture::response(404, "<html>missing on bob</html>"),
    ];
    let targets = [
        verifier("alice", &servers[0].url),
        verifier("bob", &servers[1].url),
    ];
    let directory = tempfile::tempdir().unwrap();
    let args = args(&directory);
    let mut journal = Journal::new(&args).unwrap();
    let client = client(1.0);
    for target in &targets {
        let record = fetch(&client, target, &request, &targets, "load").await;
        assert!(record.ok, "{:?}", record.issues);
        journal.consume(record).unwrap();
    }
    let mut report = Report::new(args.clone());
    report.analyze(&journal.records, &targets, &[request.clone()]);
    assert!(report.passed, "{:?}", report.findings);
    assert_eq!(
        journal.records[0].response_sha256,
        journal.records[1].response_sha256
    );
    let paths: Vec<_> = journal
        .records
        .iter()
        .map(|record| args.output.join(record.response_file.as_ref().unwrap()))
        .collect();
    assert_ne!(paths[0], paths[1]);
    assert_eq!(fs::read_to_string(&paths[0]).unwrap(), "not found on alice");
    assert_eq!(
        fs::read_to_string(&paths[1]).unwrap(),
        "<html>missing on bob</html>"
    );

    for status in [200, 500] {
        let server = Fixture::response(status, "not found");
        let record = fetch(
            &client,
            &verifier("wrong_status", &server.url),
            &request,
            &[],
            "load",
        )
        .await;
        assert!(!record.ok);
        assert!(record.response_sha256.is_none());
        assert!(record
            .issues
            .iter()
            .any(|issue| issue == &format!("expected HTTP 404, got {status}")));
    }
    let mut exact_body = request;
    exact_body.expected_body = Some("expected error".into());
    let record = fetch(&client, &targets[0], &exact_body, &targets, "load").await;
    assert!(!record.ok);
    assert!(record
        .issues
        .iter()
        .any(|issue| issue == "response body differs from expected_body"));
}

#[tokio::test]
async fn json_cases_compare_bodies_for_created_and_error_responses() {
    for status in [201, 404] {
        let servers = [
            Fixture::response(status, r#"{"message":"alice"}"#),
            Fixture::response(status, r#"{"message":"bob"}"#),
        ];
        let targets = [
            verifier("alice", &servers[0].url),
            verifier("bob", &servers[1].url),
        ];
        let request = Case {
            expected_status: status,
            targets: vec![Kind::Verifier],
            ..case()
        };
        let client = client(1.0);
        let records = vec![
            fetch(&client, &targets[0], &request, &targets, "load").await,
            fetch(&client, &targets[1], &request, &targets, "load").await,
        ];
        assert!(records.iter().all(|record| record.ok), "{records:?}");
        let directory = tempfile::tempdir().unwrap();
        let mut report = Report::new(args(&directory));
        report.analyze(&records, &targets, &[request]);
        assert!(!report.passed);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding["type"] == "response_mismatch"));
    }
}

#[tokio::test]
async fn successful_http_status_cannot_hide_wrong_snapshot_or_empty_batch() {
    for (body, path) in [
        (
            r#"{"network":"mainnet","slot":43}"#,
            "/meta?network=mainnet&slot=42",
        ),
        ("[]", "/metas?network=mainnet&slots=42"),
    ] {
        let server = Fixture::response(200, body);
        let mut case = case();
        case.path = path.into();
        let row = fetch(
            &client(1.0),
            &verifier("alice", &server.url),
            &case,
            &[],
            "load",
        )
        .await;
        assert!(!row.ok);
        assert!(row.response_sha256.is_none());
        assert_eq!(row.body.as_deref(), Some(body.as_bytes()));
    }
}

#[tokio::test]
async fn analysis_reports_disagreement_slow_verifiers_and_missing_coverage() {
    let server = Fixture::response(200, META);
    let targets: Vec<_> = ["alice", "bob", "slow", "absent"]
        .iter()
        .map(|name| verifier(name, &server.url))
        .collect();
    let mut records = Vec::new();
    for target in targets.iter().take(3) {
        for _ in 0..3 {
            let mut row = fetch(&client(1.0), target, &case(), &targets, "load").await;
            row.id = records.len() + 1;
            row.elapsed_ms = if target.name == "slow" { 500.0 } else { 10.0 };
            if target.name == "bob" {
                row.response_sha256 = Some("different".into());
            }
            records.push(row);
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let mut report = Report::new(args(&directory));
    report.analyze(&records, &targets, &[case()]);
    assert!(!report.passed);
    assert!(report
        .findings
        .iter()
        .any(|f| f["type"] == "response_mismatch"));
    assert!(report
        .findings
        .iter()
        .any(|f| f["type"] == "slow_verifier" && f["target"] == "slow"));
    assert!(report
        .findings
        .iter()
        .any(|f| f["type"] == "missing_coverage" && f["target"] == "absent"));
}

#[tokio::test]
async fn sampling_metadata_is_compared_with_load_metadata_despite_case_names() {
    let upstream = Fixture::response(200, META);
    let identity = upstream.url.clone();
    let router = Fixture::new(move |request| {
        request
            .respond(
                Response::from_string(META.replace("root", "changed-root")).with_header(
                    Header::from_bytes("X-NCN-Verifier", identity.as_bytes()).unwrap(),
                ),
            )
            .unwrap();
    });
    let targets = vec![
        verifier("alice", &upstream.url),
        Target {
            name: "router".into(),
            url: router.url.clone(),
            kind: Kind::Router,
        },
    ];
    let load_case = Case {
        name: "snapshot_meta".into(),
        targets: vec![Kind::Verifier],
        ..case()
    };
    let sampling_case = Case {
        name: "router_selection".into(),
        targets: vec![Kind::Router],
        ..case()
    };
    let client = client(1.0);
    let mut records = vec![fetch(&client, &targets[0], &load_case, &targets, "load").await];
    for _ in 0..2 {
        records.push(
            fetch(
                &client,
                &targets[1],
                &sampling_case,
                &targets,
                "router_sampling",
            )
            .await,
        );
    }
    for (index, record) in records.iter_mut().enumerate() {
        record.id = index + 1;
        assert!(record.ok, "{:?}", record.issues);
    }
    let directory = tempfile::tempdir().unwrap();
    let mut report = Report::new(args(&directory));
    report.analyze(&records, &targets, &[load_case]);
    assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
    let mismatch = &report.findings[0];
    assert_eq!(mismatch["type"], "response_mismatch");
    assert_eq!(mismatch["comparison"]["path"], sampling_case.path);
    assert_eq!(
        mismatch["requests"],
        json!(["router_selection", "snapshot_meta"])
    );
    let mut groups: Vec<_> = mismatch["variants"]
        .as_object()
        .unwrap()
        .values()
        .cloned()
        .collect();
    groups.sort_by_key(|group| group.as_array().unwrap().len());
    assert_eq!(groups, vec![json!([1]), json!([2, 3])]);
    assert_eq!(report.load_requests, 1);
    assert_eq!(report.targets.len(), 1);
    assert_eq!(report.targets[0].attempts, 1);
    assert_eq!(report.router_attempts, 2);
}

#[tokio::test]
async fn custom_router_selection_cases_are_separate_from_incompatible_sampling_requests() {
    let changed_meta = META.replace("root", "custom-root");
    for (path, format, status, expected_body, body) in [
        ("/healthz", BodyFormat::Text, 200, None, "ok"),
        (&case().path, BodyFormat::Text, 200, None, META),
        (&case().path, BodyFormat::Json, 201, None, &changed_meta),
        (
            &case().path,
            BodyFormat::Json,
            200,
            Some(changed_meta.clone()),
            &changed_meta,
        ),
    ] {
        let body = body.to_owned();
        let upstream = Fixture::new(move |request| {
            request
                .respond(Response::from_string(&body).with_status_code(status))
                .unwrap();
        });
        let identity = upstream.url.clone();
        let router = Fixture::new(move |request| {
            request
                .respond(Response::from_string(META).with_header(
                    Header::from_bytes("X-NCN-Verifier", identity.as_bytes()).unwrap(),
                ))
                .unwrap();
        });
        let targets = vec![
            verifier("alice", &upstream.url),
            Target {
                name: "router".into(),
                url: router.url.clone(),
                kind: Kind::Router,
            },
        ];
        let load_case = Case {
            name: "router_selection".into(),
            path: path.into(),
            format,
            expected_status: status,
            expected_body,
            targets: vec![Kind::Verifier],
        };
        let sampling_case = Case {
            name: "router_selection".into(),
            ..case()
        };
        let client = client(1.0);
        let records = vec![
            fetch(&client, &targets[0], &load_case, &targets, "load").await,
            fetch(
                &client,
                &targets[1],
                &sampling_case,
                &targets,
                "router_sampling",
            )
            .await,
        ];
        assert!(records.iter().all(|record| record.ok), "{records:?}");
        let directory = tempfile::tempdir().unwrap();
        let mut report = Report::new(args(&directory));
        report.analyze(&records, &targets, &[load_case]);
        assert!(report.passed, "{:?}", report.findings);
        assert_eq!(report.load_requests, 1);
        assert_eq!(report.targets[0].attempts, 1);
        assert_eq!(report.router_attempts, 1);
    }
}

#[tokio::test]
async fn failed_discovery_writes_a_failing_report_and_journal() {
    let server = Fixture::response(503, "unavailable");
    let directory = tempfile::tempdir().unwrap();
    let mut args = args(&directory);
    args.verifiers = directory.path().join("verifiers.toml");
    fs::write(
        &args.verifiers,
        format!(
            "[[verifiers]]\nname='alice'\nverification_domain='{}'\n",
            server.url
        ),
    )
    .unwrap();
    assert!(!run(args.clone()).await.unwrap());
    let report: Value =
        serde_json::from_slice(&fs::read(args.output.join("report.json")).unwrap()).unwrap();
    assert!(report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|finding| finding["type"] == "setup_failed"));
    assert!(args.output.join("summary.md").is_file());
    assert!(fs::read_to_string(args.output.join("requests.jsonl"))
        .unwrap()
        .contains("503"));
}

#[tokio::test]
async fn load_limits_concurrency_and_drains_requests() {
    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let (active_handler, peak_handler) = (active.clone(), peak.clone());
    let server = Fixture::new(move |request| {
        let count = active_handler.fetch_add(1, Ordering::SeqCst) + 1;
        peak_handler.fetch_max(count, Ordering::SeqCst);
        thread::sleep(Duration::from_millis(50));
        request.respond(Response::from_string(META)).unwrap();
        active_handler.fetch_sub(1, Ordering::SeqCst);
    });
    let directory = tempfile::tempdir().unwrap();
    let mut args = args(&directory);
    args.duration = 0.16;
    args.rps = 100.0;
    args.concurrency = 2;
    let mut journal = Journal::new(&args).unwrap();
    load(
        &args,
        &client(1.0),
        &[verifier("alice", &server.url)],
        &[case()],
        &mut journal,
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(active.load(Ordering::SeqCst), 0);
    assert_eq!(peak.load(Ordering::SeqCst), 2);
    assert!((2..=8).contains(&journal.records.len()));
    assert!(journal.records.iter().all(|row| row.ok));
}

#[tokio::test]
async fn latest_cases_use_one_snapshot_and_its_voting_wallet_with_candidate_fallback() {
    const VOTE: &str = "1vgZrjS88D7RA1CbcSAovvyd6cSVqk3Ag1Ty2kSrJVd";
    const WALLET: &str = "5KjCzFvbCkRswE9x776udwrRXADbRiboNnmFQRhEHEuR";
    let rpc = Fixture::new(|mut request| {
        assert_eq!(request.method(), &tiny_http::Method::Post);
        let mut body = String::new();
        request.as_reader().read_to_string(&mut body).unwrap();
        let payload: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(payload["method"], "getVoteAccounts");
        assert_eq!(payload["params"][0]["commitment"], "finalized");
        request.respond(Response::from_string(serde_json::json!({"jsonrpc":"2.0", "id":1,
            "result":{"current":[
                {"votePubkey":"11111111111111111111111111111111", "activatedStake":200},
                {"votePubkey":VOTE, "activatedStake":100, "nodePubkey":"11111111111111111111111111111111"}],
                "delinquent":[]}}).to_string())).unwrap();
    });
    let upstream = Fixture::new(|request| {
        let path = request.url();
        let (status, body) = if path == "/meta?network=testnet" {
            (200, serde_json::json!({"network":"testnet", "slot":42}))
        } else if path == format!("/proof/vote_account/{VOTE}?network=testnet&slot=42") {
            (
                200,
                serde_json::json!({"network":"testnet", "snapshot_slot":42,
                "meta_merkle_leaf":{"vote_account":VOTE, "voting_wallet":WALLET}, "meta_merkle_proof":[]}),
            )
        } else if path == format!("/voter/{WALLET}?network=testnet&slot=42") {
            (
                200,
                serde_json::json!({"network":"testnet", "snapshot_slot":42, "voting_wallet":WALLET,
                "vote_accounts":[{"vote_account":VOTE}], "stake_accounts":[]}),
            )
        } else {
            (404, Value::Null)
        };
        request
            .respond(Response::from_string(body.to_string()).with_status_code(status))
            .unwrap();
    });
    let upstream_url = upstream.url.clone();
    let router = Fixture::new(move |request| {
        let location = format!("{}{}", upstream_url, request.url().trim_start_matches('/'));
        request
            .respond(
                Response::empty(302).with_header(Header::from_bytes("Location", location).unwrap()),
            )
            .unwrap();
    });
    let directory = tempfile::tempdir().unwrap();
    let mut args = args(&directory);
    args.network = config::Network::Testnet;
    args.rpc_url = Some(format!("{}private?api-key=fixture-key", rpc.url));
    args.router_url = router.url.clone();
    args.verifiers = directory.path().join("verifiers.toml");
    args.requests = directory.path().join("requests.toml");
    fs::write(
        &args.verifiers,
        format!(
            "[[verifiers]]\nname='alice'\nverification_domain='{}'\n",
            upstream.url
        ),
    )
    .unwrap();
    fs::write(&args.requests, "[[requests]]\nname='vote_proof'\npath='latest'\n\n[[requests]]\nname='voter'\npath='latest'\n").unwrap();
    assert!(run(args.clone()).await.unwrap());
    let report_text = fs::read_to_string(args.output.join("report.json")).unwrap();
    let report: Value = serde_json::from_str(&report_text).unwrap();
    assert_eq!(report["slot"], 42);
    assert_eq!(
        report["workload"][0]["path"],
        format!("/proof/vote_account/{VOTE}?network=testnet&slot=42")
    );
    assert_eq!(
        report["workload"][1]["path"],
        format!("/voter/{WALLET}?network=testnet&slot=42")
    );
    let journal = fs::read_to_string(args.output.join("requests.jsonl")).unwrap();
    assert!(!report_text.contains("fixture-key"));
    assert!(!journal.contains("fixture-key"));
    let rows: Vec<Value> = journal
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(rows
        .iter()
        .any(|row| row["phase"] == "account_discovery" && row["status"] == 404));
    assert_eq!(rpc.paths.lock().unwrap().len(), 1);
    assert!(rows
        .iter()
        .filter(|row| row["phase"] == "load")
        .all(|row| row["ok"] == true));
}

#[tokio::test]
async fn voter_latest_works_alone_and_explicit_proof_paths_are_preserved() {
    const VOTE: &str = "1vgZrjS88D7RA1CbcSAovvyd6cSVqk3Ag1Ty2kSrJVd";
    const WALLET: &str = "5KjCzFvbCkRswE9x776udwrRXADbRiboNnmFQRhEHEuR";
    let rpc = Fixture::new(|request| {
        request
            .respond(Response::from_string(
                serde_json::json!({"result":{"current":[], "delinquent":[
            {"votePubkey":VOTE, "activatedStake":100}]}})
                .to_string(),
            ))
            .unwrap();
    });
    let upstream = Fixture::new(|request| {
        assert!(request.url().ends_with("network=mainnet&slot=7"));
        request.respond(Response::from_string(serde_json::json!({"network":"mainnet", "snapshot_slot":7,
            "meta_merkle_leaf":{"vote_account":VOTE, "voting_wallet":WALLET}, "meta_merkle_proof":[]}).to_string())).unwrap();
    });
    let directory = tempfile::tempdir().unwrap();
    let mut args = args(&directory);
    args.rpc_url = Some(rpc.url.clone());
    let explicit_path = "/proof/vote_account/EXPLICIT?network={network}&slot={slot}";
    let mut cases = [
        Case {
            name: "voter".into(),
            path: "latest".into(),
            ..case()
        },
        Case {
            name: "vote_proof".into(),
            path: explicit_path.into(),
            ..case()
        },
    ];
    let mut journal = Journal::new(&args).unwrap();
    resolve::resolve(
        &args,
        &client(1.0),
        &[verifier("alice", &upstream.url)],
        &mut cases,
        7,
        &mut journal,
    )
    .await
    .unwrap();
    assert_eq!(
        cases[0].path,
        format!("/voter/{WALLET}?network=mainnet&slot=7")
    );
    assert_eq!(
        cases[1].path,
        "/proof/vote_account/EXPLICIT?network=mainnet&slot=7"
    );
}

#[tokio::test]
async fn rpc_errors_and_missing_snapshot_accounts_fail_latest_discovery_with_evidence() {
    let missing = Fixture::response(404, "missing");
    let rpc_error = Fixture::response(
        200,
        r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32005,"message":"unavailable"}}"#,
    );
    let rpc_accounts = Fixture::response(
        200,
        r#"{"result":{"current":[{"votePubkey":"11111111111111111111111111111111","activatedStake":1}],"delinquent":[]}}"#,
    );
    for rpc in [&rpc_error, &rpc_accounts] {
        let directory = tempfile::tempdir().unwrap();
        let mut args = args(&directory);
        args.rpc_url = Some(rpc.url.clone());
        let mut cases = [Case {
            name: "vote_proof".into(),
            path: "latest".into(),
            ..case()
        }];
        let mut journal = Journal::new(&args).unwrap();
        assert!(resolve::resolve(
            &args,
            &client(1.0),
            &[verifier("alice", &missing.url)],
            &mut cases,
            42,
            &mut journal
        )
        .await
        .is_err());
        assert!(!journal.records.is_empty());
        assert!(journal.records[0].response_file.is_some());
        assert_eq!(cases[0].path, "latest");
    }
}

#[test]
fn router_budget_bounds_the_chance_of_missing_any_verifier() {
    let budget = sampling::required_samples(10, 0.999);
    assert_eq!(budget, 88);
    assert!(sampling::miss_probability_bound(10, budget) <= 0.001);
    assert!(sampling::miss_probability_bound(10, budget - 1) > 0.001);
    assert_eq!(sampling::required_samples(1, 0.999), 1);
}

fn rotating_router(origins: Vec<String>) -> Fixture {
    let selected = AtomicUsize::new(0);
    Fixture::new(move |request| {
        let origin = &origins[selected.fetch_add(1, Ordering::SeqCst) % origins.len()];
        let location = format!("{}{}", origin, request.url().trim_start_matches('/'));
        request
            .respond(
                Response::empty(302).with_header(Header::from_bytes("Location", location).unwrap()),
            )
            .unwrap();
    })
}

#[tokio::test]
async fn router_sampling_tops_up_existing_selections_and_covers_every_verifier() {
    let upstreams: Vec<_> = (0..3).map(|_| Fixture::response(200, META)).collect();
    let router = rotating_router(upstreams.iter().map(|server| server.url.clone()).collect());
    let mut targets = vec![Target {
        name: "router".into(),
        url: router.url.clone(),
        kind: Kind::Router,
    }];
    targets.extend(
        upstreams
            .iter()
            .enumerate()
            .map(|(index, server)| verifier(&format!("verifier-{index}"), &server.url)),
    );
    let directory = tempfile::tempdir().unwrap();
    let mut args = args(&directory);
    args.rps = 1000.0;
    let client = client(1.0);
    let mut journal = Journal::new(&args).unwrap();
    for _ in 0..2 {
        journal
            .consume(fetch(&client, &targets[0], &case(), &targets[1..], "load").await)
            .unwrap();
    }
    sample_router(
        &args,
        &client,
        &targets,
        42,
        &mut journal,
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    let required = sampling::required_samples(3, args.router_confidence);
    assert_eq!(journal.records.len(), required);
    assert_eq!(
        journal
            .records
            .iter()
            .filter(|record| record.phase == "router_sampling")
            .count(),
        required - 2
    );
    let mut report = Report::new(args);
    let cases = [Case {
        targets: vec![Kind::Router],
        ..case()
    }];
    report.analyze(&journal.records, &targets, &cases);
    assert!(report.passed, "{:?}", report.findings);
    assert_eq!(report.load_requests, 2);
    assert_eq!(report.router_attributed_samples, required);
    assert_eq!(report.router_selections.len(), 3);
    assert!(report.router_selections.values().all(|count| *count > 0));
    let markdown = report.markdown();
    assert!(markdown.find("## Router selections").unwrap() < markdown.find("## Request:").unwrap());
}

#[tokio::test]
async fn a_router_excluding_healthy_configured_verifiers_fails_with_one_error_per_miss() {
    let upstreams: Vec<_> = (0..3).map(|_| Fixture::response(200, META)).collect();
    let router = rotating_router(vec![upstreams[0].url.clone()]);
    let mut targets = vec![Target {
        name: "router".into(),
        url: router.url.clone(),
        kind: Kind::Router,
    }];
    targets.extend(
        upstreams
            .iter()
            .enumerate()
            .map(|(index, server)| verifier(&format!("verifier-{index}"), &server.url)),
    );
    let directory = tempfile::tempdir().unwrap();
    let mut args = args(&directory);
    args.rps = 1000.0;
    let client = client(1.0);
    let mut journal = Journal::new(&args).unwrap();
    for target in &targets {
        journal
            .consume(fetch(&client, target, &case(), &targets[1..], "load").await)
            .unwrap();
    }
    sample_router(
        &args,
        &client,
        &targets,
        42,
        &mut journal,
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(journal.records.iter().all(|record| record.ok));
    let mut report = Report::new(args);
    report.analyze(&journal.records, &targets, &[case()]);
    assert!(!report.passed);
    assert_eq!(report.findings.len(), 2);
    for target in ["verifier-1", "verifier-2"] {
        assert!(report
            .findings
            .iter()
            .any(|finding| finding["type"] == "router_verifier_not_selected"
                && finding["target"] == target));
    }
    let markdown = report.markdown();
    assert!(markdown.contains("verifier-1: 0 (ERROR: never selected)"));
    let findings: Vec<Value> = markdown
        .split("```json\n")
        .skip(1)
        .map(|block| serde_json::from_str(block.split("\n```").next().unwrap()).unwrap())
        .collect();
    assert_eq!(findings.len(), 2);
}

#[tokio::test]
async fn unattributed_router_responses_have_a_bounded_budget_and_report_insufficient_samples() {
    let upstream = Fixture::response(200, META);
    let router = Fixture::response(200, META);
    let targets = [
        Target {
            name: "router".into(),
            url: router.url.clone(),
            kind: Kind::Router,
        },
        verifier("alice", &upstream.url),
    ];
    let directory = tempfile::tempdir().unwrap();
    let mut args = args(&directory);
    args.rps = 1000.0;
    let mut journal = Journal::new(&args).unwrap();
    sample_router(
        &args,
        &client(1.0),
        &targets,
        42,
        &mut journal,
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(journal.records.len(), 1);
    let mut report = Report::new(args);
    report.analyze(&journal.records, &targets, &[]);
    assert!(!report.passed);
    assert_eq!(report.router_attributed_samples, 0);
    assert_eq!(report.router_attempts, 1);
    assert!(report
        .findings
        .iter()
        .any(|finding| finding["type"] == "insufficient_router_samples"));
    assert!(report
        .findings
        .iter()
        .any(|finding| finding["type"] == "router_verifier_not_selected"
            && finding["target"] == "alice"));
}

#[tokio::test]
async fn reruns_replace_response_evidence_without_removing_other_output_files() {
    let directory = tempfile::tempdir().unwrap();
    let args = args(&directory);
    let notes = args.output.join("notes.txt");
    fs::write(&notes, "keep these notes").unwrap();
    let mut previous_response: Option<std::path::PathBuf> = None;
    let client = client(1.0);
    for body in [META.to_owned(), META.replace("root", "next-root")] {
        let mut journal = Journal::new(&args).unwrap();
        if let Some(previous) = previous_response {
            assert!(!previous.exists());
        }
        assert!(fs::read_to_string(args.output.join("requests.jsonl"))
            .unwrap()
            .is_empty());
        let response = body.clone();
        let server = Fixture::new(move |request| {
            request.respond(Response::from_string(&response)).unwrap();
        });
        let target = verifier("alice", &server.url);
        let record = fetch(&client, &target, &case(), &[], "load").await;
        assert!(record.ok, "{:?}", record.issues);
        journal.consume(record).unwrap();
        let evidence = args
            .output
            .join(journal.records[0].response_file.as_ref().unwrap());
        assert_eq!(fs::read_to_string(&evidence).unwrap(), body);
        assert_eq!(
            fs::read_dir(args.output.join("responses")).unwrap().count(),
            1
        );
        assert_eq!(
            fs::read_to_string(args.output.join("requests.jsonl"))
                .unwrap()
                .lines()
                .count(),
            1
        );
        assert_eq!(fs::read_to_string(&notes).unwrap(), "keep these notes");
        previous_response = Some(evidence);
    }
    let journal = Journal::new(&args).unwrap();
    assert!(journal.records.is_empty());
    assert_eq!(
        fs::read_dir(args.output.join("responses")).unwrap().count(),
        0
    );
    assert_eq!(fs::read_to_string(&notes).unwrap(), "keep these notes");
}

#[tokio::test]
async fn router_failure_details_are_complete_linked_and_cleared_after_a_clean_run() {
    let upstream = Fixture::response(503, "unavailable");
    let router = rotating_router(vec![upstream.url.clone()]);
    let targets = [
        Target {
            name: "router".into(),
            url: router.url.clone(),
            kind: Kind::Router,
        },
        verifier("alice", &upstream.url),
    ];
    let directory = tempfile::tempdir().unwrap();
    let args = args(&directory);
    let client = client(1.0);
    let mut journal = Journal::new(&args).unwrap();
    for index in 0..55 {
        let phase = if index % 2 == 0 {
            "load"
        } else {
            "router_sampling"
        };
        journal
            .consume(fetch(&client, &targets[0], &case(), &targets[1..], phase).await)
            .unwrap();
    }
    journal
        .consume(fetch(&client, &targets[1], &case(), &targets[1..], "load").await)
        .unwrap();
    let mut report = Report::new(args.clone());
    report.analyze(&journal.records, &targets, &[]);
    report.write(&journal.records).unwrap();
    let details_path = args.output.join("router-failures.json");
    let details: Value = serde_json::from_slice(&fs::read(&details_path).unwrap()).unwrap();
    assert_eq!(details["total"], 55);
    assert_eq!(details["failures"].as_array().unwrap().len(), 55);
    assert_eq!(details["failures"][54]["id"], 55);
    assert_eq!(
        details["by_status"][0],
        serde_json::json!({"status":503, "count":55})
    );
    assert_eq!(
        details["by_verifier"][0],
        serde_json::json!({"verifier":"alice", "count":55})
    );
    assert_eq!(details["failures"][0]["hops"][0]["status"], 302);
    assert_eq!(details["failures"][0]["hops"][1]["status"], 503);
    assert!(args
        .output
        .join(details["failures"][0]["response_file"].as_str().unwrap())
        .exists());
    let summary = fs::read_to_string(args.output.join("summary.md")).unwrap();
    let router_section = summary
        .split("## Router selections")
        .nth(1)
        .unwrap()
        .split("## Findings")
        .next()
        .unwrap();
    assert!(router_section
        .contains("[Router failure details (55 requests, JSON)](router-failures.json)"));

    let mut clean_report = Report::new(args.clone());
    clean_report.analyze(&[], &[], &[]);
    clean_report.write(&[]).unwrap();
    assert!(!details_path.exists());
    assert!(!fs::read_to_string(args.output.join("summary.md"))
        .unwrap()
        .contains("router-failures.json"));
}
