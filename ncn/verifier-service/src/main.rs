mod database;
mod metrics;
mod middleware;
mod types;
mod upload;
mod utils;

use axum::{
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, StatusCode, Method},
    response::Json,
    routing::{get, post},
    Router,
};
use database::{constants::DEFAULT_DB_PATH, init_pool, models::*, operations::db_operation};
use serde_json::{json, Value};
use sqlx::sqlite::SqlitePool;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tower_http::cors::{CorsLayer, Any};
use tracing::{debug, info, Level};
use types::{NetworkQuery, VoterQuery};
use upload::handle_upload;

use crate::{
    middleware::inject_client_ip,
    utils::{env_parse, validate_network},
};

// Server configuration constants
const DEFAULT_BODY_LIMIT: usize = 100 * 1024 * 1024; // 100MB for uploads
const DEFAULT_PORT: u16 = 3000; // override with PORT env var
const DEFAULT_NETWORK: &str = "mainnet";

// Get the latest snapshot slot if not specified
async fn get_snapshot_slot(
    pool: &SqlitePool,
    network: &str,
    requested_slot: Option<u64>,
) -> Result<u64, StatusCode> {
    match requested_slot {
        Some(s) => Ok(s),
        None => db_operation(
            || SnapshotMetaRecord::get_latest_slot(pool, network),
            "Failed to get latest slot",
        )
        .await?
        .ok_or(StatusCode::NOT_FOUND),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Governance Merkle Verifier Service");

    // Check if OPERATOR_PUBKEY and METRICS_AUTH_TOKEN are set
    let operator_pubkey = std::env::var("OPERATOR_PUBKEY");
    let metrics_auth_token = std::env::var("METRICS_AUTH_TOKEN");
    if operator_pubkey.is_err() || metrics_auth_token.is_err() {
        anyhow::bail!("OPERATOR_PUBKEY or METRICS_AUTH_TOKEN is not set");
    }

    // Initialize database pool (create tables, run migrations)
    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| DEFAULT_DB_PATH.to_string());
    let pool = init_pool(&db_path).await?;
    info!("Database initialized successfully");

    // Build application with routes
    let app = {
        // Helper for rate limiter configs
        let rl = |per_second: u64, burst_size: u32| {
            Arc::new(
                GovernorConfigBuilder::default()
                    .per_second(per_second)
                    .burst_size(burst_size)
                    .finish()
                    .expect("valid rate limiter config"),
            )
        };

        // Rate limits configurable via environment variables with sane defaults
        let global_per_second: u64 = env_parse("GLOBAL_REFILL_INTERVAL", 10);
        let global_burst: u32 = env_parse("GLOBAL_RATE_BURST", 10);
        let upload_per_second: u64 = env_parse("UPLOAD_REFILL_INTERVAL", 60);
        let upload_burst: u32 = env_parse("UPLOAD_RATE_BURST", 2);

        let global_rl = rl(global_per_second, global_burst);
        let upload_rl = rl(upload_per_second, upload_burst);

        let body_limit = env_parse::<u64>("UPLOAD_BODY_LIMIT", DEFAULT_BODY_LIMIT as u64) as usize;

        // CORS: Allowed for public endpoints only
        let public_cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::OPTIONS]);

        let upload_router = Router::new()
            .route("/", post(handle_upload))
            .layer(DefaultBodyLimit::max(body_limit))
            .layer(axum::middleware::from_fn(inject_client_ip))
            .layer(GovernorLayer { config: upload_rl });

        let public_router = Router::new()
            .route("/healthz", get(health_check))
            .route("/version", get(get_version))
            .route("/meta", get(get_meta))
            .route("/voter/{voting_wallet}", get(get_voter_summary))
            .route("/proof/vote_account/{vote_account}", get(get_vote_proof))
            .route("/proof/stake_account/{stake_account}", get(get_stake_proof))
            .layer(public_cors);

        Router::new()
            .merge(public_router)
            .nest("/upload", upload_router)
            .route("/admin/stats", get(admin_stats))
            .layer(axum::middleware::from_fn(inject_client_ip))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                    .on_response(DefaultOnResponse::new().level(Level::INFO)),
            )
            .layer(GovernorLayer { config: global_rl })
            .with_state(pool.clone())
    }
    .into_make_service_with_connect_info::<SocketAddr>(); // Include socket address for rate limiting

    // Run the server
    let port: u16 = env_parse("PORT", DEFAULT_PORT);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "ok"
}

async fn get_version() -> Json<serde_json::Value> {
    let name = env!("CARGO_PKG_NAME");
    let version = env!("CARGO_PKG_VERSION");
    let git_hash = option_env!("VERIFIER_BUILD_GIT_HASH").unwrap_or("unknown");
    let build_time = option_env!("VERIFIER_BUILD_TIME_UNIX").unwrap_or("unknown");

    Json(json!({
        "name": name,
        "version": version,
        "git_hash": git_hash,
        "build_time_unix": build_time,
    }))
}

