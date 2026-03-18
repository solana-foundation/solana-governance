pub mod constants;
pub mod migrator;
pub mod models;
pub mod operations;
pub mod sql;
mod path;

use crate::utils::env_parse;
use anyhow::Result;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use sqlx::ConnectOptions;
use std::{fs, path::Path, str::FromStr};
use tracing::info;

pub use migrator::run_migrations;
use self::path::validate_db_path;

/// Create a new SQLx pool and run migrations
pub async fn init_pool(db_path: &str) -> Result<SqlitePool> {
    info!("Opening database at {:?}", db_path);

    validate_db_path(db_path)?;

    // Ensure parent directory exists
    if db_path != ":memory:" {
        let path = Path::new(db_path);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
    }

    // Build connect options and pool options
    let (url, default_max_connections) = if db_path == ":memory:" {
        // Shared in-memory DB; keep a single connection for simplicity
        (
            "sqlite:file:memdb?mode=memory&cache=shared".to_string(),
            1u32,
        )
    } else {
        (format!("sqlite:{}", db_path), 4u32)
    };

    let connect_options = SqliteConnectOptions::from_str(&url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .disable_statement_logging()
        .pragma("busy_timeout", "5000");

    let max_conns = env_parse::<u32>("SQLITE_MAX_CONNECTIONS", default_max_connections).max(1);

    let pool = SqlitePoolOptions::new()
        .max_connections(max_conns)
        .connect_with(connect_options)
        .await?;

    // Run migrations
    run_migrations(&pool).await?;

    info!("Database pool initialized successfully");
    Ok(pool)
}
