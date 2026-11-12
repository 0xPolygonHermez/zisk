//! What the worker reports to its coordinator about its host and processes.
//!
//! Rank 0 sends it with its heartbeat acknowledgements, so a coordinator sees every worker it
//! can reach, wherever it runs and whoever runs it; `telemetry.enabled = false` (or
//! `--no-telemetry`) turns it off. A thread samples it every `telemetry.interval_seconds`, so the
//! event loop only copies the last sample. It reads `/proc`, `/sys/fs/cgroup` and `/dev/shm` on
//! Linux (elsewhere it reports only the worker's start-up times), nothing a proof uses.
//!
//! When the worker ends itself (a panic or a failed recovery aborts every MPI rank), the rank
//! that ends it first writes why to a file in the cache directory; the next run reports it as it
//! registers.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tracing::warn;
use zisk_cluster_api::{WorkerExit, WorkerTelemetry};
use zisk_common::ZiskPaths;

static STARTED: LazyLock<Instant> = LazyLock::new(Instant::now);
static REGISTERED: OnceLock<f64> = OnceLock::new();
static READY: OnceLock<f64> = OnceLock::new();
static LATEST: Mutex<Option<WorkerTelemetry>> = Mutex::new(None);

/// Prefix of the files a rank writes as it ends the worker.
const EXIT_FILE_PREFIX: &str = "zisk-worker-exit-rank";

/// Note the process's start, from which uptime and start-up times count. Call it first thing.
pub fn mark_process_start() {
    LazyLock::force(&STARTED);
}

/// The worker's first registration was accepted.
pub(crate) fn mark_registered() {
    let _ = REGISTERED.set(STARTED.elapsed().as_secs_f64());
}

/// The worker finished setting up its first program.
pub(crate) fn mark_ready() {
    let _ = READY.set(STARTED.elapsed().as_secs_f64());
}

/// Sample the host and processes every `interval`, on a thread of its own.
pub(crate) fn start_sampling(interval: Duration) {
    let spawned = std::thread::Builder::new().name("zisk-telemetry".into()).spawn(move || loop {
        let sample = host::sample();
        *LATEST.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(sample);
        std::thread::sleep(interval);
    });
    if let Err(e) = spawned {
        warn!("telemetry: could not start its sampling thread: {e}");
    }
}

/// The last sample, with the current uptime and start-up times.
pub(crate) fn latest() -> WorkerTelemetry {
    let mut report =
        LATEST.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone().unwrap_or_default();
    report.uptime_seconds = STARTED.elapsed().as_secs_f64();
    report.start_to_registration_seconds = REGISTERED.get().copied().unwrap_or(0.0);
    report.start_to_ready_seconds = READY.get().copied().unwrap_or(0.0);
    report
}

/// Why the worker ends itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExitReason<'a> {
    /// A panic, at a source location `file:line`.
    Panic { location: &'a str },
    /// The cancellation handshake did not finish in time.
    RecoveryTimeout,
    /// The cancellation handshake failed.
    RecoveryFailed,
}

impl ExitReason<'_> {
    fn name(&self) -> &'static str {
        match self {
            ExitReason::Panic { .. } => "panic",
            ExitReason::RecoveryTimeout => "recovery_timeout",
            ExitReason::RecoveryFailed => "recovery_failed",
        }
    }
}

/// Write why this rank ends the worker, for the next run to report. Best effort: it is called on
/// the way to an abort.
pub(crate) fn record_exit(rank: i32, reason: ExitReason<'_>) {
    write_exit(&ZiskPaths::global().cache, rank, reason, unix_now());
}

/// How the previous run ended, if it ended itself; the files that said so are removed.
pub(crate) fn take_previous_exit() -> Option<WorkerExit> {
    take_exit(&ZiskPaths::global().cache)
}

fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

fn exit_file(dir: &Path, rank: i32) -> PathBuf {
    dir.join(format!("{EXIT_FILE_PREFIX}{rank}"))
}

fn write_exit(dir: &Path, rank: i32, reason: ExitReason<'_>, at: i64) {
    let location = match reason {
        ExitReason::Panic { location } => location,
        _ => "",
    };
    // One line, tab-separated; a location never holds a tab or a line break.
    let location: String = location.chars().filter(|c| !c.is_control()).collect();
    let _ = fs::write(exit_file(dir, rank), format!("{}\t{location}\t{at}\n", reason.name()));
}

