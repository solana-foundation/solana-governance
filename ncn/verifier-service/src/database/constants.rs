//! Database migration constants and metadata

/// Current database schema version
pub const CURRENT_SCHEMA_VERSION: i32 = 2;

/// Migration descriptions
pub const MIGRATION_DESCRIPTIONS: &[&str] = &[
    "Initial schema with network support",
    "Add snapshot_meta.total_active_stake",
];

/// Default database file name
pub const DEFAULT_DB_PATH: &str = "governance.db";
