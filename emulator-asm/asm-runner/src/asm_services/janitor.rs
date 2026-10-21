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
/// `/dev/shm/sem.{sem_prefix}*` semaphore.
///
/// Only for rolling back a setup that failed while creating a prefix's
/// segments: then no other program can be using them. A program's normal
/// teardown unlinks only its semaphores, and the segments go with the
/// last owner of their `SharedSegments`.
pub(super) fn cleanup_prefix(shm_prefix: &str, sem_prefix: &str) {
    cleanup_shm_prefix(shm_prefix);
    cleanup_sem_prefix(sem_prefix);
}

/// Unlink every `/dev/shm/{shm_prefix}*` shmem segment.
///
/// Split from the semaphore sweep because the two have different owners: the
/// segments are keyed by pid+rank+hints mode and shared by every program on the
/// worker, so only their last owner may unlink them (see `SharedSegments`). The
/// semaphores carry the program hash and belong to one program, so that program
/// unlinks its own on the way out.
pub(super) fn cleanup_shm_prefix(shm_prefix: &str) {
    for_each_dev_shm_entry(|name| {
        if name.starts_with(shm_prefix) {
            let _ = unlink_shmem(name);
        }
    });
}

/// Unlink every `/dev/shm/sem.{sem_prefix}*` semaphore.
pub(super) fn cleanup_sem_prefix(sem_prefix: &str) {
    let sem_marker = format!("sem.{sem_prefix}");
    for_each_dev_shm_entry(|name| {
        if name.starts_with(&sem_marker) {
            unlink_sem_file(name);
        }
    });
}

/// Kill the services of this process whose names begin with `process_prefix`, and unlink
/// every segment and semaphore so named, waiting at most `timeout` for the services to exit.
///
/// For a process about to exit without its teardown, which does this program by program:
/// nothing in it may use them afterwards. A service is a child of this process given a
/// `--shm_prefix` in that namespace, the helpers that create the segments included. It is
/// killed outright, since one busy with a request reads nothing else, and is not reaped: its
/// `Child` belongs to whoever started it. The names are unlinked as soon as it is signalled,
/// in case the process is killed while waiting, and again once it is gone, since a service
/// still running then may have created an output segment in between.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(super) fn release_process(process_prefix: &str, timeout: std::time::Duration) {
    let services = services_of(process_prefix);
    for &pid in &services {
        unsafe { libc::kill(pid, libc::SIGKILL) };
    }
    cleanup_shm_prefix(process_prefix);
    cleanup_sem_prefix(process_prefix);

    let running = wait_until_exited(services.clone(), timeout);
    cleanup_shm_prefix(process_prefix);
    cleanup_sem_prefix(process_prefix);

    if running.is_empty() {
        tracing::info!(
            "Killed {} ASM service process(es); unlinked every {process_prefix}* segment and \
             semaphore",
            services.len()
        );
    } else {
        tracing::warn!(
            "ASM service processes {running:?} had not exited within {timeout:?}; unlinked \
             every {process_prefix}* segment and semaphore regardless"
        );
    }
}

/// The children of this process whose `--shm_prefix` begins with `process_prefix`.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn services_of(process_prefix: &str) -> Vec<libc::pid_t> {
    let Ok(entries) = std::fs::read_dir("/proc") else { return Vec::new() };
    let me = std::process::id();
    entries
        .flatten()
        .filter_map(|entry| entry.file_name().to_str()?.parse::<libc::pid_t>().ok())
        .filter(|&pid| parent_of(pid) == Some(me))
        .filter(|&pid| shm_prefix_of(pid).is_some_and(|prefix| prefix.starts_with(process_prefix)))
        .collect()
}

/// The parent pid of `pid`, from `/proc/{pid}/stat`: `pid (comm) state ppid ...`, where
/// `comm` may hold spaces and parentheses, hence the split at the last `)`.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn parent_of(pid: libc::pid_t) -> Option<u32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    stat.rsplit_once(')')?.1.split_whitespace().nth(1)?.parse().ok()
}

