use std::{
    collections::HashMap,
    fs, process,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};

use crossbeam_queue::SegQueue;
use serde::{Deserialize, Serialize};

use crate::Stats;

/// Trait for types that can be converted to a stats ID.
/// Implemented for `u64` (raw ID), `StatsScope`, and references `&T` where `T: IntoStatsId`.
pub trait IntoStatsId {
    /// Converts the implementing type into a `u64` stats ID.
    fn as_stats_id(&self) -> u64;
}

impl IntoStatsId for u64 {
    #[inline]
    fn as_stats_id(&self) -> u64 {
        *self
    }
}

impl IntoStatsId for StatsScope {
    #[inline]
    fn as_stats_id(&self) -> u64 {
        self.id()
    }
}

impl<T: IntoStatsId> IntoStatsId for &T {
    #[inline]
    fn as_stats_id(&self) -> u64 {
        (*self).as_stats_id()
    }
}

/// Creates a new stats scope (StatsScope) and emits a Begin event.
/// When `stats` feature is disabled, creates a zero-sized StatsScope.
///
/// # Usage
/// ```ignore
/// stats_begin!(self.stats, 0, parent_scope, "PARENT_OP", 0);
/// stats_begin!(self.stats, &parent_scope, child_scope, "CHILD_OP", 0);
/// // ... work ...
/// stats_end!(self.stats, &child_scope);
/// stats_end!(self.stats, &parent_scope);
/// ```
#[cfg(feature = "stats")]
#[macro_export]
macro_rules! stats_begin {
    ($stats:expr, $parent:expr, $scope:ident, $name:expr, $index:expr) => {
        let $scope = $crate::StatsScope::new(
            $crate::IntoStatsId::as_stats_id(&$parent),
            $stats.next_id(),
            $name,
            $index,
        );
        $stats.add_stat(
            $scope.parent_id(),
            $scope.id(),
            $name,
            $index,
            $crate::ExecutorStatsEvent::Begin,
        );
    };
}

/// This macro generates code related to starting a stats scope.
#[cfg(not(feature = "stats"))]
#[macro_export]
macro_rules! stats_begin {
    ($stats:expr, $parent:expr, $scope:ident, $name:expr, $index:expr) => {
        let $scope = $crate::StatsScope;
    };
}

/// Ends a stats scope with an End event.
/// Uses name and index from the scope (passed to stats_begin).
/// When `stats` feature is disabled, this generates no code.
///
/// # Usage
/// ```ignore
/// stats_begin!(self.stats, &parent_scope, child_scope, "CHILD_OP", 0);
/// // ... work ...
/// stats_end!(self.stats, &child_scope);
/// ```
#[cfg(feature = "stats")]
#[macro_export]
macro_rules! stats_end {
    ($stats:expr, $scope:expr) => {
        $stats.add_stat(
            $scope.parent_id(),
            $scope.id(),
            $scope.name(),
            $scope.index(),
            $crate::ExecutorStatsEvent::End,
        );
    };
}

/// This macro generates code related to ending a stats scope.
#[cfg(not(feature = "stats"))]
#[macro_export]
macro_rules! stats_end {
    ($stats:expr, $scope:expr) => {};
}

/// Records a stats mark event (single point in time, not a scope).
/// When `stats` feature is disabled, this generates no code.
///
/// # Usage
/// ```ignore
/// stats_mark!(self.stats, &parent_scope, "CHECKPOINT_NAME", index);
/// ```
#[cfg(feature = "stats")]
#[macro_export]
macro_rules! stats_mark {
    ($stats:expr, $parent:expr, $name:expr, $index:expr) => {
        let __mark_id = $stats.next_id();
        $stats.add_stat(
            $crate::IntoStatsId::as_stats_id(&$parent),
            __mark_id,
            $name,
            $index,
            $crate::ExecutorStatsEvent::Mark,
        );
    };
}

