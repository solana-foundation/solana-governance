//! SQL statement constants for database operations

pub const CREATE_MIGRATIONS_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL,
    description TEXT NOT NULL
)
"#;

pub const CREATE_VOTE_ACCOUNTS_TABLE_SQL: &str = r#"
CREATE TABLE vote_accounts (
    network TEXT NOT NULL,
    snapshot_slot INTEGER NOT NULL,
    vote_account TEXT NOT NULL,
    voting_wallet TEXT NOT NULL,
    stake_merkle_root TEXT NOT NULL,
    active_stake INTEGER NOT NULL,
    meta_merkle_proof TEXT NOT NULL, -- array
    PRIMARY KEY (network, vote_account, snapshot_slot)
)
"#;

pub const CREATE_STAKE_ACCOUNTS_TABLE_SQL: &str = r#"
CREATE TABLE stake_accounts (
    network TEXT NOT NULL,
    snapshot_slot INTEGER NOT NULL,
    stake_account TEXT NOT NULL,
    vote_account TEXT NOT NULL,
    voting_wallet TEXT NOT NULL,
    active_stake INTEGER NOT NULL,
    stake_merkle_proof TEXT NOT NULL, -- array
    PRIMARY KEY (network, stake_account, snapshot_slot)
)
"#;

pub const CREATE_SNAPSHOT_META_TABLE_SQL: &str = r#"
CREATE TABLE snapshot_meta (
    network TEXT NOT NULL,
    slot INTEGER NOT NULL,
    merkle_root TEXT NOT NULL,
    snapshot_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (network, slot)
)
"#;

pub const CREATE_DB_INDEXES: &[&str] = &[
    "CREATE INDEX idx_vote_voting_wallet ON vote_accounts(network, voting_wallet, snapshot_slot)",
    "CREATE INDEX idx_stake_voting_wallet ON stake_accounts(network, voting_wallet, snapshot_slot)",
    "CREATE INDEX idx_snapshot_created_at ON snapshot_meta(network, created_at)",
    // Covering indexes to satisfy ORDER BY without extra sort
    "CREATE INDEX idx_vote_voting_wallet_order ON vote_accounts(network, voting_wallet, snapshot_slot, vote_account)",
    "CREATE INDEX idx_stake_voting_wallet_order ON stake_accounts(network, voting_wallet, snapshot_slot, stake_account)",
];
