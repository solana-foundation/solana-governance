use anyhow::Result;
use axum::http::StatusCode;
use serde_json;
use sqlx::{sqlite::SqlitePool, Executor, Row as SqlxRow, Sqlite};
use std::convert::TryFrom;
use tracing::debug;
use tracing::info;

use super::models::*;

/// Database operations for vote accounts
impl VoteAccountRecord {
    pub async fn insert_exec<'e, E>(&self, exec: E) -> Result<()>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query(
            "INSERT INTO vote_accounts
             (network, snapshot_slot, vote_account, voting_wallet, stake_merkle_root, active_stake, meta_merkle_proof)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(network, vote_account, snapshot_slot) DO UPDATE SET
             voting_wallet = excluded.voting_wallet,
             stake_merkle_root = excluded.stake_merkle_root,
             active_stake = excluded.active_stake,
             meta_merkle_proof = excluded.meta_merkle_proof",
        )
        .bind(&self.network)
        .bind(i64::try_from(self.snapshot_slot)?)
        .bind(&self.vote_account)
        .bind(&self.voting_wallet)
        .bind(&self.stake_merkle_root)
        .bind(i64::try_from(self.active_stake)?)
        .bind(serde_json::to_string(&self.meta_merkle_proof)?)
        .execute(exec)
        .await?;

        Ok(())
    }

    /// Get vote account summaries filtered by voting wallet
    pub async fn get_summary_by_voting_wallet(
        pool: &SqlitePool,
        network: &str,
        voting_wallet: &str,
        snapshot_slot: u64,
    ) -> Result<Vec<VoteAccountSummary>> {
        let rows = sqlx::query(
            "SELECT vote_account, active_stake FROM vote_accounts
             WHERE network = ? AND voting_wallet = ? AND snapshot_slot = ?
             ORDER BY vote_account",
        )
        .bind(network)
        .bind(voting_wallet)
        .bind(i64::try_from(snapshot_slot)?)
        .fetch_all(pool)
        .await?;

        let records = rows
            .into_iter()
            .map(|row| VoteAccountSummary {
                vote_account: row.get("vote_account"),
                active_stake: row.get::<i64, _>("active_stake") as u64,
            })
            .collect();

        Ok(records)
    }

    /// Get vote account by specific account, network and snapshot slot
    pub async fn get_by_account(
        pool: &SqlitePool,
        network: &str,
        vote_account: &str,
        snapshot_slot: u64,
    ) -> Result<Option<VoteAccountRecord>> {
        let row_opt = sqlx::query(
            "SELECT * FROM vote_accounts \
                   WHERE network = ? AND vote_account = ? AND snapshot_slot = ?",
        )
        .bind(network)
        .bind(vote_account)
        .bind(i64::try_from(snapshot_slot)?)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row_opt {
            let meta_merkle_proof_json: String = row.get("meta_merkle_proof");
            Ok(Some(VoteAccountRecord {
                network: row.get("network"),
                snapshot_slot: row.get::<i64, _>("snapshot_slot") as u64,
                vote_account: row.get("vote_account"),
                voting_wallet: row.get("voting_wallet"),
                stake_merkle_root: row.get("stake_merkle_root"),
                active_stake: row.get::<i64, _>("active_stake") as u64,
                meta_merkle_proof: serde_json::from_str(&meta_merkle_proof_json)
                    .unwrap_or_default(),
            }))
        } else {
            Ok(None)
        }
    }
}