/// The earliest exit written in `dir` (the rank that ended the worker first), removing them all.
fn take_exit(dir: &Path) -> Option<WorkerExit> {
    let entries = fs::read_dir(dir).ok()?;
    let mut exits = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        let Some(rank) = name.to_str().and_then(|n| n.strip_prefix(EXIT_FILE_PREFIX)) else {
            continue;
        };
        let rank = rank.parse::<u32>().ok();
        let text = fs::read_to_string(entry.path()).unwrap_or_default();
        let _ = fs::remove_file(entry.path());
        let mut fields = text.trim_end().splitn(3, '\t');
        let (Some(reason), Some(location), Some(at), Some(rank)) =
            (fields.next(), fields.next(), fields.next().and_then(|a| a.parse().ok()), rank)
        else {
            continue;
        };
        exits.push(WorkerExit {
            reason: reason.to_owned(),
            location: location.to_owned(),
            at_unix_seconds: at,
            rank,
        });
    }
    exits.into_iter().min_by_key(|exit| exit.at_unix_seconds)
}

#[cfg(target_os = "linux")]
mod host {
    //! The worker's processes, the host's ZisK shared memory and the worker's cgroup, from
    //! `/proc`, `/dev/shm` and `/sys/fs/cgroup`.

    use std::collections::{BTreeMap, HashMap};
    use std::fs;
    use std::os::unix::fs::MetadataExt;
    use std::path::{Path, PathBuf};

    use zisk_cluster_api::{CgroupUsage, ProcessMemory, SharedMemory, WorkerTelemetry};

    pub(super) fn sample() -> WorkerTelemetry {
        let cgroup = own_cgroup();
        WorkerTelemetry {
            processes: processes(cgroup.as_deref()),
            shared_memory: shared_memory(),
            cgroup: cgroup.as_deref().and_then(cgroup_usage),
            ..Default::default()
        }
    }

    /// The directory of this process's cgroup (v2), unless it is the root one.
    fn own_cgroup() -> Option<PathBuf> {
        let text = fs::read_to_string("/proc/self/cgroup").ok()?;
        let path = parse_cgroup_path(&text)?;
        Some(Path::new("/sys/fs/cgroup").join(path.trim_start_matches('/')))
    }

    pub(super) fn parse_cgroup_path(text: &str) -> Option<&str> {
        text.lines().find_map(|line| line.strip_prefix("0::")).filter(|path| *path != "/")
    }

    /// The memory of the processes in `cgroup` (or of this process alone), summed by role.
    fn processes(cgroup: Option<&Path>) -> Vec<ProcessMemory> {
        let pids: Vec<u32> = cgroup
            .and_then(|dir| fs::read_to_string(dir.join("cgroup.procs")).ok())
            .map(|text| text.lines().filter_map(|l| l.trim().parse().ok()).collect())
            .unwrap_or_else(|| vec![std::process::id()]);
        let procs: Vec<Proc> = pids.into_iter().filter_map(Proc::read).collect();
        let ranks: HashMap<u32, u32> = procs
            .iter()
            .filter(|p| p.exe.ends_with("zisk-worker"))
            .map(|p| (p.pid, p.rank.unwrap_or(0)))
            .collect();
        let mut by_role: BTreeMap<String, ProcessMemory> = BTreeMap::new();
        for p in &procs {
            let role = role(&p.exe, p.rank, ranks.get(&p.ppid).copied());
            let entry = by_role
                .entry(role.clone())
                .or_insert_with(|| ProcessMemory { role, ..Default::default() });
            entry.rss_bytes += p.memory.rss_bytes;
            entry.anon_bytes += p.memory.anon_bytes;
            entry.file_bytes += p.memory.file_bytes;
            entry.shmem_bytes += p.memory.shmem_bytes;
        }
        by_role.into_values().collect()
    }

    /// A process's role: `rankN` for a worker rank, `asm-<service>-rankN` for an ASM service of
    /// rank N (its parent), `mpirun` for the launcher, `other` otherwise.
    pub(super) fn role(exe: &str, rank: Option<u32>, parent_rank: Option<u32>) -> String {
        if exe.ends_with("zisk-worker") {
            return format!("rank{}", rank.unwrap_or(0));
        }
        if let Some(service) = asm_service(exe) {
            if let Some(rank) = parent_rank.or(rank) {
                return format!("asm-{service}-rank{rank}");
            }
        }
        match exe {
            "mpirun" | "mpiexec" | "orterun" | "prterun" | "prte" => "mpirun".to_owned(),
            _ => "other".to_owned(),
        }
    }

