pub mod constants;
pub mod migrator;
pub mod models;
pub mod operations;
mod path;
pub mod sql;

use crate::utils::env_parse;
use anyhow::Result;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use sqlx::ConnectOptions;
use std::{fs, path::Path, str::FromStr};
use tracing::info;

use self::path::validate_db_path;
pub use migrator::run_migrations;

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

    // Migrate on a dedicated pool, then open the serving pool against the final
    // schema. A connection established before `ALTER TABLE` keeps the column
    // metadata it saw at connect time, so `SELECT *` still yields the old column
    // set and looking up a newly added column by name indexes past the end of the
    // row, panicking the sqlx worker and turning the first read into a spurious
    // 404. Only connections opened after the migration see the new column.
    let migration_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(connect_options.clone())
        .await?;

    run_migrations(&migration_pool).await?;

    let pool = SqlitePoolOptions::new()
        .max_connections(max_conns)
        .connect_with(connect_options)
        .await?;

    // Closed only once the serving pool holds a connection: a shared in-memory
    // database lives exactly as long as one is open.
    migration_pool.close().await;

    info!("Database pool initialized successfully");
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::models::SnapshotMetaRecord;
    use crate::database::sql::{
        CREATE_MIGRATIONS_TABLE_SQL, CREATE_STAKE_ACCOUNTS_TABLE_SQL,
        CREATE_VOTE_ACCOUNTS_TABLE_SQL,
    };

    /// `snapshot_meta` exactly as v1 created it, before `total_active_stake`.
    const V1_SNAPSHOT_META_TABLE_SQL: &str = r#"
CREATE TABLE snapshot_meta (
    network TEXT NOT NULL,
    slot INTEGER NOT NULL,
    merkle_root TEXT NOT NULL,
    snapshot_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (network, slot)
)
"#;

    /// Writes a v1 database to `path` and closes every connection to it, so the
    /// pool under test has to open its own.
    async fn write_legacy_v1_file(path: &str) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::from_str(&format!("sqlite:{path}"))
                    .unwrap()
                    .create_if_missing(true)
                    // Match what the pre-v2 service left on disk.
                    .journal_mode(SqliteJournalMode::Wal),
            )
            .await
            .unwrap();
        for sql in [
            CREATE_MIGRATIONS_TABLE_SQL,
            CREATE_VOTE_ACCOUNTS_TABLE_SQL,
            CREATE_STAKE_ACCOUNTS_TABLE_SQL,
            V1_SNAPSHOT_META_TABLE_SQL,
        ] {
            sqlx::query(sql).execute(&pool).await.unwrap();
        }
        sqlx::query(
            "INSERT INTO schema_migrations (version, applied_at, description) VALUES (1, '', 'v1')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO snapshot_meta (network, slot, merkle_root, snapshot_hash, created_at)
             VALUES ('mainnet', 100, 'root', 'hash', '2026-01-01T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;
    }

    /// The upgrade path as a deployment actually runs it: an on-disk v1 database,
    /// migrated by `init_pool`, then read through the pool it returns.
    ///
    /// The migrator's own tests cannot catch this. They migrate and read on one
    /// connection, which sees the new column because it applied the `ALTER` — but
    /// a serving connection opened beforehand keeps its original column metadata,
    /// so `SELECT *` yields the v1 columns and resolving `total_active_stake`
    /// indexes past the end of the row. That panics the sqlx worker, and the read
    /// surfaces as "no snapshots found" rather than an error.
    #[tokio::test]
    async fn a_legacy_database_is_readable_through_the_pool_init_returns() {
        let path = std::env::temp_dir()
            .join(format!("verifier-legacy-{}.db", std::process::id()))
            .to_string_lossy()
            .into_owned();
        let _ = fs::remove_file(&path);
        write_legacy_v1_file(&path).await;

        let pool = init_pool(&path).await.expect("init_pool must migrate");

        let record = SnapshotMetaRecord::get_latest(&pool, "mainnet")
            .await
            .expect("reading a migrated legacy database must not fail")
            .expect("the v1 row must still be found");
        assert_eq!(record.slot, 100);
        assert_eq!(record.total_active_stake, None);

        pool.close().await;
        let _ = fs::remove_file(&path);
    }
}
