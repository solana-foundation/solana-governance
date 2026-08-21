use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};

use agave_snapshots::{
    error::SnapshotError, paths::full_snapshot_archives_iter,
    snapshot_archive_info::SnapshotArchiveInfoGetter, snapshot_config::SnapshotConfig,
    SnapshotVersion,
};
use clap_old::ArgMatches;
use log::{info, warn};
use solana_accounts_db::{
    accounts_db::{AccountsDbConfig, TOTAL_IO_URING_BUFFERS_SIZE_LIMIT},
    accounts_index::{
        AccountIndex, AccountSecondaryIndexes, AccountSecondaryIndexesIncludeExclude,
    },
};
use solana_genesis_utils::{
    open_genesis_config, OpenGenesisConfigError, MAX_GENESIS_ARCHIVE_UNPACKED_SIZE,
};
use solana_ledger::{
    blockstore::{Blockstore, BlockstoreError},
    blockstore_options::{AccessType, BlockstoreOptions},
    blockstore_processor::ProcessOptions,
};
use solana_metrics::{datapoint_error, datapoint_info};
use solana_runtime::{bank::Bank, snapshot_bank_utils};
use solana_sdk::clock::Slot;
use thiserror::Error;

use super::{
    arg_matches, load_and_process_ledger, resource_limits,
    snapshot_selection::select_snapshot_archives,
};