/// Database operations for stake accounts
impl StakeAccountRecord {
    pub async fn insert_exec<'e, E>(&self, exec: E) -> Result<()>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query(
            "INSERT INTO stake_accounts
             (network, snapshot_slot, stake_account, vote_account, voting_wallet, active_stake, stake_merkle_proof)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(network, stake_account, snapshot_slot) DO UPDATE SET
             vote_account = excluded.vote_account,
             voting_wallet = excluded.voting_wallet,
             active_stake = excluded.active_stake,
             stake_merkle_proof = excluded.stake_merkle_proof",
        )
        .bind(&self.network)
        .bind(i64::try_from(self.snapshot_slot)?)
        .bind(&self.stake_account)
        .bind(&self.vote_account)
        .bind(&self.voting_wallet)
        .bind(i64::try_from(self.active_stake)?)
        .bind(serde_json::to_string(&self.stake_merkle_proof)?)
        .execute(exec)
        .await?;

        Ok(())
    }

    /// Get stake account summaries filtered by voting wallet
    pub async fn get_summary_by_voting_wallet(
        pool: &SqlitePool,
        network: &str,
        voting_wallet: &str,
        snapshot_slot: u64,
    ) -> Result<Vec<StakeAccountSummary>> {
        let rows = sqlx::query(
            "SELECT stake_account, vote_account, active_stake FROM stake_accounts
             WHERE network = ? AND voting_wallet = ? AND snapshot_slot = ?
             ORDER BY stake_account",
        )
        .bind(network)
        .bind(voting_wallet)
        .bind(i64::try_from(snapshot_slot)?)
        .fetch_all(pool)
        .await?;

        let records = rows
            .into_iter()
            .map(|row| StakeAccountSummary {
                stake_account: row.get::<String, _>("stake_account"),
                vote_account: row.get::<String, _>("vote_account"),
                active_stake: row.get::<i64, _>("active_stake") as u64,
            })
            .collect();

        Ok(records)
    }

    /// Get stake account by specific account, network and snapshot slot
    pub async fn get_by_account(
        pool: &SqlitePool,
        network: &str,
        stake_account: &str,
        snapshot_slot: u64,
    ) -> Result<Option<StakeAccountRecord>> {
        let row_opt = sqlx::query(
            "SELECT * FROM stake_accounts \
                   WHERE network = ? AND stake_account = ? AND snapshot_slot = ?",
        )
        .bind(network)
        .bind(stake_account)
        .bind(i64::try_from(snapshot_slot)?)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row_opt {
            let stake_merkle_proof_json: String = row.get("stake_merkle_proof");
            Ok(Some(StakeAccountRecord {
                network: row.get("network"),
                snapshot_slot: row.get::<i64, _>("snapshot_slot") as u64,
                stake_account: row.get("stake_account"),
                vote_account: row.get("vote_account"),
                voting_wallet: row.get("voting_wallet"),
                active_stake: row.get::<i64, _>("active_stake") as u64,
                stake_merkle_proof: serde_json::from_str(&stake_merkle_proof_json)
                    .unwrap_or_default(),
            }))
        } else {
            Ok(None)
        }
    }
}

/// Database operations for snapshot metadata
impl SnapshotMetaRecord {
    pub async fn insert_exec<'e, E>(&self, exec: E) -> Result<()>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        debug!(
            "Inserting snapshot meta for slot {} on network {}",
            self.slot, self.network
        );

        sqlx::query(
            "INSERT INTO snapshot_meta
             (network, slot, merkle_root, snapshot_hash, created_at)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(network, slot) DO UPDATE SET
             merkle_root = excluded.merkle_root,
             snapshot_hash = excluded.snapshot_hash,
             created_at = excluded.created_at",
        )
        .bind(&self.network)
        .bind(i64::try_from(self.slot)?)
        .bind(&self.merkle_root)
        .bind(&self.snapshot_hash)
        .bind(&self.created_at)
        .execute(exec)
        .await?;

        Ok(())
    }

    /// Get the latest snapshot metadata for a network
    pub async fn get_latest(
        pool: &SqlitePool,
        network: &str,
    ) -> Result<Option<SnapshotMetaRecord>> {
        let row_opt = sqlx::query(
            "SELECT * FROM snapshot_meta
             WHERE network = ? ORDER BY slot DESC LIMIT 1",
        )
        .bind(network)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row_opt {
            Ok(Some(SnapshotMetaRecord {
                network: row.get("network"),
                slot: row.get::<i64, _>("slot") as u64,
                merkle_root: row.get("merkle_root"),
                snapshot_hash: row.get("snapshot_hash"),
                created_at: row.get("created_at"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Get the latest slot for a network
    pub async fn get_latest_slot(pool: &SqlitePool, network: &str) -> Result<Option<u64>> {
        let row_opt = sqlx::query(
            "SELECT slot FROM snapshot_meta
             WHERE network = ? ORDER BY slot DESC LIMIT 1",
        )
        .bind(network)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row_opt {
            Ok(Some(row.get::<i64, _>("slot") as u64))
        } else {
            Ok(None)
        }
    }
}

/// Wrapper for database operations with consistent error handling
pub async fn db_operation<T, F, Fut>(operation: F, error_msg: &str) -> Result<T, StatusCode>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    operation().await.map_err(|e| {
        info!("{}: {}", error_msg, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}
