# NCN router and verifier load test

From the repository root, with Rust 1.93.1+ and Just 1.29+ installed:

```bash
just ncn-network-loadtest
```

This sends paced GET requests to `https://ncn-governance.solana.com` and every
verifier in [`../config.toml`](../config.toml). The default load phase lasts
90 seconds, at two logical requests per second in total, with up to eight
requests in flight. It needs no local database or credentials. Automatic account
discovery uses the selected network's public Solana RPC; a corpus containing only
explicit paths does not use RPC.
The Rust binary is built on the first run. You can also invoke it directly:

```bash
cargo run --locked --manifest-path ncn-router/Cargo.toml --bin ncn-network-loadtest -- --help
```

On Linux, compiling requires `pkg-config` and OpenSSL development headers
(`sudo apt-get install pkg-config libssl-dev` on Ubuntu/Debian).

The terminal prints a report beginning with `Result: **PASS**` or
`Result: **FAIL**`. Exit code 0 means all checks passed; exit code 1 means the
report contains findings. Invalid CLI arguments exit with code 2.

## Reports

The runner writes `loadtest-results/` at the repository root:

- `summary.md`: router selections first, then one comparison table per request
  type with per-target latency percentiles, and findings as JSON code blocks.
- `report.json`: configuration, resolved workload, selected snapshot, request
  counts, HTTP status counts, latency comparisons, and all findings.
- `router-failures.json`: automatically created when routed requests fail, with
  all failed router request records and counts by HTTP status and verifier.
  It includes timestamps, URLs, redirect hops, timings, and response evidence paths,
  covering both load and additional sampling. The `Router selections` section
  links to this file. It has no 50-finding display limit and is removed on a run
  with no failed router requests.
- `requests.jsonl`: one record per logical request, including discovery.
  Records contain a request ID, timestamp, path, intended target, resolved
  verifier, each redirect and its timing, final status, total latency,
  expected status/format/body, comparison hash, response evidence path,
  and errors.
  Discovery records also identify the HTTP method; the RPC lookup uses POST.
- `responses/`: response bodies addressed by their raw body hash. Records with
  different comparison hashes can be inspected here to see the actual differences.

`request_failed` findings link directly to journal IDs. `response_mismatch`
findings identify the rendered request and expected response settings, list every
case name in the comparison, and list each comparison hash with the request IDs
that returned it. Responses are grouped by method, rendered path/query, expected
status, response format, and expected body, regardless of case name or load/sampling
phase. Per-case request counts and latency tables include only the load phase.
Differences are reported without assuming that the largest response group is correct.
Reports are overwritten on each run; use a different `--output` to retain runs.

## What is checked

The runner first queries each verifier's latest `/meta?network=mainnet`. It pins
the load phase to the most common latest slot, choosing the highest slot on a
tie. Pass `--slot` to skip discovery and use a particular snapshot. Discovery
failures remain findings even if a verifier recovers during the load phase.

The default [request corpus](requests.toml) tests `/meta`, `/metas`, `/healthz`,
a vote proof, and a voter summary. Data cases are sent through the router and directly to every configured
verifier in a repeating cycle. Health probes go directly to verifiers: production
nginx serves the router's own `/healthz` without selecting a verifier.
Direct probes cover verifiers that the router
does not select or has excluded from its whitelist. Configured verifiers are
treated as expected members of the network; an excluded verifier can therefore
fail this test even when router requests all succeed.

The runner reports:

- Request timeouts, connection/TLS errors, malformed JSON, oversized responses,
  and unexpected HTTP statuses, including 404, 429, and 5xx. It does not retry or
  hide failed attempts.
- Router selections outside the configured verifier list, missing attribution,
  and redirects that change the requested path or query.
- Configured verifiers never selected by the router, even when direct probes
  succeed, and insufficient samples to meet the router coverage target.
- Metadata, proofs, or voter summaries describing a different network or slot,
  empty batch metadata, and missing proof/voter response structures. Health
  responses must contain exactly `ok`.
