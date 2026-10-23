//! What `/metrics` reads from the coordinator's state when it is scraped.
//!
//! Worker states, capacity and registered programs are read here rather than counted as they
//! change: the pool changes state in many places, and a count kept beside it drifts (a
//! disconnected worker stays in the pool, a reconnecting one does not register again).

use std::fs;
use std::time::{Duration, Instant, UNIX_EPOCH};

use zisk_cluster_common::prometheus_text::{family, sample};
use zisk_cluster_common::{WorkerExitDto, WorkerId, WorkerState, WorkerTelemetryDto};
use zisk_common::ZiskPaths;

use crate::metrics::{journal, worker_report, worker_state_code};
use crate::Coordinator;

/// Characters of a program's hash shown in labels.
const SHORT_HASH_CHARS: usize = 12;
/// A worker's report older than this is not shown: the worker stopped sending it.
const REPORT_FRESH: Duration = Duration::from_secs(90);
/// A worker's report older than this is forgotten.
const REPORT_KEPT: Duration = Duration::from_secs(600);

impl Coordinator {
    /// A program's setup was just sent to a worker: its ack will say how long it took.
    pub(crate) async fn note_setup_sent(&self, worker_id: &WorkerId, hash_id: &str) {
        self.setups_sent
            .write()
            .await
            .insert((worker_id.clone(), hash_id.to_owned()), Instant::now());
    }

    /// Seconds since the program's setup was sent to the worker, if it was and is not acked yet.
    pub(crate) async fn take_setup_sent(&self, worker_id: &WorkerId, hash_id: &str) -> Option<f64> {
        self.setups_sent
            .write()
            .await
            .remove(&(worker_id.clone(), hash_id.to_owned()))
            .map(|sent| sent.elapsed().as_secs_f64())
    }

    /// Keep a worker's telemetry, bounded, until its next report.
    pub(crate) async fn store_worker_telemetry(
        &self,
        worker_id: &WorkerId,
        report: WorkerTelemetryDto,
    ) {
        let report = worker_report::bounded(report);
        self.worker_telemetry.write().await.insert(worker_id.clone(), (Instant::now(), report));
    }

    /// A worker said, as it registered again, how its previous run ended.
    pub(crate) fn record_worker_exit(&self, worker_id: &WorkerId, exit: WorkerExitDto) {
        let exit = worker_report::bounded_exit(exit, chrono::Utc::now().timestamp());
        worker_report::record_exit(worker_id, &exit);
    }

    /// The Prometheus text read from the coordinator's state at scrape time, to append to the
    /// recorder's output. Sets the `coordinator_workers_connected`, `coordinator_compute_units`
    /// and `coordinator_registered_programs_total` gauges first, so call it before rendering the
    /// recorder.
    pub async fn render_metrics(&self) -> String {
        let mut out = String::new();

        let (connected, cc, acc) = self.workers_pool.pool_stats().await;
        metrics::gauge!("coordinator_workers_connected").set(connected as f64);
        metrics::gauge!("coordinator_compute_units", "kind" => "total")
            .set(f64::from(cc.compute_units));
        metrics::gauge!("coordinator_compute_units", "kind" => "available")
            .set(f64::from(acc.compute_units));

        let workers = self.workers_pool.worker_states().await;
        let recovering = self.pending_recovery.read().await;
        family(
            &mut out,
            "coordinator_worker_state",
            "gauge",
            "Each worker's state: 0 disconnected, 1 ready, 2 setting up, 3 computing, 4 recovering, 5 error",
        );
        for (worker_id, state) in &workers {
            let code = worker_state_code(state, recovering.contains_key(worker_id));
            sample(
                &mut out,
                "coordinator_worker_state",
                &[("worker_id", worker_id.as_str())],
                f64::from(code),
                None,
            );
        }
        drop(recovering);

        // The workers' own reports: of connected workers, and fresh.
        let mut telemetry = self.worker_telemetry.write().await;
        telemetry.retain(|_, (at, _)| at.elapsed() <= REPORT_KEPT);
        let reports: Vec<_> = telemetry
            .iter()
            .filter(|(worker_id, (at, _))| {
                at.elapsed() <= REPORT_FRESH
                    && workers
                        .iter()
                        .any(|(id, state)| id == *worker_id && *state != WorkerState::Disconnected)
            })
            .map(|(worker_id, (_, report))| (worker_id, report))
            .collect();
        worker_report::render(&mut out, &reports);
        drop(reports);
        drop(telemetry);

        let programs = registered_programs();
        metrics::gauge!("coordinator_registered_programs_total").set(programs.len() as f64);
        family(
            &mut out,
            "coordinator_program_registered_timestamp_seconds",
            "gauge",
            "When each registered program's ELF entered the coordinator's cache",
        );
        for (hash_id, bytes, registered) in &programs {
            let bytes = bytes.to_string();
            sample(
                &mut out,
                "coordinator_program_registered_timestamp_seconds",
                &[("hash_id", hash_id), ("bytes", &bytes)],
                *registered,
                None,
            );
        }

        journal().render(&mut out);
        out
    }
}

/// The programs in the coordinator's ELF cache: hash (shortened), size and when it was written.
fn registered_programs() -> Vec<(String, u64, f64)> {
    let Ok(entries) = fs::read_dir(&ZiskPaths::global().cache) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let hash = name.to_str()?.strip_suffix(".elf")?;
            let meta = entry.metadata().ok()?;
            let registered = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_secs_f64();
            Some((hash.chars().take(SHORT_HASH_CHARS).collect(), meta.len(), registered))
        })
        .collect()
}