#[derive(Error, Debug)]
pub enum LedgerUtilsError {
    #[error("BankFromSnapshot error: {0}")]
    BankFromSnapshotError(#[from] SnapshotError),

    #[error("Missing snapshot at slot {0}")]
    MissingSnapshotAtSlot(u64),

    #[error(transparent)]
    OpenGenesisConfigError(#[from] OpenGenesisConfigError),

    #[error("{0}")]
    GenesisConfigError(String),

    #[error("{0}")]
    BlockstoreOpenError(String),

    #[error("{0}")]
    BlockstoreRangeError(String),

    #[error("{0}")]
    LoadBankForksError(String),

    #[error("{0}")]
    SnapshotCreationError(String),

    #[error("Expected bank at slot {expected}, found {found}")]
    BankSlotMismatch { expected: u64, found: u64 },
}

/// Open the blockstore in `original_access_type`. If the open fails because of
/// a missing column family — which secondary/ReadOnly mode cannot create —
/// briefly reopen in `PrimaryForMaintenance` so RocksDB persists the missing
/// CFs, then retry the original open.
fn open_blockstore_creating_missing_columns(
    ledger_path: &Path,
    original_access_type: AccessType,
) -> Result<Blockstore, BlockstoreError> {
    let open = |access_type: AccessType| {
        Blockstore::open_with_options(
            ledger_path,
            BlockstoreOptions {
                access_type,
                ..BlockstoreOptions::default()
            },
        )
    };
    match open(original_access_type.clone()) {
        Ok(blockstore) => Ok(blockstore),
        Err(BlockstoreError::RocksDb(err))
            if err
                .to_string()
                .starts_with("Invalid argument: Column family not found:") =>
        {
            warn!(
                "Blockstore at {:?} is missing a column family; reopening in \
                 PrimaryForMaintenance to create it: {}",
                ledger_path, err
            );
            let _ = open(AccessType::PrimaryForMaintenance)?;
            open(original_access_type)
        }
        Err(err) => Err(err),
    }
}

/// Create the Bank for a desired slot for given file paths.
#[allow(clippy::cognitive_complexity, clippy::too_many_arguments)]
pub fn get_bank_from_ledger(
    operator_address: String,
    ledger_path: &Path,
    account_paths: Vec<PathBuf>,
    full_snapshots_path: PathBuf,
    incremental_snapshots_path: PathBuf,
    desired_slot: &Slot,
    save_snapshot: bool,
    snapshot_save_path: PathBuf,
    cluster: &str,
) -> Result<Arc<Bank>, LedgerUtilsError> {
    let start_time = Instant::now();

    // Start validation
    datapoint_info!(
        "tip_router_cli.get_bank",
        ("operator", operator_address, String),
        ("state", "validate_path_start", String),
        ("step", 0, i64),
        ("version", env!("CARGO_PKG_VERSION").to_string(), String),
        ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
        "cluster" => cluster
    );

    // STEP 1: Load genesis config //

    datapoint_info!(
        "tip_router_cli.get_bank",
        ("operator", operator_address, String),
        ("state", "load_genesis_start", String),
        ("step", 1, i64),
        ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
        "cluster" => cluster
    );

    let genesis_config = match open_genesis_config(ledger_path, MAX_GENESIS_ARCHIVE_UNPACKED_SIZE) {
        Ok(genesis_config) => genesis_config,
        Err(e) => {
            let error_str = format!("Failed to load genesis config: {}", e);
            datapoint_error!(
                "tip_router_cli.get_bank",
                ("operator", operator_address, String),
                ("status", "error", String),
                ("state", "load_genesis", String),
                ("step", 1, i64),
                ("error", error_str, String),
                "cluster" => cluster,
            );
            return Err(LedgerUtilsError::GenesisConfigError(error_str));
        }
    };

    // STEP 2: Load blockstore //

    datapoint_info!(
        "tip_router_cli.get_bank",
        ("operator", operator_address, String),
        ("state", "load_blockstore_start", String),
        ("step", 2, i64),
        ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
        "cluster" => cluster
    );

    let access_type = AccessType::ReadOnly;
    // Error handling is a modified copy pasta from ledger utils
    let blockstore =
        match open_blockstore_creating_missing_columns(ledger_path, access_type.clone()) {
            Ok(blockstore) => blockstore,
            Err(BlockstoreError::RocksDb(err)) => {
                // Missing essential file, indicative of blockstore not existing
                let missing_blockstore = err
                    .to_string()
                    .starts_with("IO error: No such file or directory:");
                // Missing column in blockstore that is expected by software
                let missing_column = err
                    .to_string()
                    .starts_with("Invalid argument: Column family not found:");
                // The blockstore settings with Primary access can resolve the
                // above issues automatically, so only emit the help messages
                // if access type is Secondary
                let is_secondary = access_type == AccessType::ReadOnly;

                let error_str = if missing_blockstore && is_secondary {
                    format!(
                        "Failed to open blockstore at {ledger_path:?}, it is missing at least one \
                     critical file: {err:?}"
                    )
                } else if missing_column && is_secondary {
                    format!(
                    "Failed to open blockstore at {ledger_path:?}, it does not have all necessary \
                     columns: {err:?}"
                )
                } else {
                    format!("Failed to open blockstore at {ledger_path:?}: {err:?}")
                };
                datapoint_error!(
                    "tip_router_cli.get_bank",
                    ("operator", operator_address, String),
                    ("status", "error", String),
                    ("state", "load_blockstore", String),
                    ("step", 2, i64),
                    ("error", error_str, String),
                    ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
                    "cluster" => cluster,
                );
                return Err(LedgerUtilsError::BlockstoreOpenError(error_str));
            }
            Err(err) => {
                let error_str = format!("Failed to open blockstore at {ledger_path:?}: {err:?}");
                datapoint_error!(
                    "tip_router_cli.get_bank",
                    ("operator", operator_address, String),
                    ("status", "error", String),
                    ("state", "load_blockstore", String),
                    ("step", 2, i64),
                    ("error", error_str, String),
                    ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
                    "cluster" => cluster,
                );
                return Err(LedgerUtilsError::BlockstoreOpenError(error_str));
            }
        };

    let desired_slot_in_blockstore = match blockstore.meta(*desired_slot) {
        Ok(meta) => meta.is_some(),
        Err(err) => {
            warn!("Failed to get meta for slot {}: {:?}", desired_slot, err);
            false
        }
    };
    info!(
        "Desired slot {} in blockstore: {}",
        desired_slot, desired_slot_in_blockstore
    );

    // STEP 3: Load bank forks //

    datapoint_info!(
        "tip_router_cli.get_bank",
        ("operator", operator_address, String),
        ("state", "load_snapshot_config_start", String),
        ("step", 3, i64),
        ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
        "cluster" => cluster
    );

    let snapshot_config = SnapshotConfig {
        full_snapshot_archives_dir: full_snapshots_path.clone(),
        incremental_snapshot_archives_dir: incremental_snapshots_path.clone(),
        bank_snapshots_dir: full_snapshots_path.clone(),
        use_registered_io_uring_buffers: resource_limits::check_memlock_limit_for_disk_io(
            TOTAL_IO_URING_BUFFERS_SIZE_LIMIT,
        ),
        ..SnapshotConfig::new_load_only()
    };

    let process_options = ProcessOptions {
        halt_at_slot: Some(desired_slot.to_owned()),
        no_block_cost_limits: true,
        ..Default::default()
    };

    // Use the same archives load_and_process_ledger will use, so the range check
    // below covers the slots the replay actually reads. Falls back to genesis
    // when the directory holds no usable full snapshot.
    let starting_slot = select_snapshot_archives(
        &full_snapshots_path,
        &incremental_snapshots_path,
        process_options.halt_at_slot,
    )
    .map_or(0, |selected| {
        info!(
            "Selected full snapshot {} and incremental snapshot {:?}",
            selected.full.slot(),
            selected.incremental.as_ref().map(|archive| archive.slot())
        );
        selected.starting_slot()
    });
    info!("Starting slot {}", starting_slot);

    match process_options.halt_at_slot {
        // Skip the following checks for sentinel values of Some(0) and None.
        // For Some(0), no slots will be be replayed after starting_slot.
        // For None, all available children of starting_slot will be replayed.
        None | Some(0) => {}
        Some(halt_slot) => {
            if halt_slot < starting_slot {
                let error_str = String::from("halt_slot < starting_slot");
                datapoint_error!(
                    "tip_router_cli.get_bank",
                    ("operator", operator_address, String),
                    ("status", "error", String),
                    ("state", "load_blockstore", String),
                    ("step", 2, i64),
                    ("error", error_str, String),
                    ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
                    "cluster" => cluster,
                );
                return Err(LedgerUtilsError::BlockstoreRangeError(error_str));
            }
            // Check if we have the slot data necessary to replay from starting_slot to >= halt_slot.
            if !blockstore.slot_range_connected(starting_slot, halt_slot) {
                let error_str =
                    format!("Blockstore missing data to replay to slot {}", desired_slot);
                datapoint_error!(
                    "tip_router_cli.get_bank",
                    ("operator", operator_address, String),
                    ("status", "error", String),
                    ("state", "load_blockstore", String),
                    ("step", 2, i64),
                    ("error", error_str, String),
                    ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
                    "cluster" => cluster,
                );
                return Err(LedgerUtilsError::BlockstoreRangeError(error_str));
            }
        }
    }
    let exit = Arc::new(AtomicBool::new(false));

    let mut arg_matches = ArgMatches::new();
    arg_matches::set_ledger_tool_arg_matches(
        &mut arg_matches,
        snapshot_config.full_snapshot_archives_dir.clone(),
        snapshot_config.incremental_snapshot_archives_dir.clone(),
        account_paths,
    );

    // Call ledger_utils::load_and_process_ledger here
    let (bank_forks, _starting_snapshot_hashes) =
        match load_and_process_ledger::load_and_process_ledger(
            &arg_matches,
            &genesis_config,
            Arc::new(blockstore),
            process_options,
            Some(full_snapshots_path),
            Some(incremental_snapshots_path),
            operator_address.clone(),
            cluster,
        ) {
            Ok(res) => res,
            Err(e) => {
                let error_str = format!("Failed to load bank forks: {}", e);
                datapoint_error!(
                    "tip_router_cli.get_bank",
                    ("operator", operator_address, String),
                    ("state", "load_bank_forks", String),
                    ("status", "error", String),
                    ("step", 4, i64),
                    ("error", error_str, String),
                    ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
                    "cluster" => cluster,
                );
                return Err(LedgerUtilsError::LoadBankForksError(error_str));
            }
        };

    // STEP 5: Save snapshot //

    let working_bank = bank_forks
        .read()
        .expect("bank forks lock should not be poisoned when building snapshot")
        .working_bank();

    datapoint_info!(
        "tip_router_cli.get_bank",
        ("operator", operator_address, String),
        ("state", "bank_to_full_snapshot_archive_start", String),
        ("bank_hash", working_bank.hash().to_string(), String),
        ("step", 5, i64),
        ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
        "cluster" => cluster,
    );

    exit.store(true, Ordering::Relaxed);

    if save_snapshot {
        // Finalize the bank: freeze it, mark the chain as rooted, and merge parent account stores.
        working_bank.squash();
        // Flush in-memory account cache to disk so the snapshot serializer can read all accounts.
        working_bank.force_flush_accounts_cache();
        // Upstream agave 4.1.0 collapsed `bank_to_full_snapshot_archive` to
        // `(&SnapshotConfig, &Bank)`. Build a load-only config that points the
        // full-snapshot output at `snapshot_save_path` (a directory distinct from
        // the node's primary snapshot directory) and stamps the snapshot version.
        let out_snapshot_config = SnapshotConfig {
            full_snapshot_archives_dir: snapshot_save_path.clone(),
            incremental_snapshot_archives_dir: snapshot_config
                .incremental_snapshot_archives_dir
                .clone(),
            bank_snapshots_dir: ledger_path.to_path_buf(),
            snapshot_version: SnapshotVersion::default(),
            archive_format: snapshot_config.archive_format,
            ..SnapshotConfig::new_load_only()
        };
        let full_snapshot_archive_info = match snapshot_bank_utils::bank_to_full_snapshot_archive(
            &out_snapshot_config,
            &working_bank,
        ) {
            Ok(res) => res,
            Err(e) => {
                let error_str = format!("Failed to create snapshot: {}", e);
                datapoint_error!(
                    "tip_router_cli.get_bank",
                    ("operator", operator_address, String),
                    ("status", "error", String),
                    ("state", "bank_to_full_snapshot_archive", String),
                    ("step", 6, i64),
                    ("error", error_str, String),
                    ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
                    "cluster" => cluster,
                );
                return Err(LedgerUtilsError::SnapshotCreationError(error_str));
            }
        };

        info!(
            "Successfully created snapshot for slot {}, hash {}: {}",
            working_bank.slot(),
            working_bank.hash(),
            full_snapshot_archive_info.path().display(),
        );
    }
    // STEP 6: Complete //

    if working_bank.slot() != *desired_slot {
        return Err(LedgerUtilsError::BankSlotMismatch {
            expected: *desired_slot,
            found: working_bank.slot(),
        });
    }

    datapoint_info!(
        "tip_router_cli.get_bank",
        ("operator", operator_address, String),
        ("state", "get_bank_from_ledger_success", String),
        ("step", 6, i64),
        ("duration_ms", start_time.elapsed().as_millis() as i64, i64),
        "cluster" => cluster,
    );
    Ok(working_bank)
}

/// Builds the accounts-db secondary index scoped to the stake and stake-pool programs.
fn stake_program_secondary_indexes() -> AccountSecondaryIndexes {
    AccountSecondaryIndexes {
        keys: Some(AccountSecondaryIndexesIncludeExclude {
            exclude: false,
            keys: HashSet::from([
                solana_stake_interface::program::id(),
                crate::STAKE_POOL_PROGRAM_ID,
            ]),
        }),
        indexes: HashSet::from([AccountIndex::ProgramId]),
    }
}

/// Loads the bank from the snapshot at the exact slot. If the snapshot doesn't exist, result is
/// an error.
pub fn get_bank_from_snapshot_at_slot(
    snapshot_slot: u64,
    full_snapshots_path: &PathBuf,
    bank_snapshots_dir: &PathBuf,
    account_paths: Vec<PathBuf>,
    ledger_path: &Path,
) -> Result<Bank, LedgerUtilsError> {
    let mut full_snapshot_archives: Vec<_> =
        full_snapshot_archives_iter(full_snapshots_path).collect();
    full_snapshot_archives.retain(|archive| archive.snapshot_archive_info().slot == snapshot_slot);

    if full_snapshot_archives.len() != 1 {
        return Err(LedgerUtilsError::MissingSnapshotAtSlot(snapshot_slot));
    }
    let full_snapshot_archive_info = full_snapshot_archives.first().expect("unreachable");
    let process_options = ProcessOptions {
        halt_at_slot: Some(snapshot_slot.to_owned()),
        accounts_db_config: AccountsDbConfig {
            account_indexes: Some(stake_program_secondary_indexes()),
            ..AccountsDbConfig::default()
        },
        ..Default::default()
    };
    let genesis_config = open_genesis_config(ledger_path, MAX_GENESIS_ARCHIVE_UNPACKED_SIZE)?;
    let exit = Arc::new(AtomicBool::new(false));

    // Upstream agave 4.1.0 `bank_from_snapshot_archives` takes a `&SnapshotConfig`
    // (which carries `bank_snapshots_dir`) instead of a bare `bank_snapshots_dir`
    // arg, and adds a `leader_for_tests` slot after `debug_keys`.
    let snapshot_config = SnapshotConfig {
        full_snapshot_archives_dir: full_snapshots_path.clone(),
        incremental_snapshot_archives_dir: full_snapshots_path.clone(),
        bank_snapshots_dir: bank_snapshots_dir.clone(),
        use_registered_io_uring_buffers: resource_limits::check_memlock_limit_for_disk_io(
            TOTAL_IO_URING_BUFFERS_SIZE_LIMIT,
        ),
        ..SnapshotConfig::new_load_only()
    };

    let bank = snapshot_bank_utils::bank_from_snapshot_archives(
        &account_paths,
        full_snapshot_archive_info,
        None, // incremental_snapshot_archive_info
        &snapshot_config,
        &genesis_config,
        &process_options.runtime_config,
        process_options.debug_keys.clone(),
        None, // leader_for_tests
        process_options.limit_load_slot_count_from_snapshot,
        process_options.accounts_db_skip_shrink,
        process_options.accounts_db_force_initial_clean,
        process_options.verify_index,
        process_options.accounts_db_config.clone(),
        None, // accounts_update_notifier
        exit.clone(),
    )?;
    exit.store(true, Ordering::Relaxed);
    Ok(bank)
}