/// The value of `pid`'s `--shm_prefix` argument, if it has one.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn shm_prefix_of(pid: libc::pid_t) -> Option<String> {
    let cmdline = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let mut args = cmdline.split(|&byte| byte == 0);
    args.by_ref().find(|&arg| arg == b"--shm_prefix")?;
    args.next().map(|prefix| String::from_utf8_lossy(prefix).into_owned())
}

/// Wait until each of `pids`, children of this process, has exited, without reaping it.
/// Returns those still running at `timeout`.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn wait_until_exited(mut pids: Vec<libc::pid_t>, timeout: std::time::Duration) -> Vec<libc::pid_t> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        pids.retain(|&pid| !has_exited(pid));
        if pids.is_empty() || std::time::Instant::now() >= deadline {
            return pids;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// Whether the child `pid` has exited, leaving it to be reaped (`WNOWAIT`). One already
/// reaped by its owner is no longer a child (`ECHILD`).
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn has_exited(pid: libc::pid_t) -> bool {
    // Zeroed, as `si_pid` stays 0 when `WNOHANG` finds nothing to report.
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    let options = libc::WEXITED | libc::WNOHANG | libc::WNOWAIT;
    if unsafe { libc::waitid(libc::P_PID, pid as libc::id_t, &mut info, options) } != 0 {
        return std::io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD);
    }
    unsafe { info.si_pid() != 0 }
}

fn for_each_dev_shm_entry(mut f: impl FnMut(&str)) {
    let entries = match std::fs::read_dir(std::path::Path::new("/dev/shm")) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("Cannot scan /dev/shm for cleanup: {e}");
            return;
        }
    };
    for entry in entries.flatten() {
        if let Some(name) = entry.file_name().to_str() {
            f(name);
        }
    }
}