async fn admin_stats(headers: HeaderMap) -> Result<Json<serde_json::Value>, StatusCode> {
    let expected = std::env::var("METRICS_AUTH_TOKEN").ok();
    let provided = headers
        .get("x-metrics-token")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    match (expected, provided) {
        (Some(exp), Some(got)) if got == exp => Ok(Json(metrics::snapshot_as_json())),
        (Some(_), _) => Err(StatusCode::UNAUTHORIZED),
        (None, _) => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

async fn get_meta(
    State(pool): State<SqlitePool>,
    Query(params): Query<NetworkQuery>,
) -> Result<Json<SnapshotMetaRecord>, StatusCode> {
    let network = params.network.as_deref().unwrap_or(DEFAULT_NETWORK);
    validate_network(network)?;

    let meta_record_option = db_operation(
        || SnapshotMetaRecord::get_latest(&pool, network),
        "Failed to get snapshot meta record",
    )
    .await?;

    if let Some(record) = meta_record_option {
        Ok(Json(record))
    } else {
        info!("No snapshots found for network: {}", network);
        Err(StatusCode::NOT_FOUND)
    }
}

async fn get_voter_summary(
    State(pool): State<SqlitePool>,
    Path(voting_wallet): Path<String>,
    Query(params): Query<VoterQuery>,
) -> Result<Json<Value>, StatusCode> {
    let network = params.network.as_deref().unwrap_or(DEFAULT_NETWORK);
    validate_network(network)?;

    let snapshot_slot = params.slot;

    // Get vote account summaries
    let vote_accounts = db_operation(
        || {
            VoteAccountRecord::get_summary_by_voting_wallet(
                &pool,
                network,
                &voting_wallet,
                snapshot_slot,
            )
        },
        "Failed to get vote accounts",
    )
    .await?;

    // Get stake account summaries
    let stake_accounts = db_operation(
        || {
            StakeAccountRecord::get_summary_by_voting_wallet(
                &pool,
                network,
                &voting_wallet,
                snapshot_slot,
            )
        },
        "Failed to get stake accounts",
    )
    .await?;

    debug!(
        "Found {} vote accounts and {} stake accounts for voting wallet {}",
        vote_accounts.len(),
        stake_accounts.len(),
        voting_wallet
    );

    Ok(Json(json!({
        "network": network,
        "snapshot_slot": snapshot_slot,
        "voting_wallet": voting_wallet,
        "vote_accounts": vote_accounts,
        "stake_accounts": stake_accounts
    })))
}

async fn get_vote_proof(
    State(pool): State<SqlitePool>,
    Path(vote_account): Path<String>,
    Query(params): Query<VoterQuery>,
) -> Result<Json<Value>, StatusCode> {
    let network = params.network.as_deref().unwrap_or(DEFAULT_NETWORK);
    validate_network(network)?;

    let snapshot_slot = params.slot;

    // Get vote account record from database
    let vote_record_option = db_operation(
        || VoteAccountRecord::get_by_account(&pool, network, &vote_account, snapshot_slot),
        "Failed to get vote account record",
    )
    .await?;

    if let Some(vote_record) = vote_record_option {
        let meta_merkle_leaf = json!({
            "voting_wallet": vote_record.voting_wallet,
            "vote_account": vote_record.vote_account,
            "stake_merkle_root": vote_record.stake_merkle_root,
            "active_stake": vote_record.active_stake
        });

        Ok(Json(json!({
            "network": network,
            "snapshot_slot": snapshot_slot,
            "meta_merkle_leaf": meta_merkle_leaf,
            "meta_merkle_proof": vote_record.meta_merkle_proof
        })))
    } else {
        info!(
            "Vote account {} not found for network {} at slot {}",
            vote_account, network, snapshot_slot
        );
        metrics::record_proofs_not_found(metrics::ProofKind::Vote);
        Err(StatusCode::NOT_FOUND)
    }
}

async fn get_stake_proof(
    State(pool): State<SqlitePool>,
    Path(stake_account): Path<String>,
    Query(params): Query<VoterQuery>,
) -> Result<Json<Value>, StatusCode> {
    let network = params.network.as_deref().unwrap_or(DEFAULT_NETWORK);
    validate_network(network)?;

    let snapshot_slot = params.slot;

    // Get stake account record from database
    let stake_record_option = db_operation(
        || StakeAccountRecord::get_by_account(&pool, network, &stake_account, snapshot_slot),
        "Failed to get stake account record",
    )
    .await?;

    if let Some(stake_record) = stake_record_option {
        let stake_merkle_leaf = json!({
            "voting_wallet": stake_record.voting_wallet,
            "stake_account": stake_record.stake_account,
            "active_stake": stake_record.active_stake
        });

        Ok(Json(json!({
            "network": network,
            "snapshot_slot": snapshot_slot,
            "stake_merkle_leaf": stake_merkle_leaf,
            "stake_merkle_proof": stake_record.stake_merkle_proof,
            "vote_account": stake_record.vote_account
        })))
    } else {
        info!(
            "Stake account {} not found for network {} at slot {}",
            stake_account, network, snapshot_slot
        );
        metrics::record_proofs_not_found(metrics::ProofKind::Stake);
        Err(StatusCode::NOT_FOUND)
    }
}