- Different successful response bodies for the same case, across direct and
  routed requests and across repeated requests to the same target. JSON object
  key order is ignored; array order is preserved because proof order matters.
  Only the top-level `created_at` field in `/meta` and each `/metas` record is
  ignored: it describes each verifier's local upload time.
- Verifiers whose p50 latency exceeds both three times the median of other
  verifiers' p50 values and that peer median plus 250 ms, for the same case.
  Both the verifier and its peers need at least three successful samples.
  Insufficient samples or peers are shown explicitly in the report. Router
  timing is excluded from these comparisons.
- Missing direct or router coverage for a case when the run was too short to
  issue every combination. Increase the duration or rate to cover a larger corpus.

The test compares proof responses and checks their structure; it does not
cryptographically verify Merkle proofs or check snapshots against on-chain ballots.
The default corpus discovers its vote account and voter wallet automatically.

Redirect-mode selections come from the router's `Location`. Proxy mode requires
the router's `X-NCN-Verifier` response header, which also identifies the selected
verifier on upstream failures. Deploy the accompanying router change before testing
a proxy-mode router. A configured verifier with zero router selections fails
the run; direct probes still provide independent verifier checks.

## Router selection coverage

The runner sizes its router sample budget for a 99.9% probability of selecting
every configured verifier at least once, assuming independent, uniform choices.
For `N` configured verifier origins, the probability of missing any verifier after
`M` selections is at most `N * (1 - 1/N)^M`. It chooses the smallest integer `M`
that makes this bound no greater than `1 - router_confidence`.

With ten verifiers, the default budget is **88 attributed router selections**.
Selections from the normal load phase count toward this budget. After load ends,
the runner sends enough additional pinned `/meta` requests through the router to
reach it, using the same rate and concurrency limits. These additional requests
have phase `router_sampling` in the journal and do not change the per-case load
tables or load-phase latency measurements. Their failures and response differences
are still included in the findings.

Every configured verifier with zero selections gets a
`router_verifier_not_selected` error. Timeouts after an identified selection count
as a selection and also remain request failures. Responses with no identifiable
configured verifier do not count; if these leave the budget incomplete, the report
includes `insufficient_router_samples`. Additional attempts are bounded by the
budget, so an unavailable router cannot cause an endless sampling loop.

The summary lists required/actual samples, failures, and the time spent on extra
sampling above the request tables. `report.json` also records the probability
bound for the observed sample count. Override the target with
`--router-confidence`, a number strictly between zero and one.

At two requests/s, a ten-verifier run with fourteen normal router selections adds
about 37 seconds of router sampling. The `--duration` option controls the mixed
load phase; extra router sampling and discovery add to total run time.

## Increase load or choose a network

```bash
just ncn-network-loadtest --network testnet --duration 300 --rps 20 --concurrency 16
just ncn-network-loadtest --slot 340850340 --output /tmp/ncn-historical-run
just ncn-network-loadtest --router-url http://localhost:8080 --verifiers /tmp/verifiers.toml
```

The historical slot is an example; use one retained by your verifiers. A custom
verifier file uses the same `[[verifiers]]`, `name`, and `verification_domain`
fields as the router configuration. Duplicate base URLs are tested once.

The requested rate is shared across all logical requests, including router
and direct probes. With ten verifiers, each data case sends 1/11 of its
requests through the router. The default corpus sends four router requests per
54-request cycle because health probes go directly to verifiers. The default
90-second duration allows at least three samples for every case and target at the
default rate when requests complete without sustained backpressure. Redirects add
HTTP requests to the logical request count. Discovery
also respects the configured rate. The runner applies backpressure at the
concurrency limit and reports the achieved rate rather than building a backlog.
Load-phase elapsed time includes draining in-flight requests. Discovery and
additional router sampling have separate timing.

The async HTTP client pools connections. Timing includes network, any TLS setup,
response body reads, and redirects. Each HTTP hop has a deadline covering the
request and its response body. Responses are limited to 2 MiB and five redirects.
Press Ctrl-C to stop issuing load and drain in-flight requests before writing a
report marked as interrupted.

