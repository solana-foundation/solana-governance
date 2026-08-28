//! Database migration implementation (SQLx)

use anyhow::Result;
use sqlx::sqlite::SqlitePool;
use tracing::info;

use super::constants::MIGRATION_DESCRIPTIONS;
use super::sql::{
    ADD_SNAPSHOT_TOTAL_ACTIVE_STAKE_SQL, CREATE_DB_INDEXES, CREATE_MIGRATIONS_TABLE_SQL,
    CREATE_SNAPSHOT_META_TABLE_SQL, CREATE_STAKE_ACCOUNTS_TABLE_SQL,
    CREATE_VOTE_ACCOUNTS_TABLE_SQL,
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
    if current_version < 2 {
        apply_migration_v2(pool).await?;
    }
    if current_version < 3 {
        apply_migration_v3(pool).await?;
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

/// Apply migration version 2: record the snapshot's total active stake.
///
/// A database created after this change already has the column, because
/// `CREATE_SNAPSHOT_META_TABLE_SQL` gained it at the same time — so the
/// `ALTER TABLE` is only needed by a database created before it. SQLite has no
/// `ADD COLUMN IF NOT EXISTS`, so the column is probed first rather than
/// running the `ALTER` and swallowing its error, which would also hide a
/// genuinely broken schema.
async fn apply_migration_v2(pool: &SqlitePool) -> Result<()> {
    info!("Applying migration v2: {}", MIGRATION_DESCRIPTIONS[1]);

    let mut tx = pool.begin().await?;

    let already_present = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM pragma_table_info('snapshot_meta') WHERE name = 'total_active_stake'",
    )
    .fetch_one(&mut *tx)
    .await?;

    if already_present == 0 {
        sqlx::query(ADD_SNAPSHOT_TOTAL_ACTIVE_STAKE_SQL)
            .execute(&mut *tx)
            .await?;
    }

    sqlx::query(
        "INSERT INTO schema_migrations (version, applied_at, description) VALUES (?, ?, ?)",
    )
    .bind(2)
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(MIGRATION_DESCRIPTIONS[1])
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    info!("Migration v2 completed successfully");
    Ok(())
}

/// Apply migration version 3: populate totals for snapshots indexed before
/// `total_active_stake` was recorded at upload time.
///
/// Old uploads still indexed one `vote_accounts` row for every meta Merkle
/// leaf, and each row retains that leaf's `active_stake`. Summing those rows
/// recreates the immutable snapshot total; it does not consult the live
/// cluster stake. Leave rows without indexed vote accounts as NULL, since a
/// zero total would incorrectly claim that their contents are known.
async fn apply_migration_v3(pool: &SqlitePool) -> Result<()> {
    info!("Applying migration v3: {}", MIGRATION_DESCRIPTIONS[2]);

    let mut tx = pool.begin().await?;

    sqlx::query(
        "UPDATE snapshot_meta
         SET total_active_stake = (
             SELECT SUM(vote_accounts.active_stake)
             FROM vote_accounts
             WHERE vote_accounts.network = snapshot_meta.network
               AND vote_accounts.snapshot_slot = snapshot_meta.slot
         )
         WHERE total_active_stake IS NULL
           AND EXISTS (
             SELECT 1
             FROM vote_accounts
             WHERE vote_accounts.network = snapshot_meta.network
               AND vote_accounts.snapshot_slot = snapshot_meta.slot
         )",
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO schema_migrations (version, applied_at, description) VALUES (?, ?, ?)",
    )
    .bind(3)
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(MIGRATION_DESCRIPTIONS[2])
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    info!("Migration v3 completed successfully");
    Ok(())
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

    async fn pool() -> SqlitePool {
        SqlitePool::connect("sqlite::memory:").await.unwrap()
    }

    /// A database at the pre-v2 schema, with a row already in it.
    async fn legacy_v1_database() -> SqlitePool {
        let pool = pool().await;
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
        pool
    }

    #[tokio::test]
    async fn upgrading_a_v1_database_adds_the_column_and_keeps_existing_rows() {
        let pool = legacy_v1_database().await;
        run_migrations(&pool).await.unwrap();

        assert_eq!(get_current_version(&pool).await.unwrap(), 3);

        // The pre-existing snapshot has no indexed vote accounts, so its total
        // remains unknown rather than being fabricated as zero.
        let record = SnapshotMetaRecord::get_latest(&pool, "mainnet")
            .await
            .unwrap()
            .expect("the v1 row must still be readable");
        assert_eq!(record.slot, 100);
        assert_eq!(record.merkle_root, "root");
        assert_eq!(record.total_active_stake, None);
    }

    #[tokio::test]
    async fn a_fresh_database_lands_on_v3_with_the_column_present() {
        let pool = pool().await;
        run_migrations(&pool).await.unwrap();
        assert_eq!(get_current_version(&pool).await.unwrap(), 3);

        let record = SnapshotMetaRecord {
            network: "mainnet".to_string(),
            slot: 1,
            merkle_root: "root".to_string(),
            snapshot_hash: "hash".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            total_active_stake: Some(4_000_000_000_000_000),
        };
        record.insert_exec(&pool).await.unwrap();

        let read = SnapshotMetaRecord::get_latest(&pool, "mainnet")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(read.total_active_stake, Some(4_000_000_000_000_000));
    }

    #[tokio::test]
    async fn lookup_returns_the_requested_slot_not_only_the_newest() {
        let pool = pool().await;
        run_migrations(&pool).await.unwrap();

        for slot in [100_u64, 200] {
            SnapshotMetaRecord {
                network: "mainnet".to_string(),
                slot,
                merkle_root: format!("root-{slot}"),
                snapshot_hash: format!("hash-{slot}"),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                total_active_stake: Some(slot),
            }
            .insert_exec(&pool)
            .await
            .unwrap();
        }

        let latest = SnapshotMetaRecord::get_latest(&pool, "mainnet")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(latest.slot, 200);

        let older = SnapshotMetaRecord::lookup(&pool, "mainnet", Some(100))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(older.slot, 100);
        assert_eq!(older.merkle_root, "root-100");
        assert_eq!(older.total_active_stake, Some(100));

        assert!(SnapshotMetaRecord::lookup(&pool, "mainnet", Some(999))
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn upgrading_backfills_totals_from_legacy_vote_accounts() {
        let pool = legacy_v1_database().await;
        sqlx::query(
            "INSERT INTO vote_accounts
             (network, snapshot_slot, vote_account, voting_wallet, stake_merkle_root, active_stake, meta_merkle_proof)
             VALUES
             ('mainnet', 100, 'vote-a', 'wallet-a', 'root-a', 400, '[]'),
             ('mainnet', 100, 'vote-b', 'wallet-b', 'root-b', 600, '[]')",
        )
        .execute(&pool)
        .await
        .unwrap();

        run_migrations(&pool).await.unwrap();

        let record = SnapshotMetaRecord::get_latest(&pool, "mainnet")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(record.total_active_stake, Some(1_000));
    }

    #[tokio::test]
    async fn migrations_are_idempotent() {
        // Startup runs these on every boot; a second pass must be a no-op
        // rather than failing on a duplicate ALTER.
        let pool = legacy_v1_database().await;
        run_migrations(&pool).await.unwrap();
        run_migrations(&pool).await.unwrap();
        assert_eq!(get_current_version(&pool).await.unwrap(), 3);
    }
}