/// This macro generates code related to recording a stats mark event.
#[cfg(not(feature = "stats"))]
#[macro_export]
macro_rules! stats_mark {
    ($stats:expr, $parent:expr, $name:expr, $index:expr) => {};
}

/// Stats scope that holds scope information (parent_id, id, name, index).
/// Created by `stats_begin!` macro, ended by `stats_end!` macro.
/// When `stats` feature is disabled, this is a zero-sized type.
///
/// # Usage
/// ```ignore
/// stats_begin!(self.stats, 0, parent_scope, "PARENT", 0);
/// stats_begin!(self.stats, &parent_scope, child_scope, "CHILD", 0);
/// // ... work ...
/// stats_end!(self.stats, &child_scope);
/// stats_end!(self.stats, &parent_scope);
/// ```
#[cfg(feature = "stats")]
pub struct StatsScope {
    parent_id: u64,
    id: u64,
    name: &'static str,
    index: usize,
}

#[cfg(feature = "stats")]
impl StatsScope {
    /// Creates a new stats scope. Does NOT emit Begin - use `stats_begin!` macro instead.
    #[inline]
    pub fn new(parent_id: u64, id: u64, name: &'static str, index: usize) -> Self {
        Self { parent_id, id, name, index }
    }

    /// Returns the parent ID of the stats scope.
    #[inline]
    pub fn parent_id(&self) -> u64 {
        self.parent_id
    }

    /// Returns the ID of the stats scope.
    #[inline]
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Returns the name of the stats scope.
    #[inline]
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the index of the stats scope.
    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }
}

/// Zero-sized type when stats feature is disabled. All methods are no-ops.
#[cfg(not(feature = "stats"))]
pub struct StatsScope;

#[cfg(not(feature = "stats"))]
impl StatsScope {
    /// Returns a zero ID for the parent scope when stats are disabled.
    #[inline]
    pub fn parent_id(&self) -> u64 {
        0
    }

    /// Returns a zero ID for the current scope when stats are disabled.
    #[inline]
    pub fn id(&self) -> u64 {
        0
    }

    /// Returns an empty name when stats are disabled.
    #[inline]
    pub fn name(&self) -> &'static str {
        ""
    }

    /// Returns a zero index when stats are disabled.
    #[inline]
    pub fn index(&self) -> usize {
        0
    }
}

/// The `ExecutorStatsEvent` enum defines the types of events that can be recorded in the executor stats,
/// including the beginning and end of a scope, as well as mark events for specific checkpoints.
#[derive(Debug, Clone)]
pub enum ExecutorStatsEvent {
    /// Indicates the beginning of a stats scope.
    Begin,
    /// Indicates the end of a stats scope.
    End,
    /// Represents a mark event, which is a single point in time (not a scope) used for recording specific checkpoints or events.
    Mark,
}

#[derive(Debug, Clone)]
struct ExecutorStatsEntry {
    parent_id: u64,
    id: u64,
    name: &'static str,
    index: usize,
    event: ExecutorStatsEvent,
    timestamp: Instant,
}

/// The `ExecutorStats` struct is responsible for collecting and managing statistics
/// related to the execution of tasks or operations.
///
/// All recording operations (`add_stat`, `next_id`) are lock-free so they can be
/// called concurrently from rayon worker threads without serializing on a global
/// mutex — important because lock contention would distort the very timings this
/// profiler measures. Entries are pushed onto a lock-free [`SegQueue`]; the cold
/// reporting paths (`store_stats`, `print_stats`) drain that queue into `finalized`
/// once, so they remain non-destructive and may be called repeatedly.
#[derive(Debug, Default)]
pub struct ExecutorStats {
    /// Reference instant for relative timestamps. Touched only on the cold
    /// set/reset/report paths, so a plain `Mutex` is fine (never contended).
    start_time: Mutex<Option<Instant>>,
    /// Monotonic unique-id source. Lock-free.
    last_id: AtomicU64,
    /// Lock-free queue of not-yet-reported entries (the hot path pushes here).
    pending: SegQueue<ExecutorStatsEntry>,
    /// Entries already drained from `pending` by a reporting call, kept so that
    /// `store_stats`/`print_stats` are non-destructive and idempotent.
    finalized: Mutex<Vec<ExecutorStatsEntry>>,
    /// A mapping of witness statistics, where the key is an airgroup ID and the value is a `Stats` struct containing relevant metrics.
    witness_stats: Mutex<HashMap<usize, Stats>>,
}