    /// The ASM service an executable is (`<hash>-<service>.bin`).
    fn asm_service(exe: &str) -> Option<&'static str> {
        ["mt", "mo", "moh", "mol", "rh"]
            .into_iter()
            .find(|service| exe.ends_with(&format!("-{service}.bin")))
    }

    struct Proc {
        pid: u32,
        ppid: u32,
        exe: String,
        rank: Option<u32>,
        memory: ProcessMemory,
    }

    impl Proc {
        fn read(pid: u32) -> Option<Proc> {
            let dir = PathBuf::from(format!("/proc/{pid}"));
            let cmdline = fs::read(dir.join("cmdline")).ok()?;
            let argv0 = cmdline.split(|&b| b == 0).next().unwrap_or_default();
            let exe = String::from_utf8_lossy(argv0);
            let exe = exe.rsplit('/').next().unwrap_or_default().to_owned();
            let (memory, ppid) = parse_status(&fs::read_to_string(dir.join("status")).ok()?);
            let environ = fs::read(dir.join("environ")).unwrap_or_default();
            Some(Proc { pid, ppid, exe, rank: mpi_rank(&environ), memory })
        }
    }

    /// Memory and parent PID from `/proc/<pid>/status`.
    pub(super) fn parse_status(text: &str) -> (ProcessMemory, u32) {
        let mut memory = ProcessMemory::default();
        let mut ppid = 0;
        for line in text.lines() {
            let Some((key, value)) = line.split_once(':') else { continue };
            let kb = || value.split_whitespace().next().and_then(|v| v.parse::<u64>().ok());
            match key {
                "VmRSS" => memory.rss_bytes = kb().unwrap_or(0) * 1024,
                "RssAnon" => memory.anon_bytes = kb().unwrap_or(0) * 1024,
                "RssFile" => memory.file_bytes = kb().unwrap_or(0) * 1024,
                "RssShmem" => memory.shmem_bytes = kb().unwrap_or(0) * 1024,
                "PPid" => ppid = value.trim().parse().unwrap_or(0),
                _ => {}
            }
        }
        (memory, ppid)
    }

    /// The MPI rank in a process's environment (Open MPI, MPICH or PMIx).
    pub(super) fn mpi_rank(environ: &[u8]) -> Option<u32> {
        environ.split(|&b| b == 0).find_map(|var| {
            let var = std::str::from_utf8(var).ok()?;
            ["OMPI_COMM_WORLD_RANK=", "PMI_RANK=", "PMIX_RANK="]
                .iter()
                .find_map(|prefix| var.strip_prefix(prefix))
                .and_then(|rank| rank.parse().ok())
        })
    }

    /// The host's ZisK shared memory, by segment kind and whether its owner still runs.
    fn shared_memory() -> Vec<SharedMemory> {
        let Ok(entries) = fs::read_dir("/dev/shm") else { return Vec::new() };
        let mut by_kind: BTreeMap<(&'static str, bool), SharedMemory> = BTreeMap::new();
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name();
            let Some((kind, owner)) = name.to_str().and_then(shm_segment) else { continue };
            let Ok(meta) = entry.metadata() else { continue };
            let alive = Path::new(&format!("/proc/{owner}")).exists();
            let m = by_kind.entry((kind, alive)).or_insert_with(|| SharedMemory {
                kind: kind.to_owned(),
                owner_alive: alive,
                ..Default::default()
            });
            m.bytes += meta.blocks() * 512;
            m.segments += 1;
        }
        by_kind.into_values().collect()
    }

    /// A ZisK segment's kind and owner PID, from its name (`ZISK_<pid>_…_<kind>` or
    /// `sem.ZISK_<pid>_…`).
    pub(super) fn shm_segment(name: &str) -> Option<(&'static str, u32)> {
        let semaphore = name.starts_with("sem.");
        let rest = name.strip_prefix("sem.").unwrap_or(name).strip_prefix("ZISK_")?;
        let owner = rest.split('_').next()?.parse().ok()?;
        if semaphore {
            return Some(("semaphore", owner));
        }
        let kind =
            ["control_input", "control_output", "precompile", "input", "output", "rom", "ram"]
                .into_iter()
                .find(|kind| {
                    name.ends_with(kind)
                        || name.rsplit_once('_').is_some_and(|(head, tail)| {
                            tail.parse::<u32>().is_ok() && head.ends_with(kind)
                        })
                })
                .unwrap_or("other");
        Some((kind, owner))
    }

    /// The cgroup's memory and CPU, when its files are readable.
    fn cgroup_usage(dir: &Path) -> Option<CgroupUsage> {
        let read = |file: &str| fs::read_to_string(dir.join(file)).ok();
        let current = read("memory.current")?.trim().parse().ok()?;
        let stat = read("memory.stat").unwrap_or_default();
        let cpu = read("cpu.stat").unwrap_or_default();
        Some(CgroupUsage {
            memory_current_bytes: current,
            memory_peak_bytes: read("memory.peak").and_then(|p| p.trim().parse().ok()).unwrap_or(0),
            anon_bytes: stat_value(&stat, "anon"),
            file_bytes: stat_value(&stat, "file"),
            shmem_bytes: stat_value(&stat, "shmem"),
            cpu_seconds: stat_value(&cpu, "usage_usec") as f64 / 1e6,
        })
    }

    /// A `key value` line's value from a cgroup stat file.
    pub(super) fn stat_value(text: &str, key: &str) -> u64 {
        text.lines()
            .find_map(|line| line.strip_prefix(key)?.strip_prefix(' ')?.trim().parse().ok())
            .unwrap_or(0)
    }
}

