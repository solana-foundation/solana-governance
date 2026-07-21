//! Vendored slice of `jito-tip-router/tip-router-operator-cli` ledger utilities.
//!
//! Only the self-contained ledger-loading path is copied here (no `jito-*`
//! dependencies): [`ledger_utils`] (bank reconstruction from ledger/snapshot),
//! [`load_and_process_ledger`], and the `clap_old` [`arg_matches`] builder used
//! to drive the ledger-tool code paths.

use std::path::PathBuf;

pub mod arg_matches;
pub mod ledger_utils;
pub mod load_and_process_ledger;
pub mod resource_limits;

/// Filesystem locations the operator CLI uses to reconstruct a bank and
/// (optionally) write a backup snapshot. Copied verbatim from
/// `tip-router-operator-cli::cli::SnapshotPaths`.
#[derive(Debug, Clone)]
pub struct SnapshotPaths {
    pub ledger_path: PathBuf,
    pub account_paths: Vec<PathBuf>,
    pub full_snapshots_path: PathBuf,
    pub incremental_snapshots_path: PathBuf,
    /// Used when storing or loading snapshots that the operator CLI is working with
    pub backup_snapshots_dir: PathBuf,
}