## Choose vote accounts and voter wallets automatically

Use `path = "latest"` for a case named `vote_proof` or `voter`:

```toml
[[requests]]
name = "vote_proof"
path = "latest"

[[requests]]
name = "voter"
path = "latest"
```

The runner pins the snapshot slot using network metadata, then calls
[`getVoteAccounts`](https://solana.com/docs/rpc/http/getvoteaccounts) on that network's RPC.
It tries staked vote accounts in descending stake order until a configured verifier
returns a matching proof for the selected snapshot. The voter wallet comes from
that proof's `meta_merkle_leaf.voting_wallet`, which can differ from the validator's
current identity or authority. A voter case can use `latest` without a vote-proof case.

Both cases resolve once before load starts. Every router and verifier then receives
the same account, wallet, network, and pinned slot, recorded as concrete paths in
`report.json`'s `workload`. Passing `--slot` overrides the snapshot used for this
discovery, including for historical tests; it does not force the newest snapshot.

Discovery tries up to ten candidate accounts and twenty proof probes. A candidate
may be absent from the snapshot: its 404 is retained in the journal but is not a
failure finding. Other discovery failures remain findings. If no candidate has a
usable proof, the runner writes a setup failure and does not start load.
RPC responses are limited to 16 MiB; verifier responses retain the 2 MiB limit.

The RPC defaults to `https://api.mainnet-beta.solana.com` or
`https://api.testnet.solana.com`, according to `--network`. Override it with
`--rpc-url` or `NCN_LOADTEST_RPC_URL`. RPC credentials in paths or query strings
are omitted from the configuration report and journal URLs. A secret RPC URL can
be supplied through Doppler's environment injection.

## Specify voter and proof accounts explicitly

Copy `requests.toml` to a new TOML file and add accounts present in the snapshot:

```toml
[[requests]]
name = "vote_proof"
path = "/proof/vote_account/1vgZrjS88D7RA1CbcSAovvyd6cSVqk3Ag1Ty2kSrJVd?network={network}&slot={slot}"

[[requests]]
name = "voter"
path = "/voter/5KjCzFvbCkRswE9x776udwrRXADbRiboNnmFQRhEHEuR?network={network}&slot={slot}"
```

These IDs come from the verifier service's testnet example; replace them with
accounts in your chosen snapshot. Stake proof paths use
`/proof/stake_account/ACCOUNT?network={network}&slot={slot}`.

```bash
just ncn-network-loadtest --network testnet --requests /tmp/proof-requests.toml
```

Each `[[requests]]` entry accepts:

| Field             | Type         | Default                  | Meaning                                                                           |
| ----------------- | ------------ | ------------------------ | --------------------------------------------------------------------------------- |
| `name`            | string       | required                 | Unique case name used for grouping and reports                                    |
| `path`            | string       | required                 | Explicit path with `{network}` and `{slot}`, or `latest` for `vote_proof`/`voter` |
| `expected_status` | integer      | `200`                    | Expected final HTTP status                                                        |
| `format`          | string       | `json`                   | `json` for semantic comparisons, `text` for exact bytes, or `status` to compare only the expected status |
| `expected_body`   | string       | unset                    | Optional exact UTF-8 body assertion                                               |
| `targets`         | string array | `["router", "verifier"]` | Send through the router, directly to all verifiers, or both                       |

Data paths must include the selected network and pinned slot. `/metas` uses
`slots={slot}`. Use distinct case names for different requests, including negative
cases with an expected non-200 status. `latest` cases must use JSON format and
expect HTTP 200; explicit paths retain their normal format/status options.

Use `format = "status"` for a negative test whose error body may differ between
verifiers or contain plain text, HTML, or no content:

```toml
[[requests]]
name = "missing_vote_proof"
path = "/proof/vote_account/ABSENT_ACCOUNT?network={network}&slot={slot}"
expected_status = 404
format = "status"
```

Status comparisons hash the expected status after checking that the actual status
matches it. They skip body parsing and body comparisons, but still read and save
the response under the normal size and timeout limits. An explicit `expected_body`
continues to assert the exact body. Unexpected statuses remain failures in every
format. JSON and text cases, including those expecting 201 or error statuses,
continue to compare bodies.

## GitHub Actions

The [NCN network load test workflow](../../.github/workflows/ncn-network-loadtest.yaml)
runs daily at 10:00 UTC and supports **Run workflow** with URL, network,
duration, rate, concurrency, router coverage confidence, slot, RPC URL, and corpus inputs. It publishes the Markdown
summary to the Actions run and uploads all evidence for 30 days, including when
the load test fails. In the Actions summary, the router failure details link opens
the report artifact containing `router-failures.json`; in the downloaded summary,
the link opens the adjacent JSON file. Jobs fail on any finding. A concurrency group prevents
overlapping live runs; a 15-minute job timeout bounds CI execution.

Merge the workflow to the default branch to activate scheduled execution.
GitHub schedules can run late and public repositories disable schedules after
60 days without activity. See [GitHub's schedule documentation](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#schedule).

To change scheduled defaults, set repository Actions variables:

- `NCN_LOADTEST_ROUTER_URL`, `NCN_LOADTEST_NETWORK`
- `NCN_LOADTEST_DURATION`, `NCN_LOADTEST_RPS`, `NCN_LOADTEST_CONCURRENCY`
- `NCN_LOADTEST_ROUTER_CONFIDENCE`: router selection coverage target (default `0.999`)
- `NCN_LOADTEST_REQUESTS`: path to a committed TOML corpus
- `NCN_LOADTEST_RPC_URL`: optional public RPC override for automatic account discovery

Manual inputs take precedence over variables. Edit the workflow's cron to
change the cadence. The workflow uses public read endpoints and needs no secrets.

## CLI options

All options are optional. Paths supplied on the CLI are relative to the current
directory; built-in paths resolve from the repository path used to build the binary.

| Option                | Type                   | Default                             | Purpose                                                              |
| --------------------- | ---------------------- | ----------------------------------- | -------------------------------------------------------------------- |
| `--router-url`        | URL                    | `https://ncn-governance.solana.com` | Router to exercise                                                   |
| `--verifiers`         | path                   | `ncn-router/config.toml`            | Expected verifier list                                               |
| `--requests`          | path                   | `ncn-router/loadtest/requests.toml` | TOML request corpus                                                  |
| `--network`           | enum                   | `mainnet`                           | `mainnet` or `testnet`                                               |
| `--rpc-url`           | URL                    | public RPC for the network          | Account discovery RPC; overrides `NCN_LOADTEST_RPC_URL`              |
| `--slot`              | positive integer       | discovered                          | Snapshot to compare                                                  |
| `--duration`          | positive seconds       | `90`                                | Load issuance duration                                               |
| `--rps`               | positive number        | `2`                                 | Maximum logical requests per second                                  |
| `--concurrency`       | positive integer       | `8`                                 | Maximum logical requests in flight                                   |
| `--router-confidence` | number between 0 and 1 | `0.999`                             | Target probability of selecting every verifier under uniform routing |
| `--timeout`           | positive seconds       | `10`                                | Request and body deadline per hop                                    |
| `--slow-factor`       | positive number        | `3`                                 | Relative p50 outlier threshold                                       |
| `--slow-ms`           | positive milliseconds  | `250`                               | Absolute p50 excess required                                         |
| `--min-samples`       | positive integer       | `3`                                 | Successful samples per verifier/case for comparisons                 |
| `--output`            | path                   | `loadtest-results`                  | Report destination                                                   |

Run the local fixture suite without contacting the live network:

```bash
just ncn-network-loadtest-test
```

The regular PR CI runs this suite. The existing verifier-service `loadtest`
binary remains available for database-driven testing of one verifier.

Check Rust formatting and linting with `just ncn-network-loadtest-check`.