#[cfg(not(target_os = "linux"))]
mod host {
    use zisk_cluster_api::WorkerTelemetry;

    pub(super) fn sample() -> WorkerTelemetry {
        WorkerTelemetry::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_rank_to_end_the_worker_is_reported_and_the_files_go() {
        let dir = std::env::temp_dir().join(format!("zisk-exit-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        write_exit(&dir, 2, ExitReason::Panic { location: "src/a.rs:7" }, 1_000);
        write_exit(&dir, 0, ExitReason::RecoveryTimeout, 1_005);
        let exit = take_exit(&dir).unwrap();
        assert_eq!(
            exit,
            WorkerExit {
                reason: "panic".into(),
                location: "src/a.rs:7".into(),
                at_unix_seconds: 1_000,
                rank: 2
            }
        );
        assert_eq!(take_exit(&dir), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(target_os = "linux")]
    mod linux {
        use super::super::host::*;

        #[test]
        fn status_gives_memory_and_parent() {
            let (memory, ppid) = parse_status(
                "Name:\tzisk-worker\nPPid:\t4242\nVmRSS:\t   2048 kB\nRssAnon:\t 1024 kB\n\
                 RssFile:\t  512 kB\nRssShmem:\t  512 kB\n",
            );
            assert_eq!(ppid, 4242);
            assert_eq!(memory.rss_bytes, 2048 * 1024);
            assert_eq!(memory.anon_bytes, 1024 * 1024);
            assert_eq!(memory.shmem_bytes, 512 * 1024);
        }

        #[test]
        fn roles_name_ranks_their_asm_services_and_mpirun() {
            assert_eq!(role("zisk-worker", Some(3), None), "rank3");
            assert_eq!(role("7db408b63ca7f8e-moh.bin", None, Some(1)), "asm-moh-rank1");
            assert_eq!(role("abc-mt.bin", Some(2), None), "asm-mt-rank2");
            assert_eq!(role("mpirun", None, None), "mpirun");
            assert_eq!(role("bash", None, None), "other");
        }

        #[test]
        fn the_rank_comes_from_the_mpi_environment() {
            assert_eq!(mpi_rank(b"HOME=/x\0OMPI_COMM_WORLD_RANK=3\0"), Some(3));
            assert_eq!(mpi_rank(b"PMI_RANK=1\0"), Some(1));
            assert_eq!(mpi_rank(b"HOME=/x\0"), None);
        }

        #[test]
        fn segments_are_classified_by_kind_and_owner() {
            assert_eq!(shm_segment("ZISK_2733564_3_h0_RH_rom"), Some(("rom", 2733564)));
            assert_eq!(shm_segment("ZISK_2733564_3_h0_MT_output_0"), Some(("output", 2733564)));
            assert_eq!(shm_segment("ZISK_7_3_h0_control_input"), Some(("control_input", 7)));
            assert_eq!(shm_segment("sem.ZISK_9_abc_0_h0_MO_chunk_done"), Some(("semaphore", 9)));
            assert_eq!(shm_segment("ZISK_9_odd"), Some(("other", 9)));
            assert_eq!(shm_segment("__KMP_REGISTERED_LIB_1003"), None);
        }

        #[test]
        fn the_cgroup_path_and_stats_are_read() {
            assert_eq!(
                parse_cgroup_path("0::/system.slice/zisk-worker.service\n"),
                Some("/system.slice/zisk-worker.service")
            );
            assert_eq!(parse_cgroup_path("0::/\n"), None);
            assert_eq!(stat_value("anon 10\nanon_thp 5\nfile 3\n", "anon"), 10);
            assert_eq!(stat_value("usage_usec 2500000\n", "usage_usec"), 2_500_000);
        }
    }
}
