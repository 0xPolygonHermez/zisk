//! `/dev/shm` janitorial cleanup — unlinking the POSIX shared-memory segments
//! and named semaphores created for the ASM services.
//!
//! Split out from `AsmServices` (which is about process supervision) so the
//! filesystem cleanup concern stands on its own. All entry detection goes
//! through the `naming` predicates so the janitor can't drift from how the
//! names are built.

use anyhow::Result;

use crate::{is_zisk_sem_file, is_zisk_shmem_file, sem_file_to_posix_name};

/// `shm_unlink` a `/dev/shm` segment by its file name.
fn unlink_shmem(name: &str) -> Result<()> {
    let cstr = std::ffi::CString::new(name)?;
    unsafe { libc::shm_unlink(cstr.as_ptr()) };
    Ok(())
}

/// `sem_unlink` a `/dev/shm` semaphore given its backing-file name (`sem.FOO`).
fn unlink_sem_file(name: &str) {
    if let Some(sem_name) = sem_file_to_posix_name(name) {
        if let Ok(cstr) = std::ffi::CString::new(sem_name) {
            unsafe { libc::sem_unlink(cstr.as_ptr()) };
        }
    }
}

/// Scan `/dev/shm` for stale `ZISK_*` shmem segments and `sem.ZISK_*`
/// semaphores left by dead processes and unlink them.
pub(super) fn cleanup_stale() {
    tracing::info!("Cleaning up stale shared memory and semaphores");
    let dev_shm = std::path::Path::new("/dev/shm");
    let entries = match std::fs::read_dir(dev_shm) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let name = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue,
        };

        // stdio shmem: "ZISK_{pid}_{rank}_..."        → parts[1] is PID
        // stdio sem:   "sem.ZISK_{pid}_{hash}_{rank}_..."
        let is_sem = is_zisk_sem_file(&name);
        let is_shm = is_zisk_shmem_file(&name);
        if !is_shm && !is_sem {
            continue;
        }

        let parts: Vec<&str> = name.splitn(3, '_').collect();
        if parts.len() < 3 {
            continue;
        }
        let Ok(pid) = parts[1].parse::<u32>() else { continue };

        // Check if the process is still alive.
        let alive = unsafe { libc::kill(pid as i32, 0) };
        if alive == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM) {
            continue; // process alive or owned by another user
        }

        // Process is dead (ESRCH) — unlink the stale entry.
        if is_sem {
            // sem file "sem.FOO" → POSIX name "/FOO"
            tracing::debug!("Cleaning up stale semaphore: /dev/shm/{}", name);
            unlink_sem_file(&name);
        } else {
            tracing::debug!("Cleaning up stale shared memory: /dev/shm/{}", name);
            let _ = unlink_shmem(&name);
        }
    }
}

/// Unlink every `/dev/shm/{shm_prefix}*` shmem segment and
/// `/dev/shm/sem.{sem_prefix}*` semaphore. The C-side `server_cleanup`
/// only unlinks if `delete_input_shm`/`delete_output_shm` flags are
/// set — which the long-running ASM service children don't have — so
/// the parent has to do it. Call after `stop_asm_services` so the
/// children are already detached from the segments.
pub(super) fn cleanup_prefix(shm_prefix: &str, sem_prefix: &str) {
    let dev_shm = std::path::Path::new("/dev/shm");
    let entries = match std::fs::read_dir(dev_shm) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("Cannot scan /dev/shm for cleanup: {e}");
            return;
        }
    };
    let sem_marker = format!("sem.{}", sem_prefix);
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_string) else { continue };
        if name.starts_with(shm_prefix) {
            let _ = unlink_shmem(&name);
        } else if name.starts_with(&sem_marker) {
            unlink_sem_file(&name);
        }
    }
}
