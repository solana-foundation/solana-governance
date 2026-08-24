//! Database migration constants and metadata

/// Current database schema version
pub const CURRENT_SCHEMA_VERSION: i32 = 3;

/// Migration descriptions
pub const MIGRATION_DESCRIPTIONS: &[&str] = &[
    "Initial schema with network support",
    "Add snapshot_meta.total_active_stake",
    "Backfill snapshot_meta.total_active_stake from indexed vote accounts",
];

/// Default database file name
pub const DEFAULT_DB_PATH: &str = "governance.db";
