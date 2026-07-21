//! Mirrors `solana_core::resource_limits::check_memlock_limit_for_disk_io`, which
//! upstream `agave-ledger-tool` uses to gate `SnapshotConfig::use_registered_io_uring_buffers`.
//!
//! Registering io_uring buffers requires locking up to `TOTAL_IO_URING_BUFFERS_SIZE_LIMIT`
//! bytes via `RLIMIT_MEMLOCK`. Without this check, `SnapshotConfig::default()` leaves
//! `use_registered_io_uring_buffers: true` unconditionally, so on hosts where the memlock
//! ulimit can't be raised high enough, io_uring buffer registration fails with ENOMEM and
//! that surfaces as a hard "Cannot allocate memory" error while unpacking the snapshot
//! archive -- unrelated to how much free RAM the host actually has.
//!
//! We don't depend on `solana-core` for this because it would pull in the entire validator
//! core crate for a couple dozen lines of libc calls.

#[cfg(target_os = "linux")]
fn get_memlock() -> libc::rlimit {
    let mut memlock = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    if unsafe { libc::getrlimit(libc::RLIMIT_MEMLOCK, &mut memlock) } != 0 {
        log::warn!("getrlimit(RLIMIT_MEMLOCK) failed");
    }
    memlock
}

#[cfg(target_os = "linux")]
fn try_adjust_ulimit_memlock(min_required: usize) -> bool {
    let mut memlock = get_memlock();
    let current = memlock.rlim_cur as usize;
    if current < min_required {
        memlock.rlim_cur = min_required as u64;
        memlock.rlim_max = min_required as u64;
        if unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &memlock) } != 0 {
            log::warn!(
                "Unable to increase the maximum memory lock limit to {min_required} from \
                 {current}; disabling registered io_uring buffers for snapshot IO"
            );
            return false;
        }
        memlock = get_memlock();
        log::info!("Bumped maximum memory lock limit: {}", memlock.rlim_cur);
    }
    true
}

/// Returns whether it's safe to enable `SnapshotConfig::use_registered_io_uring_buffers`
/// given `required_size` bytes of buffers, falling back to `false` (unregistered buffers)
/// rather than letting a later io_uring syscall fail with ENOMEM.
pub fn check_memlock_limit_for_disk_io(required_size: usize) -> bool {
    #[cfg(target_os = "linux")]
    {
        try_adjust_ulimit_memlock(required_size)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = required_size;
        false
    }
}