impl ExecutorStats {
    /// Creates a new `ExecutorStats` instance with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets the executor stats by clearing all collected statistics and resetting the start time and last ID.
    pub fn reset(&self) {
        *self.start_time.lock().unwrap_or_else(|e| e.into_inner()) = None;
        self.last_id.store(0, Ordering::Relaxed);
        while self.pending.pop().is_some() {}
        self.finalized.lock().unwrap_or_else(|e| e.into_inner()).clear();
        self.witness_stats.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    /// Adds a new statistic entry to the executor stats. Lock-free.
    pub fn add_stat(
        &self,
        parent_id: u64,
        id: u64,
        name: &'static str,
        index: usize,
        event: ExecutorStatsEvent,
    ) {
        self.pending.push(ExecutorStatsEntry {
            parent_id,
            id,
            name,
            index,
            event,
            timestamp: Instant::now(),
        });
    }

    /// Sets the start time for the executor stats, which is used as a reference point for calculating timestamps of events.
    pub fn set_start_time(&self, start_time: Instant) {
        *self.start_time.lock().unwrap_or_else(|e| e.into_inner()) = Some(start_time);
    }

    /// Generates the next unique ID for a new stats entry. Lock-free.
    pub fn next_id(&self) -> u64 {
        self.last_id.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Inserts witness statistics for a specific airgroup ID.
    pub fn insert_witness_stats(&self, airgroup_id: usize, stats: Stats) {
        self.witness_stats.lock().unwrap_or_else(|e| e.into_inner()).insert(airgroup_id, stats);
    }

    /// Sets the witness duration for a specific airgroup ID.
    pub fn set_witness_duration(&self, airgroup_id: usize, duration: u128) {
        if let Some(stats) =
            self.witness_stats.lock().unwrap_or_else(|e| e.into_inner()).get_mut(&airgroup_id)
        {
            stats.witness_duration = duration;
        }
    }

    /// Returns a snapshot of the witness statistics collected so far.
    pub fn witness_stats(&self) -> HashMap<usize, Stats> {
        self.witness_stats.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Drains any pending entries into `finalized` and returns the full set,
    /// ordered by timestamp. Non-destructive across repeated calls.
    fn collect_sorted(&self) -> Vec<ExecutorStatsEntry> {
        let mut finalized = self.finalized.lock().unwrap_or_else(|e| e.into_inner());
        while let Some(entry) = self.pending.pop() {
            finalized.push(entry);
        }
        let mut entries = finalized.clone();
        drop(finalized);
        entries.sort_by_key(|e| e.timestamp);
        entries
    }

    /// Stores stats in JSON and CSV file formats
    pub fn store_stats(&self) {
        #[derive(Serialize, Deserialize, Debug)]
        struct Task {
            parent_id: u64,
            id: u64,
            name: String,
            index: u64,
            event: String,
            timestamp: u64,
        }

        let start_time = (*self.start_time.lock().unwrap_or_else(|e| e.into_inner()))
            .unwrap_or_else(Instant::now);
        let tasks: Vec<Task> = self
            .collect_sorted()
            .into_iter()
            .map(|stat| Task {
                parent_id: stat.parent_id,
                id: stat.id,
                name: stat.name.to_string(),
                index: stat.index as u64,
                event: match stat.event {
                    ExecutorStatsEvent::Begin => "Begin".to_string(),
                    ExecutorStatsEvent::End => "End".to_string(),
                    ExecutorStatsEvent::Mark => "Mark".to_string(),
                },
                timestamp: stat.timestamp.saturating_duration_since(start_time).as_nanos() as u64,
            })
            .collect();

        tracing::info!("Collected a total of {} statistics", tasks.len());

        // Save to stats.json
        /////////////////////

        // Convert to pretty-printed JSON
        let json = match serde_json::to_string_pretty(&tasks) {
            Ok(json) => json,
            Err(e) => {
                tracing::error!("Failed to serialize stats to JSON: {e}");
                return;
            }
        };

        // Write to file
        let json_file_name = format!("stats_{}.json", process::id());
        let _ = fs::write(&json_file_name, json);

        // Save to stats.csv
        ////////////////////

        // Create a CSV-formatted string with the tasks data
        let mut csv = String::new();
        for task in tasks {
            csv += &format!(
                "{},{},{},{},{},{}\n",
                task.parent_id, task.id, task.name, task.index, task.event, task.timestamp
            );
        }

        // Write to file
        let csv_file_name = format!("stats_{}.csv", process::id());
        let _ = fs::write(&csv_file_name, csv);

        tracing::info!("Statistics have been saved to {} and {}", json_file_name, csv_file_name);
    }

    /// Prints stats
    pub fn print_stats(&self) {
        let start_time = (*self.start_time.lock().unwrap_or_else(|e| e.into_inner()))
            .unwrap_or_else(Instant::now);
        let entries = self.collect_sorted();
        println!("Collected a total of {} statistics", entries.len());
        for stat in &entries {
            println!(
                "parent_id={} id={} name={} index={} event={:?} timestamp={}",
                stat.parent_id,
                stat.id,
                stat.name,
                stat.index,
                stat.event,
                stat.timestamp.saturating_duration_since(start_time).as_nanos() as u64
            );
        }
    }
}

/// The `ExecutorStatsHandle` struct provides a cheap, cloneable handle to a shared
/// [`ExecutorStats`] collector. Since `ExecutorStats` is internally synchronized
/// (lock-free on the recording path), the handle is just a shared pointer and every
/// method forwards directly to the inner collector.
#[derive(Debug, Default, Clone)]
pub struct ExecutorStatsHandle {
    inner: Arc<ExecutorStats>,
}

impl ExecutorStatsHandle {
    /// Creates a new `ExecutorStatsHandle` instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets the executor stats by clearing all collected statistics and resetting the start time and last ID.
    pub fn reset(&self) {
        self.inner.reset();
    }

    /// Adds a new statistic entry to the executor stats.
    pub fn add_stat(
        &self,
        parent_id: u64,
        id: u64,
        name: &'static str,
        index: usize,
        event: ExecutorStatsEvent,
    ) {
        self.inner.add_stat(parent_id, id, name, index, event);
    }

    /// Sets the start time for the executor stats, which is used as a reference point for calculating timestamps of events.
    pub fn set_start_time(&self, start_time: Instant) {
        self.inner.set_start_time(start_time);
    }

    /// Generates the next unique ID for a new stats entry.
    pub fn next_id(&self) -> u64 {
        self.inner.next_id()
    }

    /// Stores stats in JSON and CSV file formats
    pub fn store_stats(&self) {
        self.inner.store_stats();
    }

    /// Prints stats
    pub fn print_stats(&self) {
        self.inner.print_stats();
    }

    /// Returns the shared `ExecutorStats` instance.
    pub fn get_inner(&self) -> Arc<ExecutorStats> {
        self.inner.clone()
    }

    /// Returns a snapshot of the witness statistics collected so far.
    pub fn witness_stats(&self) -> HashMap<usize, Stats> {
        self.inner.witness_stats()
    }

    /// Inserts witness statistics for a specific airgroup ID.
    pub fn insert_witness_stats(&self, airgroup_id: usize, stats: Stats) {
        self.inner.insert_witness_stats(airgroup_id, stats);
    }

    /// Sets the witness duration for a specific airgroup ID.
    pub fn set_witness_duration(&self, airgroup_id: usize, duration: u128) {
        self.inner.set_witness_duration(airgroup_id, duration);
    }
}