#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn shm_create(name: &str) {
        let c = CString::new(name).unwrap();
        unsafe {
            let fd = libc::shm_open(c.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o600);
            assert!(fd >= 0, "shm_open create failed for {name}");
            libc::ftruncate(fd, 64);
            libc::close(fd);
        }
    }
    fn shm_exists(name: &str) -> bool {
        let c = CString::new(name).unwrap();
        unsafe {
            let fd = libc::shm_open(c.as_ptr(), libc::O_RDONLY, 0);
            if fd >= 0 {
                libc::close(fd);
                true
            } else {
                false
            }
        }
    }

    /// Cleaning one prefix unlinks its own segments and no neighbour's: the other
    /// hints mode, a rank whose number it begins (1 against 10), a pid whose digits
    /// it begins (12 against 123). The janitor unlinks by `starts_with`, so this
    /// holds only because `shm_prefix_for` keeps every prefix prefix-free.
    #[test]
    fn cleaning_a_prefix_spares_every_neighbouring_prefix() {
        // Pids above Linux's limit, so no real process's segment is touched.
        let base = 400_000_000 + std::process::id();
        let target = crate::shm_prefix_for(base, 1, false);
        let neighbours = [
            crate::shm_prefix_for(base, 1, true),
            crate::shm_prefix_for(base, 10, false),
            crate::shm_prefix_for(base * 10 + 3, 1, false),
        ];
        let target_seg = format!("{target}_MT_output_0");
        let neighbour_segs: Vec<String> =
            neighbours.iter().map(|prefix| format!("{prefix}_MT_output_0")).collect();
        shm_create(&target_seg);
        neighbour_segs.iter().for_each(|seg| shm_create(seg));

        cleanup_shm_prefix(&target);

        let survivors: Vec<bool> = neighbour_segs.iter().map(|seg| shm_exists(seg)).collect();
        let target_gone = !shm_exists(&target_seg);
        for seg in neighbour_segs.iter().chain([&target_seg]) {
            let _ = unlink_shmem(seg);
        }
        assert!(target_gone, "{target_seg} must be unlinked");
        assert!(
            survivors.iter().all(|s| *s),
            "neighbours must survive: {neighbour_segs:?} {survivors:?}"
        );
    }

    /// A shell waiting on its stdin, with the arguments a service gets for `shm_prefix`.
    fn service_on(shm_prefix: &str) -> std::process::Child {
        std::process::Command::new("sh")
            .args(["-c", "read line", "sh", "--shm_prefix", shm_prefix, "--sem_prefix", shm_prefix])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .expect("spawn a stand-in service")
    }

    /// Releasing a process kills every service of its, on any rank and hints mode, and
    /// unlinks every segment and semaphore it named; another process's are left alone.
    #[test]
    fn releasing_a_process_kills_its_services_and_unlinks_its_names() {
        use std::os::unix::process::ExitStatusExt;

        // Pids above Linux's limit, apart from the other tests', so no real name is touched.
        let pid = 420_000_000 + std::process::id();
        let (ours, theirs) = (crate::process_prefix_for(pid), crate::process_prefix_for(pid + 1));
        let mut services = [service_on(&format!("{ours}0_h0")), service_on(&format!("{ours}1_h1"))];
        let mut neighbour = service_on(&format!("{theirs}0_h0"));
        let segments = [format!("{ours}0_h0_MT_output_0"), format!("{ours}1_h1_input")];
        let neighbour_segment = format!("{theirs}0_h0_input");
        segments.iter().chain([&neighbour_segment]).for_each(|segment| shm_create(segment));
        let semaphore = format!("{ours}{}_0_h0_MT_chunk_done", "a".repeat(32));
        let _semaphore = named_sem::NamedSemaphore::create(format!("/{semaphore}"), 0).unwrap();

        release_process(&ours, std::time::Duration::from_secs(10));

        let killed: Vec<_> = services
            .iter_mut()
            .map(|service| service.try_wait().unwrap().and_then(|status| status.signal()))
            .collect();
        let spared = neighbour.try_wait().unwrap().is_none();
        let _ = neighbour.kill();
        let _ = neighbour.wait();
        let neighbour_segment_kept = shm_exists(&neighbour_segment);
        let _ = unlink_shmem(&neighbour_segment);

        assert_eq!(killed, [Some(libc::SIGKILL); 2], "every service of the process is killed");
        assert!(spared, "another process's service must be left running");
        for segment in &segments {
            assert!(!shm_exists(segment), "{segment} must be unlinked");
        }
        assert!(
            !std::path::Path::new(&format!("/dev/shm/sem.{semaphore}")).exists(),
            "the semaphore must be unlinked"
        );
        assert!(neighbour_segment_kept, "another process's segment must be left alone");
    }

    #[test]
    fn cleanup_prefix_unlinks_matching_shmem_and_semaphores() {
        // Unique prefixes so the scan can never touch a real ZISK segment.
        let shm_prefix = format!("ZISK_unittest_jan_{}", std::process::id());
        let sem_prefix = format!("ZISK_unittest_jansem_{}", std::process::id());

        let seg_a = format!("{shm_prefix}_input");
        let seg_b = format!("{shm_prefix}_MT_output");
        shm_create(&seg_a);
        shm_create(&seg_b);
        assert!(shm_exists(&seg_a) && shm_exists(&seg_b));

        let sem_name = format!("/{sem_prefix}_chunk_done");
        let _sem = named_sem::NamedSemaphore::create(&sem_name, 0).unwrap();
        let sem_backing = format!("/dev/shm/sem.{sem_prefix}_chunk_done");
        assert!(std::path::Path::new(&sem_backing).exists());

        cleanup_prefix(&shm_prefix, &sem_prefix);

        assert!(!shm_exists(&seg_a), "shmem segment should be unlinked");
        assert!(!shm_exists(&seg_b), "shmem segment should be unlinked");
        assert!(!std::path::Path::new(&sem_backing).exists(), "semaphore should be unlinked");
    }
}
