//! Picks the snapshot pair a replay should resume from.
//!
//! Both `ledger_utils::get_bank_from_ledger` (which pre-flights the blockstore
//! range) and `load_and_process_ledger` (which actually loads the bank) need the
//! same answer. They live in different modules, so the choice is made here once
//! and shared.

use std::{collections::HashMap, path::Path};

use agave_snapshots::{
    paths::{full_snapshot_archives_iter, incremental_snapshot_archives_iter},
    snapshot_archive_info::{
        FullSnapshotArchiveInfo, IncrementalSnapshotArchiveInfo, SnapshotArchiveInfoGetter,
    },
};
use solana_sdk::clock::Slot;

/// A full snapshot and, when one exists, the incremental snapshot built on top
/// of it.
pub struct SelectedSnapshots {
    pub full: FullSnapshotArchiveInfo,
    pub incremental: Option<IncrementalSnapshotArchiveInfo>,
}

impl SelectedSnapshots {
    /// The slot the bank resumes from once these archives are loaded.
    pub fn starting_slot(&self) -> Slot {
        self.incremental
            .as_ref()
            .map_or_else(|| self.full.slot(), |archive| archive.slot())
    }
}

/// Find the pair that resumes closest to `halt_at_slot`, ignoring archives above
/// it.
///
/// The highest full snapshot is not always the best starting point. A directory
/// can hold a full snapshot that no incremental is based on, in which case an
/// older full plus a recent incremental resumes much later. Scoring every full
/// snapshot by the slot its pair actually reaches picks the shortest replay and
/// keeps unrelated archives in the directory from dragging the start slot
/// backwards.
///
/// Returns `None` when the directory holds no full snapshot at or below
/// `halt_at_slot`.
pub fn select_snapshot_archives(
    full_snapshot_archives_dir: &Path,
    incremental_snapshot_archives_dir: &Path,
    halt_at_slot: Option<Slot>,
) -> Option<SelectedSnapshots> {
    let within_range = |slot: Slot| halt_at_slot.is_none_or(|halt_slot| slot <= halt_slot);

    // Highest usable incremental for each base slot.
    let mut best_incremental: HashMap<Slot, Slot> = HashMap::new();
    for archive in incremental_snapshot_archives_iter(incremental_snapshot_archives_dir) {
        if !within_range(archive.slot()) {
            continue;
        }
        best_incremental
            .entry(archive.base_slot())
            .and_modify(|slot| *slot = (*slot).max(archive.slot()))
            .or_insert(archive.slot());
    }

    // Rank by the slot the pair reaches, then by the full snapshot's own slot so
    // that two pairs reaching the same slot resolve to the one with less
    // incremental data to load.
    let (full_slot, incremental_slot) = full_snapshot_archives_iter(full_snapshot_archives_dir)
        .map(|archive| archive.slot())
        .filter(|slot| within_range(*slot))
        .map(|full_slot| (full_slot, best_incremental.get(&full_slot).copied()))
        .max_by_key(|(full_slot, incremental_slot)| {
            ((*incremental_slot).unwrap_or(*full_slot), *full_slot)
        })?;

    let full =
        full_snapshot_archives_iter(full_snapshot_archives_dir).find(|a| a.slot() == full_slot)?;
    let incremental = incremental_slot.and_then(|incremental_slot| {
        incremental_snapshot_archives_iter(incremental_snapshot_archives_dir)
            .find(|a| a.base_slot() == full_slot && a.slot() == incremental_slot)
    });

    Some(SelectedSnapshots { full, incremental })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use tempfile::TempDir;

    const HASH: &str = "11111111111111111111111111111111";

    fn write_full(dir: &Path, slot: Slot) {
        fs::write(dir.join(format!("snapshot-{slot}-{HASH}.tar.zst")), b"").unwrap();
    }

    fn write_incremental(dir: &Path, base_slot: Slot, slot: Slot) {
        fs::write(
            dir.join(format!(
                "incremental-snapshot-{base_slot}-{slot}-{HASH}.tar.zst"
            )),
            b"",
        )
        .unwrap();
    }

    /// A previous run leaves its own generated snapshot in the directory. It is
    /// the highest full snapshot but nothing is based on it, so pairing an older
    /// full with a recent incremental resumes much later.
    #[test]
    fn skips_a_higher_full_snapshot_that_has_no_incremental() {
        let dir = TempDir::new().unwrap();
        write_full(dir.path(), 100);
        write_incremental(dir.path(), 100, 500);
        write_full(dir.path(), 300);

        let selected = select_snapshot_archives(dir.path(), dir.path(), Some(600)).unwrap();

        assert_eq!(selected.full.slot(), 100);
        assert_eq!(
            selected.incremental.as_ref().map(|archive| archive.slot()),
            Some(500)
        );
        assert_eq!(selected.starting_slot(), 500);
    }

    #[test]
    fn ignores_archives_above_the_halt_slot() {
        let dir = TempDir::new().unwrap();
        write_full(dir.path(), 100);
        write_incremental(dir.path(), 100, 500);
        write_incremental(dir.path(), 100, 900);
        write_full(dir.path(), 800);

        let selected = select_snapshot_archives(dir.path(), dir.path(), Some(600)).unwrap();

        assert_eq!(selected.starting_slot(), 500);
    }

    #[test]
    fn uses_the_full_snapshot_when_nothing_is_based_on_it() {
        let dir = TempDir::new().unwrap();
        write_full(dir.path(), 300);
        write_incremental(dir.path(), 100, 250);

        let selected = select_snapshot_archives(dir.path(), dir.path(), Some(600)).unwrap();

        assert_eq!(selected.full.slot(), 300);
        assert!(selected.incremental.is_none());
        assert_eq!(selected.starting_slot(), 300);
    }

    #[test]
    fn returns_none_when_every_full_snapshot_is_above_the_halt_slot() {
        let dir = TempDir::new().unwrap();
        write_full(dir.path(), 900);

        assert!(select_snapshot_archives(dir.path(), dir.path(), Some(600)).is_none());
    }
}
