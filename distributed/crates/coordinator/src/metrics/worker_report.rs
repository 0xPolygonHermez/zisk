//! What workers report about their hosts and processes, bounded before it is kept and rendered
//! into `/metrics` per worker.
//!
//! The coordinator does not authenticate its workers, so a report is informational: nothing
//! acts on it, and it is bounded so that no worker can inflate the coordinator's series. Label
//! values come from closed sets or are dropped, numbers must be finite and non-negative, and
//! lists are capped.

use std::collections::{BTreeMap, HashSet};
use std::sync::{LazyLock, Mutex};

use zisk_cluster_common::prometheus_text::{family, sample};
use zisk_cluster_common::{
    CgroupUsageDto, ProcessMemoryDto, SharedMemoryDto, WorkerExitDto, WorkerId, WorkerTelemetryDto,
};

use super::journal;

/// Process roles kept per report: up to 8 ranks with 5 ASM services each, mpirun and other.
pub(crate) const MAX_ROLES: usize = 64;
/// Highest MPI rank accepted in a role or an exit.
const MAX_RANK: u32 = 255;
/// ASM services a role can name.
const ASM_SERVICES: [&str; 5] = ["mt", "mo", "moh", "mol", "rh"];
/// Shared memory segment kinds.
const SHM_KINDS: [&str; 9] = [
    "control_input",
    "control_output",
    "input",
    "output",
    "precompile",
    "ram",
    "rom",
    "semaphore",
    "other",
];
/// Why a worker ended itself.
const EXIT_REASONS: [&str; 3] = ["panic", "recovery_timeout", "recovery_failed"];
/// Longest panic location kept, in characters.
const MAX_LOCATION_CHARS: usize = 120;
/// Distinct panic locations counted; the ones after count as `other`.
const MAX_PANIC_LOCATIONS: usize = 64;
/// Oldest previous exit accepted: a worker reports the one before its restart.
const MAX_EXIT_AGE_SECONDS: i64 = 7 * 24 * 3600;

/// `report` with what is out of bounds dropped or zeroed.
pub(crate) fn bounded(report: WorkerTelemetryDto) -> WorkerTelemetryDto {
    let mut processes = BTreeMap::new();
    for p in report.processes {
        if processes.len() < MAX_ROLES && valid_role(&p.role) {
            processes.entry(p.role.clone()).or_insert(p);
        }
    }
    let mut shared_memory = BTreeMap::new();
    for m in report.shared_memory {
        if SHM_KINDS.contains(&m.kind.as_str()) {
            shared_memory.entry((m.kind.clone(), m.owner_alive)).or_insert(m);
        }
    }
    WorkerTelemetryDto {
        uptime_seconds: seconds(report.uptime_seconds),
        start_to_registration_seconds: seconds(report.start_to_registration_seconds),
        start_to_ready_seconds: seconds(report.start_to_ready_seconds),
        processes: processes.into_values().collect(),
        shared_memory: shared_memory.into_values().collect(),
        cgroup: report.cgroup.map(|c| CgroupUsageDto { cpu_seconds: seconds(c.cpu_seconds), ..c }),
    }
}

/// `exit` with an unknown reason as `other`, a location that is not `file:line` dropped, and a
/// time outside the last week, or in the future, set to `now`.
pub(crate) fn bounded_exit(exit: WorkerExitDto, now_unix_seconds: i64) -> WorkerExitDto {
    let reason =
        if EXIT_REASONS.contains(&exit.reason.as_str()) { exit.reason } else { "other".to_owned() };
    let location = if valid_location(&exit.location) { exit.location } else { String::new() };
    let at = exit.at_unix_seconds;
    let at_unix_seconds = if at <= now_unix_seconds && now_unix_seconds - at <= MAX_EXIT_AGE_SECONDS
    {
        at
    } else {
        now_unix_seconds
    };
    WorkerExitDto { reason, location, at_unix_seconds, rank: exit.rank.min(MAX_RANK) }
}

/// Every exit reason's counter at zero (see `record_coordinator_started`).
pub(crate) fn zero_exit_counters() {
    for reason in EXIT_REASONS.into_iter().chain(["other"]) {
        metrics::counter!("coordinator_worker_exits_total", "reason" => reason).increment(0);
    }
}

/// A worker reported how its previous run ended, as it registered again.
pub(crate) fn record_exit(worker_id: &WorkerId, exit: &WorkerExitDto) {
    metrics::counter!("coordinator_worker_exits_total", "reason" => exit.reason.clone())
        .increment(1);
    if exit.reason == "panic" {
        metrics::counter!("coordinator_worker_panics_total", "location" => panic_location(&exit.location))
            .increment(1);
    }
    let at = chrono::DateTime::from_timestamp(exit.at_unix_seconds, 0)
        .map(|t| t.format("%H:%M:%S").to_string())
        .unwrap_or_default();
    let location =
        if exit.location.is_empty() { String::new() } else { format!(" at {}", exit.location) };
    journal().event(
        "worker exited",
        format!("{worker_id} rank {}: {}{location} ({at})", exit.rank, exit.reason),
    );
}

