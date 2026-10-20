//! What `/metrics` reads from the coordinator's state when it is scraped.
//!
//! Worker states, capacity and registered programs are read here rather than counted as they
//! change: the pool changes state in many places, and a count kept beside it drifts (a
//! disconnected worker stays in the pool, a reconnecting one does not register again).

use std::fs;
use std::time::UNIX_EPOCH;

use zisk_common::ZiskPaths;

use crate::metrics::{gauge_family, gauge_sample, journal, worker_state_code};
use crate::Coordinator;

/// Characters of a program's hash shown in labels.
const SHORT_HASH_CHARS: usize = 12;

impl Coordinator {
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
        gauge_family(
            &mut out,
            "coordinator_worker_state",
            "Each worker's state: 0 disconnected, 1 ready, 2 setting up, 3 computing, 4 recovering, 5 error",
        );
        for (worker_id, state) in &workers {
            let code = worker_state_code(state, recovering.contains_key(worker_id));
            gauge_sample(
                &mut out,
                "coordinator_worker_state",
                &[("worker_id", worker_id.as_str())],
                f64::from(code),
            );
        }
        drop(recovering);

        let programs = registered_programs();
        metrics::gauge!("coordinator_registered_programs_total").set(programs.len() as f64);
        gauge_family(
            &mut out,
            "coordinator_program_registered_timestamp_seconds",
            "When each registered program's ELF entered the coordinator's cache",
        );
        for (hash_id, bytes, registered) in &programs {
            let bytes = bytes.to_string();
            gauge_sample(
                &mut out,
                "coordinator_program_registered_timestamp_seconds",
                &[("hash_id", hash_id), ("bytes", &bytes)],
                *registered,
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
