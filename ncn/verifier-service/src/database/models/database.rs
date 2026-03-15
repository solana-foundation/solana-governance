//! Database schema models

use serde::{Deserialize, Serialize};

/// Migration record for tracking schema versions
#[derive(Debug, Clone)]
pub struct MigrationRecord {
    pub version: i32,
    pub applied_at: String,
    pub description: String,
}

/// Vote account record in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteAccountRecord {
    pub network: String,
    pub snapshot_slot: u64,
    pub vote_account: String,
    pub voting_wallet: String,
    pub stake_merkle_root: String,
    pub active_stake: u64,
    pub meta_merkle_proof: Vec<String>, // JSON array of base58 hashes
}

/// Stake account record in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeAccountRecord {
    pub network: String,
    pub snapshot_slot: u64,
    pub stake_account: String,
    pub vote_account: String,
    pub voting_wallet: String,
    pub active_stake: u64,
    pub stake_merkle_proof: Vec<String>, // JSON array of base58 hashes
}

/// Snapshot metadata record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetaRecord {
    pub network: String,
    pub slot: u64,
    pub merkle_root: String,
    pub snapshot_hash: String,
    pub created_at: String, // ISO8601 UTC timestamp
}