/// The `location` label of a panic: its location while fewer than [`MAX_PANIC_LOCATIONS`] have
/// been seen, else `other`.
fn panic_location(location: &str) -> String {
    static SEEN: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);
    if location.is_empty() {
        return "unknown".to_owned();
    }
    let mut seen = SEEN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if seen.contains(location) || seen.len() < MAX_PANIC_LOCATIONS {
        seen.insert(location.to_owned());
        location.to_owned()
    } else {
        "other".to_owned()
    }
}

fn seconds(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

/// `rankN`, `asm-<service>-rankN`, `mpirun` or `other`.
fn valid_role(role: &str) -> bool {
    let rank = |r: &str| r.parse::<u32>().is_ok_and(|n| n <= MAX_RANK) && !r.starts_with('+');
    match role {
        "mpirun" | "other" => true,
        _ => match role.strip_prefix("asm-") {
            Some(rest) => rest
                .split_once("-rank")
                .is_some_and(|(service, r)| ASM_SERVICES.contains(&service) && rank(r)),
            None => role.strip_prefix("rank").is_some_and(rank),
        },
    }
}

/// `path:line` made of path characters, at most [`MAX_LOCATION_CHARS`] long.
fn valid_location(location: &str) -> bool {
    location.len() <= MAX_LOCATION_CHARS
        && location.rsplit_once(':').is_some_and(|(path, line)| {
            !path.is_empty()
                && line.parse::<u32>().is_ok()
                && path.chars().all(|c| c.is_ascii_alphanumeric() || "/._-".contains(c))
        })
}

/// Append the reports of the workers in `reports` to a `/metrics` body.
pub(crate) fn render(out: &mut String, reports: &[(&WorkerId, &WorkerTelemetryDto)]) {
    family(
        out,
        "coordinator_worker_uptime_seconds",
        "gauge",
        "Seconds since each worker process started, as it reports",
    );
    for (worker_id, r) in reports {
        sample(
            out,
            "coordinator_worker_uptime_seconds",
            &[("worker_id", worker_id.as_str())],
            r.uptime_seconds,
            None,
        );
    }

    family(out, "coordinator_worker_start_to_seconds", "gauge", "Seconds from each worker's start to its first registration and to its first program set up, as it reports");
    for (worker_id, r) in reports {
        for (milestone, value) in
            [("registration", r.start_to_registration_seconds), ("ready", r.start_to_ready_seconds)]
        {
            if value > 0.0 {
                sample(
                    out,
                    "coordinator_worker_start_to_seconds",
                    &[("worker_id", worker_id.as_str()), ("milestone", milestone)],
                    value,
                    None,
                );
            }
        }
    }

    family(out, "coordinator_worker_process_memory_bytes", "gauge", "Each worker's processes' memory by role (rankN, asm-<service>-rankN, mpirun) and kind (rss, anon, file, shmem), as it reports");
    for (worker_id, r) in reports {
        for p in &r.processes {
            render_process(out, worker_id, p);
        }
    }

    family(out, "coordinator_worker_shm_bytes", "gauge", "The ZisK shared memory on each worker's host, by segment kind and whether its owner runs, as it reports");
    for (worker_id, r) in reports {
        for m in &r.shared_memory {
            render_shm(out, "coordinator_worker_shm_bytes", worker_id, m, m.bytes as f64);
        }
    }
    family(out, "coordinator_worker_shm_segments", "gauge", "The ZisK shared memory segments on each worker's host, by kind and whether their owner runs, as it reports");
    for (worker_id, r) in reports {
        for m in &r.shared_memory {
            render_shm(out, "coordinator_worker_shm_segments", worker_id, m, f64::from(m.segments));
        }
    }

    family(
        out,
        "coordinator_worker_cgroup_memory_bytes",
        "gauge",
        "Each worker's cgroup memory: current, peak, anon, file, shmem, as it reports",
    );
    for (worker_id, r) in reports {
        if let Some(c) = &r.cgroup {
            for (kind, value) in [
                ("current", c.memory_current_bytes),
                ("peak", c.memory_peak_bytes),
                ("anon", c.anon_bytes),
                ("file", c.file_bytes),
                ("shmem", c.shmem_bytes),
            ] {
                sample(
                    out,
                    "coordinator_worker_cgroup_memory_bytes",
                    &[("worker_id", worker_id.as_str()), ("kind", kind)],
                    value as f64,
                    None,
                );
            }
        }
    }
    family(
        out,
        "coordinator_worker_cgroup_cpu_seconds_total",
        "counter",
        "Each worker's cgroup CPU time, as it reports",
    );
    for (worker_id, r) in reports {
        if let Some(c) = &r.cgroup {
            sample(
                out,
                "coordinator_worker_cgroup_cpu_seconds_total",
                &[("worker_id", worker_id.as_str())],
                c.cpu_seconds,
                None,
            );
        }
    }
}

fn render_process(out: &mut String, worker_id: &WorkerId, p: &ProcessMemoryDto) {
    for (kind, value) in [
        ("rss", p.rss_bytes),
        ("anon", p.anon_bytes),
        ("file", p.file_bytes),
        ("shmem", p.shmem_bytes),
    ] {
        sample(
            out,
            "coordinator_worker_process_memory_bytes",
            &[("worker_id", worker_id.as_str()), ("role", &p.role), ("kind", kind)],
            value as f64,
            None,
        );
    }
}

fn render_shm(out: &mut String, name: &str, worker_id: &WorkerId, m: &SharedMemoryDto, value: f64) {
    let owner_alive = if m.owner_alive { "true" } else { "false" };
    sample(
        out,
        name,
        &[("worker_id", worker_id.as_str()), ("kind", &m.kind), ("owner_alive", owner_alive)],
        value,
        None,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process(role: &str) -> ProcessMemoryDto {
        ProcessMemoryDto { role: role.to_owned(), rss_bytes: 1, ..Default::default() }
    }

    #[test]
    fn only_known_roles_are_kept() {
        for role in ["rank0", "rank3", "asm-mt-rank0", "asm-moh-rank7", "mpirun", "other"] {
            assert!(valid_role(role), "{role}");
        }
        for role in ["rank", "rank-1", "rank+1", "rank256", "asm-xx-rank0", "asm-mt-", "bash", "r"]
        {
            assert!(!valid_role(role), "{role}");
        }
    }

    #[test]
    fn a_report_is_bounded_before_it_is_kept() {
        let mut processes: Vec<_> = (0..200).map(|i| process(&format!("rank{i}"))).collect();
        processes.push(process("evil\"label"));
        processes.push(process("rank0"));
        let report = WorkerTelemetryDto {
            uptime_seconds: f64::NAN,
            start_to_registration_seconds: -3.0,
            start_to_ready_seconds: 12.5,
            processes,
            shared_memory: vec![
                SharedMemoryDto { kind: "rom".into(), owner_alive: true, bytes: 8, segments: 2 },
                SharedMemoryDto { kind: "../etc".into(), owner_alive: true, bytes: 8, segments: 2 },
            ],
            cgroup: Some(CgroupUsageDto { cpu_seconds: f64::INFINITY, ..Default::default() }),
        };
        let kept = bounded(report);
        assert_eq!(kept.uptime_seconds, 0.0);
        assert_eq!(kept.start_to_registration_seconds, 0.0);
        assert_eq!(kept.start_to_ready_seconds, 12.5);
        assert_eq!(kept.processes.len(), MAX_ROLES);
        assert!(kept.processes.iter().all(|p| valid_role(&p.role)));
        assert_eq!(kept.shared_memory.len(), 1);
        assert_eq!(kept.cgroup.unwrap().cpu_seconds, 0.0);
    }

    #[test]
    fn an_exit_is_bounded() {
        let now = 1_800_000_000;
        let exit = bounded_exit(
            WorkerExitDto {
                reason: "pwned".into(),
                location: "src/a.rs:12\n".into(),
                at_unix_seconds: now + 60,
                rank: 9999,
            },
            now,
        );
        assert_eq!(
            exit,
            WorkerExitDto {
                reason: "other".into(),
                location: String::new(),
                at_unix_seconds: now,
                rank: MAX_RANK
            }
        );

        let exit = bounded_exit(
            WorkerExitDto {
                reason: "panic".into(),
                location: "state-machines/mem-cpp/src/lib.rs:88".into(),
                at_unix_seconds: now - 30,
                rank: 2,
            },
            now,
        );
        assert_eq!(exit.reason, "panic");
        assert_eq!(exit.location, "state-machines/mem-cpp/src/lib.rs:88");
        assert_eq!(exit.at_unix_seconds, now - 30);
    }

    #[test]
    fn a_report_renders_per_worker() {
        let worker = WorkerId::from("w1".to_owned());
        let report = WorkerTelemetryDto {
            uptime_seconds: 60.0,
            start_to_registration_seconds: 20.0,
            processes: vec![process("rank0")],
            cgroup: Some(CgroupUsageDto {
                memory_current_bytes: 5,
                cpu_seconds: 2.5,
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut out = String::new();
        render(&mut out, &[(&worker, &report)]);
        for line in [
            r#"coordinator_worker_uptime_seconds{worker_id="w1"} 60"#,
            r#"coordinator_worker_start_to_seconds{worker_id="w1",milestone="registration"} 20"#,
            r#"coordinator_worker_process_memory_bytes{worker_id="w1",role="rank0",kind="rss"} 1"#,
            r#"coordinator_worker_cgroup_memory_bytes{worker_id="w1",kind="current"} 5"#,
            r#"coordinator_worker_cgroup_cpu_seconds_total{worker_id="w1"} 2.5"#,
        ] {
            assert!(out.lines().any(|l| l == line), "{line} missing in\n{out}");
        }
        // Not reported yet: no ready milestone.
        assert!(!out.contains(r#"milestone="ready""#));
    }
}
