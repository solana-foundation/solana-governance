//! Database migration implementation (SQLx)

use anyhow::Result;
use sqlx::sqlite::SqlitePool;
use tracing::info;

use super::constants::MIGRATION_DESCRIPTIONS;
use super::sql::{
    CREATE_DB_INDEXES, CREATE_MIGRATIONS_TABLE_SQL, CREATE_SNAPSHOT_META_TABLE_SQL,
    CREATE_STAKE_ACCOUNTS_TABLE_SQL, CREATE_VOTE_ACCOUNTS_TABLE_SQL,
};

/// Run all pending database migrations
pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    info!("Running database migrations");

    // Create migrations table if it doesn't exist
    create_migrations_table(pool).await?;

    // Get current version
    let current_version = get_current_version(pool).await?;
    info!("Current database version: {}", current_version);

    // Apply migrations in order
    if current_version < 1 {
        apply_migration_v1(pool).await?;
    }

    info!("All migrations completed");
    Ok(())
}

/// Create the schema_migrations table
async fn create_migrations_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(CREATE_MIGRATIONS_TABLE_SQL)
        .execute(pool)
        .await?;
    Ok(())
}

/// Get the current schema version
async fn get_current_version(pool: &SqlitePool) -> Result<i32> {
    let version: Option<i32> = sqlx::query_scalar("SELECT MAX(version) FROM schema_migrations")
        .fetch_one(pool)
        .await
        .unwrap_or(None);
    Ok(version.unwrap_or(0))
}

/// Apply migration version 1: Initiate tables and indexes.
async fn apply_migration_v1(pool: &SqlitePool) -> Result<()> {
    info!("Applying migration v1: {}", MIGRATION_DESCRIPTIONS[0]);

    let mut tx = pool.begin().await?;

    // Create core tables and indexes
    sqlx::query(CREATE_VOTE_ACCOUNTS_TABLE_SQL)
        .execute(&mut *tx)
        .await?;
    sqlx::query(CREATE_STAKE_ACCOUNTS_TABLE_SQL)
        .execute(&mut *tx)
        .await?;
    sqlx::query(CREATE_SNAPSHOT_META_TABLE_SQL)
        .execute(&mut *tx)
        .await?;

    for index_sql in CREATE_DB_INDEXES {
        sqlx::query(index_sql).execute(&mut *tx).await?;
    }

    // Record migration
    sqlx::query(
        "INSERT INTO schema_migrations (version, applied_at, description) VALUES (?, ?, ?)",
    )
    .bind(1)
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(MIGRATION_DESCRIPTIONS[0])
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    info!("Migration v1 completed successfully");
    Ok(())
}
