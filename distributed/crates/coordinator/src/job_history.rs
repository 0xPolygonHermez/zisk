use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{postgres::PgPoolOptions, types::Uuid, PgPool, Postgres, QueryBuilder, Row};
use tokio::sync::mpsc;
use tracing::{info, warn};
use zisk_cluster_common::Job;

const EVENT_TYPE_JOB_SNAPSHOT: &str = "job.snapshot";
const EVENT_TYPE_JOB_EVENT_LOG: &str = "job.event_log";
pub const EVENT_TYPE_JOB_QUEUED: &str = "job.queued";
pub const EVENT_TYPE_JOB_STARTED: &str = "job.started";
pub const EVENT_TYPE_JOB_WAITING_FOR_INPUT: &str = "job.waiting_for_input";
pub const EVENT_TYPE_JOB_PHASE_CHANGED: &str = "job.phase_changed";
pub const EVENT_TYPE_JOB_SUCCEEDED: &str = "job.succeeded";
pub const EVENT_TYPE_JOB_FAILED: &str = "job.failed";
pub const EVENT_TYPE_JOB_CANCELLED: &str = "job.cancelled";
const EVENT_TYPE_PHASE_STARTED: &str = "phase.started";
const EVENT_TYPE_PHASE_COMPLETED: &str = "phase.completed";
const EVENT_TYPE_WORKER_ASSIGNED: &str = "worker.assigned";
pub const EVENT_TYPE_PROGRAM_REGISTERED: &str = "program.registered";
pub const EVENT_TYPE_COORDINATOR_STARTED: &str = "coordinator.started";
pub const EVENT_TYPE_WORKER_REGISTERED: &str = "worker.registered";
pub const EVENT_TYPE_WORKER_RECONNECTED: &str = "worker.reconnected";
pub const EVENT_TYPE_WORKER_DISCONNECTED: &str = "worker.disconnected";
pub const EVENT_TYPE_WORKER_UNREGISTERED: &str = "worker.unregistered";
pub const EVENT_TYPE_WORKER_ERROR: &str = "worker.error";
const PHASE_EVENT_STARTED: &str = "started";
const PHASE_EVENT_ENDED: &str = "ended";
const WORKER_ROLE_PARTICIPANT: &str = "participant";
const WORKER_ROLE_AGGREGATOR: &str = "aggregator";
const DEFAULT_MAX_CONNECTIONS: u32 = 5;
/// Upper bound for worker-error page size.
const WORKER_ERROR_QUERY_LIMIT_MAX: usize = 1000;
/// Default page size when the caller supplies `limit = 0`.
const WORKER_ERROR_QUERY_LIMIT_DEFAULT: usize = 100;
/// Maximum stored worker-error message length.
pub const WORKER_ERROR_MESSAGE_MAX_CHARS: usize = 500;

/// Bounded worker-error reason taxonomy.
pub mod worker_error_reason {
    pub const HEARTBEAT_LOST: &str = "heartbeat_lost";
    pub const CHANNEL_CLOSED: &str = "channel_closed";
    pub const SETUP_FAIL: &str = "setup_fail";
    pub const PROVE_FAIL: &str = "prove_fail";
    pub const AGG_FAIL: &str = "agg_fail";
    pub const UNREACHABLE: &str = "unreachable";
    pub const UNKNOWN: &str = "unknown";
}

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub trait JobHistoryStore: Send + Sync {
    fn try_record_snapshot(&self, snapshot: JobHistorySnapshot);

    fn try_record_event(&self, event: JobHistoryEvent) {
        let _ = event;
    }

    fn try_record_event_envelope(&self, envelope: JobHistoryEventEnvelope) {
        if let Some(snapshot) = envelope.projection {
            self.try_record_snapshot(snapshot);
        }
    }

    fn try_record_lifecycle_event(&self, event: JobHistoryLifecycleEvent) {
        self.try_record_event_envelope(event.into_envelope());
    }

    fn list_recent_jobs<'a>(
        &'a self,
        query: JobHistoryListQuery,
    ) -> futures::future::BoxFuture<'a, Result<JobHistoryPage>>;

    fn get_job<'a>(
        &'a self,
        job_id: Uuid,
    ) -> futures::future::BoxFuture<'a, Result<Option<JobHistoryJob>>>;

    fn last_successful_proof_timestamp<'a>(
        &'a self,
        coordinator_id: &'a str,
    ) -> futures::future::BoxFuture<'a, Result<Option<DateTime<Utc>>>>;

    /// Marks stale running rows from earlier coordinator processes as failed.
    fn reconcile_interrupted_jobs<'a>(
        &'a self,
        _coordinator_id: &'a str,
        _process_started_at: DateTime<Utc>,
        _reason: &'a str,
    ) -> futures::future::BoxFuture<'a, Result<u64>> {
        Box::pin(async { Ok(0) })
    }

    /// Records a worker-attributable error event.
    fn record_worker_error<'a>(
        &'a self,
        event: JobHistoryWorkerError,
    ) -> futures::future::BoxFuture<'a, Result<()>> {
        let _ = event;
        Box::pin(async { Ok(()) })
    }

    /// Returns recent worker error events matching `query`.
    fn recent_worker_errors<'a>(
        &'a self,
        query: WorkerErrorQuery,
    ) -> futures::future::BoxFuture<'a, Result<Vec<JobHistoryWorkerError>>> {
        let _ = query;
        Box::pin(async { Ok(Vec::new()) })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobHistorySnapshot {
    pub coordinator_id: String,
    pub job_id: Uuid,
    pub hash_id: String,
    pub program: String,
    pub state: String,
    pub failure_reason: Option<String>,
    pub proof_type: String,
    pub received_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub workers: Vec<String>,
    pub agg_worker_id: Option<String>,
    pub phase_timings: Vec<JobHistoryPhaseTiming>,
    pub instances: Option<u64>,
    pub executed_steps: Option<u64>,
}

/// Append-only lifecycle event-log row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobHistoryEvent {
    pub coordinator_id: String,
    pub job_id: Option<Uuid>,
    pub worker_id: Option<String>,
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
    pub schema_version: i32,
    pub dedupe_key: String,
    pub payload: Value,
}

impl JobHistoryEvent {
    pub fn new(
        coordinator_id: impl Into<String>,
        job_id: Option<Uuid>,
        worker_id: Option<String>,
        event_type: impl Into<String>,
        occurred_at: DateTime<Utc>,
        dedupe_key: impl Into<String>,
        payload: Value,
    ) -> Self {
        Self {
            coordinator_id: coordinator_id.into(),
            job_id,
            worker_id,
            event_type: event_type.into(),
            occurred_at,
            schema_version: 1,
            dedupe_key: dedupe_key.into(),
            payload,
        }
    }

    pub fn coordinator_event(
        coordinator_id: impl Into<String>,
        event_type: &'static str,
        occurred_at: DateTime<Utc>,
        payload: Value,
    ) -> Self {
        let coordinator_id = coordinator_id.into();
        Self::new(
            coordinator_id.clone(),
            None,
            None,
            event_type,
            occurred_at,
            format!("{event_type}:{coordinator_id}:{}", occurred_at.timestamp_micros()),
            payload,
        )
    }

    pub fn worker_event(
        coordinator_id: impl Into<String>,
        worker_id: impl Into<String>,
        event_type: &'static str,
        occurred_at: DateTime<Utc>,
        payload: Value,
    ) -> Self {
        let coordinator_id = coordinator_id.into();
        let worker_id = worker_id.into();
        Self::new(
            coordinator_id.clone(),
            None,
            Some(worker_id.clone()),
            event_type,
            occurred_at,
            format!("{event_type}:{coordinator_id}:{worker_id}:{}", occurred_at.timestamp_micros()),
            payload,
        )
    }
}

#[derive(Debug, Clone)]
pub struct JobHistoryEventEnvelope {
    pub event: JobHistoryEvent,
    pub projection: Option<JobHistorySnapshot>,
}

impl JobHistoryEventEnvelope {
    pub fn event(event: JobHistoryEvent) -> Self {
        Self { event, projection: None }
    }

    pub fn snapshot(snapshot: JobHistorySnapshot) -> Self {
        let payload = snapshot_payload(&snapshot);
        let occurred_at = snapshot.completed_at.or(snapshot.received_at).unwrap_or_else(Utc::now);
        let event = JobHistoryEvent::new(
            snapshot.coordinator_id.clone(),
            Some(snapshot.job_id),
            None,
            EVENT_TYPE_JOB_SNAPSHOT,
            occurred_at,
            format!("{}:{}:{}", EVENT_TYPE_JOB_SNAPSHOT, snapshot.job_id, payload_digest(&payload)),
            payload,
        );
        Self { event, projection: Some(snapshot) }
    }
}

#[derive(Debug, Clone)]
pub struct JobHistoryLifecycleEvent {
    pub event_type: &'static str,
    pub occurred_at: DateTime<Utc>,
    pub payload: Value,
    pub snapshot: JobHistorySnapshot,
}

impl JobHistoryLifecycleEvent {
    pub fn new(
        event_type: &'static str,
        occurred_at: DateTime<Utc>,
        payload: Value,
        snapshot: JobHistorySnapshot,
    ) -> Self {
        Self { event_type, occurred_at, payload, snapshot }
    }

    pub fn into_envelope(self) -> JobHistoryEventEnvelope {
        let event = JobHistoryEvent::new(
            self.snapshot.coordinator_id.clone(),
            Some(self.snapshot.job_id),
            None,
            self.event_type,
            self.occurred_at,
            format!(
                "{}:{}:{}",
                self.event_type,
                self.snapshot.job_id,
                payload_digest(&self.payload)
            ),
            self.payload,
        );
        JobHistoryEventEnvelope { event, projection: Some(self.snapshot) }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobHistoryPhaseTiming {
    pub phase: String,
    pub start_at: DateTime<Utc>,
    pub end_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct JobHistoryListQuery {
    pub coordinator_id: Option<String>,
    pub job_id: Option<Uuid>,
    pub state: Option<String>,
    pub hash_id: Option<String>,
    pub cursor: Option<DateTime<Utc>>,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobHistoryPage {
    pub data: Vec<JobHistoryJob>,
    pub pagination: JobHistoryPagination,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobHistoryPagination {
    pub limit: usize,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobHistoryJob {
    pub coordinator_id: String,
    pub job_id: Uuid,
    pub job_label: String,
    pub hash_id: String,
    pub program: String,
    pub state: String,
    pub failure_reason: Option<String>,
    pub proof_type: String,
    pub received_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub workers: Vec<String>,
    pub workers_count: usize,
    pub agg_worker_id: Option<String>,
    pub phase_timings: Vec<JobHistoryPhaseTiming>,
    pub contributions_duration_ms: Option<u64>,
    pub prove_duration_ms: Option<u64>,
    pub aggregate_duration_ms: Option<u64>,
    pub execution_duration_ms: Option<u64>,
    pub age_seconds: Option<u64>,
    pub current_phase: Option<String>,
    pub current_phase_started_at: Option<DateTime<Utc>>,
    pub current_phase_age_seconds: Option<u64>,
    pub last_update_age_seconds: u64,
    pub instances: Option<u64>,
    pub executed_steps: Option<u64>,
    pub updated_at: DateTime<Utc>,
    pub sort_at: DateTime<Utc>,
}

/// Worker-attributable error event row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobHistoryWorkerError {
    pub coordinator_id: String,
    pub worker_id: String,
    pub job_id: Uuid,
    pub hash_id: String,
    pub program: String,
    pub reason: String,
    pub message: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

impl JobHistoryWorkerError {
    /// Bounds the stored error message length.
    pub fn truncate_message(&mut self) {
        if let Some(message) = self.message.as_mut() {
            truncate_chars_in_place(message, WORKER_ERROR_MESSAGE_MAX_CHARS);
        }
    }
}

/// Filter / pagination parameters for [`JobHistoryStore::recent_worker_errors`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkerErrorQuery {
    /// Requested page size; `0` uses the default.
    #[serde(default)]
    pub limit: usize,
    #[serde(default)]
    pub worker_id: Option<String>,
    #[serde(default)]
    pub job_id: Option<Uuid>,
    #[serde(default)]
    pub program: Option<String>,
    #[serde(default)]
    pub programs: Vec<String>,
    #[serde(default)]
    pub since: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramPerformancePage {
    pub data: Vec<ProgramPerformanceSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramPerformanceSummary {
    pub program: String,
    pub jobs: usize,
    pub success_rate: Option<f64>,
    pub avg_duration_ms: Option<u64>,
    pub p95_duration_ms: Option<u64>,
    pub p99_duration_ms: Option<u64>,
    pub avg_steps_per_second: Option<f64>,
}

impl ProgramPerformancePage {
    pub fn from_jobs(mut jobs: Vec<JobHistoryJob>) -> Self {
        jobs.retain(|job| matches!(job.state.as_str(), "Completed" | "Failed" | "Cancelled"));

        let mut by_program: HashMap<String, Vec<JobHistoryJob>> = HashMap::new();
        for job in jobs {
            by_program.entry(job.program.clone()).or_default().push(job);
        }

        let mut data = by_program
            .into_iter()
            .map(|(program, jobs)| ProgramPerformanceSummary::from_jobs(program, jobs))
            .collect::<Vec<_>>();
        data.sort_by(|left, right| {
            right.jobs.cmp(&left.jobs).then_with(|| left.program.cmp(&right.program))
        });
        Self { data }
    }
}

impl ProgramPerformanceSummary {
    fn from_jobs(program: String, jobs: Vec<JobHistoryJob>) -> Self {
        let success = jobs.iter().filter(|job| job.state == "Completed").count();
        let durations = jobs.iter().filter_map(|job| job.duration_ms).collect::<Vec<_>>();
        let avg_steps_per_second = average_steps_per_second(&jobs);
        let total = jobs.len();

        Self {
            program,
            jobs: total,
            success_rate: (total > 0).then_some(success as f64 / total as f64),
            avg_duration_ms: average_u64(&durations),
            p95_duration_ms: quantile_u64(&mut durations.clone(), 0.95),
            p99_duration_ms: quantile_u64(&mut durations.clone(), 0.99),
            avg_steps_per_second,
        }
    }
}

struct JobRow {
    coordinator_id: String,
    job_id: Uuid,
    hash_id: String,
    program: String,
    state: String,
    failure_reason: Option<String>,
    proof_type: String,
    received_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    duration_ms: Option<i64>,
    instances: Option<i64>,
    executed_steps: Option<i64>,
    agg_worker_id: Option<String>,
    updated_at: DateTime<Utc>,
    sort_at: DateTime<Utc>,
}

impl JobHistorySnapshot {
    pub fn from_job(coordinator_id: &str, job: &Job) -> Result<Self> {
        let job_id = Uuid::parse_str(job.job_id.as_str())
            .with_context(|| format!("parse job_id {} as UUID", job.job_id.as_str()))?;
        let mut phase_timings = job
            .phase_timings
            .iter()
            .map(|(phase, timing)| JobHistoryPhaseTiming {
                phase: phase.to_string(),
                start_at: timing.start_time,
                end_at: timing.end_time,
                duration_ms: timing.end_time.map(|end| {
                    end.signed_duration_since(timing.start_time).num_milliseconds().max(0) as u64
                }),
            })
            .collect::<Vec<_>>();
        phase_timings.sort_by_key(|timing| timing.start_at);

        Ok(Self {
            coordinator_id: coordinator_id.to_owned(),
            job_id,
            hash_id: job.hash_id.clone(),
            program: crate::program_registry::default_alias_for_hash(&job.hash_id),
            state: job.state.to_string(),
            failure_reason: None,
            proof_type: format!("{:?}", job.proof_type),
            received_at: job.task_received_time,
            completed_at: job.terminated_at,
            duration_ms: job.duration_ms,
            workers: job.workers.iter().map(|worker| worker.as_string()).collect(),
            agg_worker_id: job.agg_worker_id.as_ref().map(|worker| worker.as_string()),
            phase_timings,
            instances: job.instances,
            executed_steps: job.executed_steps,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PostgresJobHistoryOptions {
    pub auto_migrate: bool,
    pub channel_capacity: usize,
    pub batch_size: usize,
    pub flush_interval: Duration,
}

impl Default for PostgresJobHistoryOptions {
    fn default() -> Self {
        Self {
            auto_migrate: true,
            channel_capacity: 10_000,
            batch_size: 100,
            flush_interval: Duration::from_millis(250),
        }
    }
}

pub struct PostgresJobHistoryStore {
    pool: PgPool,
    tx: mpsc::Sender<JobHistoryWrite>,
}

#[derive(Debug, Clone)]
enum JobHistoryWrite {
    Event(Box<JobHistoryEventEnvelope>),
    Snapshot(Box<JobHistorySnapshot>),
}

pub async fn start_postgres_job_history(
    database_url: &str,
    options: PostgresJobHistoryOptions,
) -> Result<Arc<dyn JobHistoryStore>> {
    let pool = PgPoolOptions::new()
        .max_connections(DEFAULT_MAX_CONNECTIONS)
        .connect(database_url)
        .await
        .context("connect to Postgres job history database")?;

    if options.auto_migrate {
        MIGRATOR.run(&pool).await.context("run job history migrations")?;
    }

    record_pool_metrics(&pool);
    metrics::gauge!("coordinator_db_write_queue_depth").set(0.0);

    let channel_capacity = options.channel_capacity.max(1);
    let (tx, rx) = mpsc::channel(channel_capacity);
    let store = Arc::new(PostgresJobHistoryStore { pool: pool.clone(), tx });

    tokio::spawn(writer_loop(pool, rx, options));

    info!(
        channel_capacity,
        "Postgres job history enabled; terminal snapshots are buffered off the hot path"
    );

    Ok(store)
}

impl JobHistoryStore for PostgresJobHistoryStore {
    fn try_record_snapshot(&self, snapshot: JobHistorySnapshot) {
        match self.tx.try_send(JobHistoryWrite::Snapshot(Box::new(snapshot))) {
            Ok(()) => record_queue_depth(&self.tx),
            Err(mpsc::error::TrySendError::Full(_)) => {
                record_dropped_event(EVENT_TYPE_JOB_SNAPSHOT);
                metrics::gauge!("coordinator_db_write_queue_depth")
                    .set(self.tx.max_capacity() as f64);
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                record_dropped_event(EVENT_TYPE_JOB_SNAPSHOT);
                metrics::gauge!("coordinator_db_write_queue_depth").set(0.0);
                warn!("Postgres job history writer is closed; dropping history snapshot");
            }
        }
    }

    fn try_record_event(&self, event: JobHistoryEvent) {
        self.try_record_event_envelope(JobHistoryEventEnvelope::event(event));
    }

    fn try_record_event_envelope(&self, envelope: JobHistoryEventEnvelope) {
        match self.tx.try_send(JobHistoryWrite::Event(Box::new(envelope))) {
            Ok(()) => record_queue_depth(&self.tx),
            Err(mpsc::error::TrySendError::Full(_)) => {
                record_dropped_event(EVENT_TYPE_JOB_EVENT_LOG);
                metrics::gauge!("coordinator_db_write_queue_depth")
                    .set(self.tx.max_capacity() as f64);
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                record_dropped_event(EVENT_TYPE_JOB_EVENT_LOG);
                metrics::gauge!("coordinator_db_write_queue_depth").set(0.0);
                warn!("Postgres job history writer is closed; dropping history event");
            }
        }
    }

    fn try_record_lifecycle_event(&self, event: JobHistoryLifecycleEvent) {
        self.try_record_event_envelope(event.into_envelope());
    }

    fn list_recent_jobs<'a>(
        &'a self,
        query: JobHistoryListQuery,
    ) -> futures::future::BoxFuture<'a, Result<JobHistoryPage>> {
        Box::pin(async move {
            let started = Instant::now();
            let result = list_recent_jobs(&self.pool, query).await;
            record_query_metric(
                "list_recent_jobs",
                if result.is_ok() { "success" } else { "error" },
                started,
            );
            record_pool_metrics(&self.pool);
            result
        })
    }

    fn get_job<'a>(
        &'a self,
        job_id: Uuid,
    ) -> futures::future::BoxFuture<'a, Result<Option<JobHistoryJob>>> {
        Box::pin(async move {
            let started = Instant::now();
            let result = get_job(&self.pool, job_id).await;
            record_query_metric(
                "get_job",
                if result.is_ok() { "success" } else { "error" },
                started,
            );
            record_pool_metrics(&self.pool);
            result
        })
    }

    fn last_successful_proof_timestamp<'a>(
        &'a self,
        coordinator_id: &'a str,
    ) -> futures::future::BoxFuture<'a, Result<Option<DateTime<Utc>>>> {
        Box::pin(async move {
            let started = Instant::now();
            let result = sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
                r#"
                SELECT MAX(completed_at)
                FROM job_history_jobs
                WHERE coordinator_id = $1 AND state = 'Completed'
                "#,
            )
            .bind(coordinator_id)
            .fetch_one(&self.pool)
            .await
            .context("read last successful proof timestamp");
            record_query_metric(
                "last_successful_proof",
                if result.is_ok() { "success" } else { "error" },
                started,
            );
            record_pool_metrics(&self.pool);
            result
        })
    }

    fn reconcile_interrupted_jobs<'a>(
        &'a self,
        coordinator_id: &'a str,
        process_started_at: DateTime<Utc>,
        reason: &'a str,
    ) -> futures::future::BoxFuture<'a, Result<u64>> {
        Box::pin(async move {
            let started = Instant::now();
            let result =
                reconcile_interrupted_jobs(&self.pool, coordinator_id, process_started_at, reason)
                    .await;
            record_query_metric(
                "reconcile_interrupted_jobs",
                if result.is_ok() { "success" } else { "error" },
                started,
            );
            record_pool_metrics(&self.pool);
            result
        })
    }

    fn record_worker_error<'a>(
        &'a self,
        event: JobHistoryWorkerError,
    ) -> futures::future::BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let started = Instant::now();
            let result = write_worker_error_event(&self.pool, event).await;
            record_query_metric(
                "record_worker_error",
                if result.is_ok() { "success" } else { "error" },
                started,
            );
            record_pool_metrics(&self.pool);
            result
        })
    }

    fn recent_worker_errors<'a>(
        &'a self,
        query: WorkerErrorQuery,
    ) -> futures::future::BoxFuture<'a, Result<Vec<JobHistoryWorkerError>>> {
        Box::pin(async move {
            let started = Instant::now();
            let result = list_worker_errors(&self.pool, query).await;
            record_query_metric(
                "recent_worker_errors",
                if result.is_ok() { "success" } else { "error" },
                started,
            );
            record_pool_metrics(&self.pool);
            result
        })
    }
}

async fn reconcile_interrupted_jobs(
    pool: &PgPool,
    coordinator_id: &str,
    process_started_at: DateTime<Utc>,
    reason: &str,
) -> Result<u64> {
    let result = sqlx::query(
        r#"
        UPDATE job_history_jobs
        SET
            state = 'Failed',
            failure_reason = COALESCE(NULLIF(failure_reason, ''), $3),
            completed_at = COALESCE(completed_at, $2),
            duration_ms = COALESCE(
                duration_ms,
                CASE
                    WHEN received_at IS NULL THEN NULL
                    ELSE GREATEST(0, FLOOR(EXTRACT(EPOCH FROM ($2 - received_at)) * 1000))::BIGINT
                END
            ),
            updated_at = $2
        WHERE coordinator_id = $1
            AND state LIKE 'Running%'
            AND updated_at < $2
        "#,
    )
    .bind(coordinator_id)
    .bind(process_started_at)
    .bind(reason)
    .execute(pool)
    .await
    .context("reconcile interrupted running job history rows")?;

    Ok(result.rows_affected())
}

async fn write_worker_error_event(pool: &PgPool, mut event: JobHistoryWorkerError) -> Result<()> {
    // Keep row size bounded at the storage boundary.
    event.truncate_message();
    let history_event = worker_error_history_event(&event);
    let mut tx = pool.begin().await.context("begin worker error event transaction")?;
    insert_job_event(&mut tx, &history_event).await?;
    insert_worker_error_projection(&mut tx, &event).await?;
    update_projection_cursor(&mut tx).await?;
    tx.commit().await.context("commit worker error event transaction")?;
    Ok(())
}

async fn insert_worker_error_projection(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    event: &JobHistoryWorkerError,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO job_history_worker_errors (
            coordinator_id, worker_id, job_id, hash_id, program, reason, message, occurred_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(&event.coordinator_id)
    .bind(&event.worker_id)
    .bind(event.job_id)
    .bind(&event.hash_id)
    .bind(&event.program)
    .bind(&event.reason)
    .bind(event.message.as_deref())
    .bind(event.occurred_at)
    .execute(&mut **tx)
    .await
    .context("insert worker error event")?;
    Ok(())
}

async fn list_worker_errors(
    pool: &PgPool,
    query: WorkerErrorQuery,
) -> Result<Vec<JobHistoryWorkerError>> {
    let limit = normalize_worker_error_limit(query.limit);
    let mut qb = QueryBuilder::<Postgres>::new(
        r#"
        SELECT coordinator_id, worker_id, job_id, hash_id, program, reason, message, occurred_at
        FROM job_history_worker_errors
        WHERE 1 = 1
        "#,
    );

    if let Some(worker_id) = query.worker_id.as_deref().filter(|value| !value.is_empty()) {
        qb.push(" AND worker_id = ").push_bind(worker_id);
    }
    if let Some(job_id) = query.job_id {
        qb.push(" AND job_id = ").push_bind(job_id);
    }
    let mut programs: Vec<&str> = query
        .program
        .as_deref()
        .filter(|value| !value.is_empty())
        .into_iter()
        .chain(query.programs.iter().map(String::as_str).filter(|value| !value.is_empty()))
        .collect();
    programs.sort_unstable();
    programs.dedup();
    if !programs.is_empty() {
        qb.push(" AND program IN (");
        let mut separated = qb.separated(", ");
        for program in programs {
            separated.push_bind(program);
        }
        separated.push_unseparated(")");
    }
    if let Some(since) = query.since {
        qb.push(" AND occurred_at >= ").push_bind(since);
    }

    qb.push(" ORDER BY occurred_at DESC, id DESC LIMIT ").push_bind(limit as i64);

    let rows = qb.build().fetch_all(pool).await.context("list worker error rows")?;

    rows.into_iter()
        .map(|row| {
            Ok(JobHistoryWorkerError {
                coordinator_id: row.try_get("coordinator_id")?,
                worker_id: row.try_get("worker_id")?,
                job_id: row.try_get("job_id")?,
                hash_id: row.try_get("hash_id")?,
                program: row.try_get("program")?,
                reason: row.try_get("reason")?,
                message: row.try_get("message")?,
                occurred_at: row.try_get("occurred_at")?,
            })
        })
        .collect()
}

fn normalize_worker_error_limit(limit: usize) -> usize {
    match limit {
        0 => WORKER_ERROR_QUERY_LIMIT_DEFAULT,
        n if n > WORKER_ERROR_QUERY_LIMIT_MAX => WORKER_ERROR_QUERY_LIMIT_MAX,
        n => n,
    }
}

fn truncate_chars_in_place(value: &mut String, max_chars: usize) {
    if value.chars().count() <= max_chars {
        return;
    }
    let byte_idx =
        value.char_indices().nth(max_chars).map(|(idx, _)| idx).unwrap_or_else(|| value.len());
    value.truncate(byte_idx);
}

async fn list_recent_jobs(pool: &PgPool, query: JobHistoryListQuery) -> Result<JobHistoryPage> {
    let limit = normalize_limit(query.limit);
    let mut rows = fetch_job_rows(pool, &query, Some(limit + 1)).await?;
    let has_more = rows.len() > limit;
    if has_more {
        rows.truncate(limit);
    }
    let next_cursor = if has_more { rows.last().map(|row| row.sort_at.to_rfc3339()) } else { None };
    let data = hydrate_job_rows(pool, rows).await?;
    Ok(JobHistoryPage { data, pagination: JobHistoryPagination { limit, next_cursor, has_more } })
}

async fn get_job(pool: &PgPool, job_id: Uuid) -> Result<Option<JobHistoryJob>> {
    let row = fetch_job_row_by_id(pool, job_id).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(hydrate_job_rows(pool, vec![row]).await?.into_iter().next())
}

async fn fetch_job_row_by_id(pool: &PgPool, job_id: Uuid) -> Result<Option<JobRow>> {
    let row = sqlx::query(
        r#"
        SELECT
            coordinator_id,
            job_id,
            hash_id,
            program,
            state,
            failure_reason,
            proof_type,
            received_at,
            completed_at,
            duration_ms,
            instances,
            executed_steps,
            agg_worker_id,
            updated_at,
            COALESCE(completed_at, received_at, updated_at) AS sort_at
        FROM job_history_jobs
        WHERE job_id = $1
        "#,
    )
    .bind(job_id)
    .fetch_optional(pool)
    .await
    .context("get job history row")?;
    row.map(row_to_job_row).transpose()
}

async fn fetch_job_rows(
    pool: &PgPool,
    query: &JobHistoryListQuery,
    limit: Option<usize>,
) -> Result<Vec<JobRow>> {
    let mut qb = QueryBuilder::<Postgres>::new(
        r#"
        SELECT
            coordinator_id,
            job_id,
            hash_id,
            program,
            state,
            failure_reason,
            proof_type,
            received_at,
            completed_at,
            duration_ms,
            instances,
            executed_steps,
            agg_worker_id,
            updated_at,
            COALESCE(completed_at, received_at, updated_at) AS sort_at
        FROM job_history_jobs
        WHERE 1 = 1
        "#,
    );

    if let Some(coordinator_id) = query.coordinator_id.as_deref().filter(|value| !value.is_empty())
    {
        qb.push(" AND coordinator_id = ").push_bind(coordinator_id);
    }
    if let Some(job_id) = query.job_id {
        qb.push(" AND job_id = ").push_bind(job_id);
    }
    if let Some(state) = query.state.as_deref().filter(|value| !value.is_empty()) {
        qb.push(" AND state = ").push_bind(state);
    }
    if let Some(hash_id) = query.hash_id.as_deref().filter(|value| !value.is_empty()) {
        qb.push(" AND hash_id = ").push_bind(hash_id);
    }
    if let Some(cursor) = query.cursor {
        qb.push(" AND COALESCE(completed_at, received_at, updated_at) < ").push_bind(cursor);
    }
    qb.push(" ORDER BY sort_at DESC, job_id DESC");
    if let Some(limit) = limit {
        qb.push(" LIMIT ").push_bind(limit as i64);
    }

    let rows = qb
        .build()
        .fetch_all(pool)
        .await
        .context("list job history rows")?
        .into_iter()
        .map(row_to_job_row)
        .collect::<Result<Vec<_>>>()?;
    Ok(rows)
}

fn row_to_job_row(row: sqlx::postgres::PgRow) -> Result<JobRow> {
    Ok(JobRow {
        coordinator_id: row.try_get("coordinator_id")?,
        job_id: row.try_get("job_id")?,
        hash_id: row.try_get("hash_id")?,
        program: row.try_get("program")?,
        state: row.try_get("state")?,
        failure_reason: row.try_get("failure_reason")?,
        proof_type: row.try_get("proof_type")?,
        received_at: row.try_get("received_at")?,
        completed_at: row.try_get("completed_at")?,
        duration_ms: row.try_get("duration_ms")?,
        instances: row.try_get("instances")?,
        executed_steps: row.try_get("executed_steps")?,
        agg_worker_id: row.try_get("agg_worker_id")?,
        updated_at: row.try_get("updated_at")?,
        sort_at: row.try_get("sort_at")?,
    })
}

async fn hydrate_job_rows(pool: &PgPool, rows: Vec<JobRow>) -> Result<Vec<JobHistoryJob>> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let job_ids = rows.iter().map(|row| row.job_id).collect::<Vec<_>>();
    let workers = fetch_workers(pool, &job_ids).await?;
    let phase_timings = fetch_phase_timings(pool, &job_ids).await?;
    let now = Utc::now();

    Ok(rows
        .into_iter()
        .map(|row| {
            let job_workers = workers.get(&row.job_id).cloned().unwrap_or_default();
            let job_phase_timings = phase_timings.get(&row.job_id).cloned().unwrap_or_default();
            let active_phase = active_phase(&row.state, &job_phase_timings);
            let current_phase_started_at = active_phase.map(|timing| timing.start_at);
            JobHistoryJob {
                coordinator_id: row.coordinator_id,
                job_id: row.job_id,
                job_label: short_job_label(row.job_id),
                program: row.program,
                hash_id: row.hash_id,
                state: row.state,
                failure_reason: row.failure_reason,
                proof_type: row.proof_type,
                received_at: row.received_at,
                completed_at: row.completed_at,
                duration_ms: i64_to_u64(row.duration_ms),
                workers_count: job_workers.len(),
                workers: job_workers,
                agg_worker_id: row.agg_worker_id,
                contributions_duration_ms: phase_duration_ms(&job_phase_timings, "Contributions"),
                prove_duration_ms: phase_duration_ms(&job_phase_timings, "Prove"),
                aggregate_duration_ms: phase_duration_ms(&job_phase_timings, "Aggregate"),
                execution_duration_ms: phase_duration_ms(&job_phase_timings, "Execution"),
                age_seconds: job_age_seconds(row.received_at, row.completed_at, now),
                current_phase: active_phase.map(|timing| timing.phase.clone()),
                current_phase_started_at,
                current_phase_age_seconds: current_phase_started_at
                    .map(|started_at| elapsed_seconds(started_at, now)),
                last_update_age_seconds: elapsed_seconds(row.updated_at, now),
                phase_timings: job_phase_timings,
                instances: i64_to_u64(row.instances),
                executed_steps: i64_to_u64(row.executed_steps),
                updated_at: row.updated_at,
                sort_at: row.sort_at,
            }
        })
        .collect())
}

fn short_job_label(job_id: Uuid) -> String {
    job_id.as_simple().to_string()[..8].to_owned()
}

fn active_phase<'a>(
    state: &str,
    timings: &'a [JobHistoryPhaseTiming],
) -> Option<&'a JobHistoryPhaseTiming> {
    if !state.starts_with("Running") {
        return None;
    }
    timings.iter().rev().find(|timing| timing.end_at.is_none())
}

fn job_age_seconds(
    received_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Option<u64> {
    let received_at = received_at?;
    Some(elapsed_seconds(received_at, completed_at.unwrap_or(now)))
}

fn elapsed_seconds(started_at: DateTime<Utc>, ended_at: DateTime<Utc>) -> u64 {
    ended_at.signed_duration_since(started_at).num_seconds().max(0) as u64
}

fn phase_duration_ms(timings: &[JobHistoryPhaseTiming], phase: &str) -> Option<u64> {
    timings.iter().find(|timing| timing.phase == phase).and_then(|timing| timing.duration_ms)
}

async fn fetch_workers(pool: &PgPool, job_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<String>>> {
    let rows = sqlx::query(
        r#"
        SELECT job_id, worker_id
        FROM job_history_job_workers
        WHERE job_id = ANY($1) AND role = 'participant'
        ORDER BY job_id, worker_id
        "#,
    )
    .bind(job_ids)
    .fetch_all(pool)
    .await
    .context("list job worker rows")?;

    let mut workers: HashMap<Uuid, Vec<String>> = HashMap::new();
    for row in rows {
        workers.entry(row.try_get("job_id")?).or_default().push(row.try_get("worker_id")?);
    }
    Ok(workers)
}

async fn fetch_phase_timings(
    pool: &PgPool,
    job_ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<JobHistoryPhaseTiming>>> {
    let rows = sqlx::query(
        r#"
        SELECT job_id, phase, event_type, occurred_at, duration_ms
        FROM job_history_phase_events
        WHERE job_id = ANY($1)
        ORDER BY job_id, occurred_at, event_type
        "#,
    )
    .bind(job_ids)
    .fetch_all(pool)
    .await
    .context("list job phase events")?;

    let mut phases: BTreeMap<(Uuid, String), JobHistoryPhaseTiming> = BTreeMap::new();
    for row in rows {
        let job_id: Uuid = row.try_get("job_id")?;
        let phase: String = row.try_get("phase")?;
        let event_type: String = row.try_get("event_type")?;
        let occurred_at: DateTime<Utc> = row.try_get("occurred_at")?;
        let duration_ms: Option<i64> = row.try_get("duration_ms")?;
        let timing = phases.entry((job_id, phase.clone())).or_insert(JobHistoryPhaseTiming {
            phase,
            start_at: occurred_at,
            end_at: None,
            duration_ms: None,
        });
        match event_type.as_str() {
            PHASE_EVENT_STARTED => {
                timing.start_at = timing.start_at.min(occurred_at);
            }
            PHASE_EVENT_ENDED => {
                timing.end_at =
                    Some(timing.end_at.map_or(occurred_at, |current| current.max(occurred_at)));
                timing.duration_ms = i64_to_u64(duration_ms);
            }
            _ => {}
        }
    }

    let mut by_job: HashMap<Uuid, Vec<JobHistoryPhaseTiming>> = HashMap::new();
    for ((job_id, _), timing) in phases {
        by_job.entry(job_id).or_default().push(timing);
    }
    for timings in by_job.values_mut() {
        timings.sort_by_key(|timing| timing.start_at);
    }
    Ok(by_job)
}

fn normalize_limit(limit: usize) -> usize {
    match limit {
        0 => 50,
        1..=500 => limit,
        _ => 500,
    }
}

async fn writer_loop(
    pool: PgPool,
    mut rx: mpsc::Receiver<JobHistoryWrite>,
    options: PostgresJobHistoryOptions,
) {
    let batch_size = options.batch_size.max(1);
    let flush_interval = options.flush_interval.max(Duration::from_millis(1));
    let mut interval = tokio::time::interval(flush_interval);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    interval.tick().await;

    let mut batch = Vec::with_capacity(batch_size);

    loop {
        tokio::select! {
            maybe_write = rx.recv() => {
                match maybe_write {
                    Some(write) => {
                        batch.push(write);
                        metrics::gauge!("coordinator_db_write_queue_depth")
                            .set((rx.len() + batch.len()) as f64);
                        if batch.len() >= batch_size {
                            flush_batch(&pool, &mut batch).await;
                            record_writer_queue_depth(&rx, &batch);
                        }
                    }
                    None => break,
                }
            }
            _ = interval.tick(), if !batch.is_empty() => {
                flush_batch(&pool, &mut batch).await;
                record_writer_queue_depth(&rx, &batch);
            }
        }
    }

    if !batch.is_empty() {
        flush_batch(&pool, &mut batch).await;
    }
    metrics::gauge!("coordinator_db_write_queue_depth").set(0.0);
}

async fn flush_batch(pool: &PgPool, batch: &mut Vec<JobHistoryWrite>) {
    let writes = std::mem::take(batch);
    if writes.is_empty() {
        return;
    }
    let write_counts = JobHistoryWriteCounts::from_writes(&writes);

    let started = Instant::now();
    let result = write_history_writes(pool, writes).await;
    record_query_metric("write_history", if result.is_ok() { "success" } else { "error" }, started);
    record_pool_metrics(pool);
    if let Err(error) = result {
        write_counts.record_dropped();
        warn!("failed to flush Postgres job history writes: {error:#}");
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct JobHistoryWriteCounts {
    events: u64,
    snapshots: u64,
}

impl JobHistoryWriteCounts {
    fn from_writes(writes: &[JobHistoryWrite]) -> Self {
        let mut counts = Self::default();
        for write in writes {
            match write {
                JobHistoryWrite::Event(_) => counts.events += 1,
                JobHistoryWrite::Snapshot(_) => counts.snapshots += 1,
            }
        }
        counts
    }

    fn record_dropped(self) {
        record_dropped_events(EVENT_TYPE_JOB_EVENT_LOG, self.events);
        record_dropped_events(EVENT_TYPE_JOB_SNAPSHOT, self.snapshots);
    }
}

async fn write_history_writes(pool: &PgPool, writes: Vec<JobHistoryWrite>) -> Result<()> {
    let mut tx = pool.begin().await.context("begin job history transaction")?;
    let mut projections: HashMap<Uuid, JobHistorySnapshot> = HashMap::new();
    for write in writes {
        match write {
            JobHistoryWrite::Event(envelope) => {
                let envelope = *envelope;
                write_event_envelope(&mut tx, &envelope).await?;
                if let Some(snapshot) = envelope.projection {
                    projections.insert(snapshot.job_id, snapshot);
                }
            }
            JobHistoryWrite::Snapshot(snapshot) => {
                let snapshot = *snapshot;
                let envelope = JobHistoryEventEnvelope::snapshot(snapshot);
                write_event_envelope(&mut tx, &envelope).await?;
                if let Some(snapshot) = envelope.projection {
                    projections.insert(snapshot.job_id, snapshot);
                }
            }
        }
    }

    let snapshots = projections.into_values().collect::<Vec<_>>();
    if !snapshots.is_empty() {
        write_snapshot_projection(&mut tx, &snapshots).await?;
    }
    update_projection_cursor(&mut tx).await?;
    tx.commit().await.context("commit job history transaction")
}

async fn write_event_envelope(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    envelope: &JobHistoryEventEnvelope,
) -> Result<()> {
    insert_job_event(tx, &envelope.event).await?;
    if let Some(snapshot) = envelope.projection.as_ref() {
        for event in derived_events_for_snapshot(snapshot) {
            insert_job_event(tx, &event).await?;
        }
    }
    Ok(())
}

async fn insert_job_event(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    event: &JobHistoryEvent,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO job_event_log (
            occurred_at, coordinator_id, job_id, worker_id, event_type,
            schema_version, dedupe_key, payload
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb)
        ON CONFLICT (dedupe_key) DO NOTHING
        "#,
    )
    .bind(event.occurred_at)
    .bind(&event.coordinator_id)
    .bind(event.job_id)
    .bind(event.worker_id.as_deref())
    .bind(&event.event_type)
    .bind(event.schema_version)
    .bind(&event.dedupe_key)
    .bind(event.payload.to_string())
    .execute(&mut **tx)
    .await
    .context("insert job event log row")?;
    Ok(())
}

async fn update_projection_cursor(tx: &mut sqlx::Transaction<'_, Postgres>) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE projection_cursors
        SET last_event_id = (SELECT COALESCE(MAX(event_id), 0) FROM job_event_log),
            updated_at = NOW()
        WHERE name = 'job_history'
        "#,
    )
    .execute(&mut **tx)
    .await
    .context("update job history projection cursor")?;
    Ok(())
}

fn derived_events_for_snapshot(snapshot: &JobHistorySnapshot) -> Vec<JobHistoryEvent> {
    let mut events = Vec::new();
    let program = if snapshot.program.is_empty() {
        crate::program_registry::UNKNOWN_PROGRAM_ALIAS.to_owned()
    } else {
        snapshot.program.clone()
    };
    let base_payload = json!({
        "job_id": snapshot.job_id,
        "hash_id": snapshot.hash_id,
        "program": program,
        "proof_type": snapshot.proof_type,
    });

    for worker_id in &snapshot.workers {
        let payload = extend_json(
            base_payload.clone(),
            json!({
                "worker_id": worker_id,
                "role": WORKER_ROLE_PARTICIPANT,
            }),
        );
        events.push(JobHistoryEvent::new(
            snapshot.coordinator_id.clone(),
            Some(snapshot.job_id),
            Some(worker_id.clone()),
            EVENT_TYPE_WORKER_ASSIGNED,
            snapshot.received_at.unwrap_or_else(Utc::now),
            format!(
                "{}:{}:{}:{}",
                EVENT_TYPE_WORKER_ASSIGNED, snapshot.job_id, worker_id, WORKER_ROLE_PARTICIPANT
            ),
            payload,
        ));
    }

    if let Some(worker_id) = &snapshot.agg_worker_id {
        let payload = extend_json(
            base_payload.clone(),
            json!({
                "worker_id": worker_id,
                "role": WORKER_ROLE_AGGREGATOR,
            }),
        );
        events.push(JobHistoryEvent::new(
            snapshot.coordinator_id.clone(),
            Some(snapshot.job_id),
            Some(worker_id.clone()),
            EVENT_TYPE_WORKER_ASSIGNED,
            snapshot.completed_at.or(snapshot.received_at).unwrap_or_else(Utc::now),
            format!(
                "{}:{}:{}:{}",
                EVENT_TYPE_WORKER_ASSIGNED, snapshot.job_id, worker_id, WORKER_ROLE_AGGREGATOR
            ),
            payload,
        ));
    }

    for timing in &snapshot.phase_timings {
        let started_payload = extend_json(
            base_payload.clone(),
            json!({
                "phase": timing.phase,
                "event": PHASE_EVENT_STARTED,
            }),
        );
        events.push(JobHistoryEvent::new(
            snapshot.coordinator_id.clone(),
            Some(snapshot.job_id),
            None,
            EVENT_TYPE_PHASE_STARTED,
            timing.start_at,
            format!(
                "{}:{}:{}:{}",
                EVENT_TYPE_PHASE_STARTED,
                snapshot.job_id,
                timing.phase,
                timing.start_at.timestamp_micros()
            ),
            started_payload,
        ));
        if let Some(end_at) = timing.end_at {
            let completed_payload = extend_json(
                base_payload.clone(),
                json!({
                    "phase": timing.phase,
                    "event": PHASE_EVENT_ENDED,
                    "duration_ms": timing.duration_ms,
                }),
            );
            events.push(JobHistoryEvent::new(
                snapshot.coordinator_id.clone(),
                Some(snapshot.job_id),
                None,
                EVENT_TYPE_PHASE_COMPLETED,
                end_at,
                format!(
                    "{}:{}:{}:{}",
                    EVENT_TYPE_PHASE_COMPLETED,
                    snapshot.job_id,
                    timing.phase,
                    end_at.timestamp_micros()
                ),
                completed_payload,
            ));
        }
    }

    events
}

fn snapshot_payload(snapshot: &JobHistorySnapshot) -> Value {
    serde_json::to_value(snapshot).unwrap_or_else(|error| {
        json!({
            "serialization_error": error.to_string(),
            "job_id": snapshot.job_id,
            "state": snapshot.state,
        })
    })
}

fn worker_error_history_event(event: &JobHistoryWorkerError) -> JobHistoryEvent {
    let payload = json!({
        "job_id": event.job_id,
        "worker_id": event.worker_id,
        "hash_id": event.hash_id,
        "program": event.program,
        "reason": event.reason,
        "message": event.message,
    });
    JobHistoryEvent::new(
        event.coordinator_id.clone(),
        Some(event.job_id),
        Some(event.worker_id.clone()),
        EVENT_TYPE_WORKER_ERROR,
        event.occurred_at,
        format!(
            "{}:{}:{}:{}:{}",
            EVENT_TYPE_WORKER_ERROR,
            event.job_id,
            event.worker_id,
            event.reason,
            payload_digest(&payload)
        ),
        payload,
    )
}

fn extend_json(mut base: Value, extra: Value) -> Value {
    if let (Some(base), Some(extra)) = (base.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            base.insert(key.clone(), value.clone());
        }
    }
    base
}

fn payload_digest(payload: &Value) -> String {
    let hash = blake3::hash(payload.to_string().as_bytes()).to_hex().to_string();
    hash.chars().take(16).collect()
}

async fn write_snapshot_projection(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    snapshots: &[JobHistorySnapshot],
) -> Result<()> {
    write_job_rows(tx, snapshots).await?;
    for snapshot in snapshots {
        write_worker_rows(tx, snapshot).await?;
        write_phase_events(tx, snapshot).await?;
    }
    Ok(())
}

async fn write_job_rows(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    snapshots: &[JobHistorySnapshot],
) -> Result<()> {
    let mut qb = QueryBuilder::<Postgres>::new(
        r#"
        INSERT INTO job_history_jobs (
            job_id, coordinator_id, hash_id, program, state, failure_reason, proof_type, received_at,
            completed_at, duration_ms, instances, executed_steps, agg_worker_id, updated_at
        )
        "#,
    );
    qb.push_values(snapshots, |mut row, snapshot| {
        row.push_bind(snapshot.job_id)
            .push_bind(&snapshot.coordinator_id)
            .push_bind(&snapshot.hash_id)
            .push_bind(&snapshot.program)
            .push_bind(&snapshot.state)
            .push_bind(&snapshot.failure_reason)
            .push_bind(&snapshot.proof_type)
            .push_bind(snapshot.received_at)
            .push_bind(snapshot.completed_at)
            .push_bind(option_u64_to_i64(snapshot.duration_ms))
            .push_bind(option_u64_to_i64(snapshot.instances))
            .push_bind(option_u64_to_i64(snapshot.executed_steps))
            .push_bind(&snapshot.agg_worker_id)
            .push("NOW()");
    });
    qb.push(
        r#"
        ON CONFLICT (job_id) DO UPDATE SET
            coordinator_id = EXCLUDED.coordinator_id,
            hash_id = EXCLUDED.hash_id,
            program = EXCLUDED.program,
            state = EXCLUDED.state,
            failure_reason = COALESCE(EXCLUDED.failure_reason, job_history_jobs.failure_reason),
            proof_type = EXCLUDED.proof_type,
            received_at = EXCLUDED.received_at,
            completed_at = EXCLUDED.completed_at,
            duration_ms = EXCLUDED.duration_ms,
            instances = EXCLUDED.instances,
            executed_steps = EXCLUDED.executed_steps,
            agg_worker_id = EXCLUDED.agg_worker_id,
            updated_at = NOW()
        "#,
    );
    qb.build().execute(&mut **tx).await.context("upsert job history rows")?;
    Ok(())
}

async fn write_worker_rows(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    snapshot: &JobHistorySnapshot,
) -> Result<()> {
    sqlx::query("DELETE FROM job_history_job_workers WHERE job_id = $1")
        .bind(snapshot.job_id)
        .execute(&mut **tx)
        .await
        .context("delete stale job worker rows")?;

    let mut rows = snapshot
        .workers
        .iter()
        .map(|worker_id| (worker_id.as_str(), WORKER_ROLE_PARTICIPANT))
        .collect::<Vec<_>>();
    if let Some(agg_worker_id) = &snapshot.agg_worker_id {
        rows.push((agg_worker_id.as_str(), WORKER_ROLE_AGGREGATOR));
    }
    if rows.is_empty() {
        return Ok(());
    }

    let mut qb = QueryBuilder::<Postgres>::new(
        "INSERT INTO job_history_job_workers (job_id, worker_id, role) ",
    );
    qb.push_values(rows, |mut row, (worker_id, role)| {
        row.push_bind(snapshot.job_id).push_bind(worker_id).push_bind(role);
    });
    qb.push(" ON CONFLICT (job_id, worker_id, role) DO NOTHING");
    qb.build().execute(&mut **tx).await.context("insert job worker rows")?;
    Ok(())
}

async fn write_phase_events(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    snapshot: &JobHistorySnapshot,
) -> Result<()> {
    if snapshot.phase_timings.is_empty() {
        return Ok(());
    }
    let mut rows = Vec::with_capacity(snapshot.phase_timings.len() * 2);
    for timing in &snapshot.phase_timings {
        rows.push((timing.phase.as_str(), PHASE_EVENT_STARTED, timing.start_at, None));
        if let Some(end_at) = timing.end_at {
            rows.push((timing.phase.as_str(), PHASE_EVENT_ENDED, end_at, timing.duration_ms));
        }
    }

    let mut qb = QueryBuilder::<Postgres>::new(
        "INSERT INTO job_history_phase_events (job_id, phase, event_type, occurred_at, duration_ms) ",
    );
    qb.push_values(rows, |mut row, (phase, event_type, occurred_at, duration_ms)| {
        row.push_bind(snapshot.job_id)
            .push_bind(phase)
            .push_bind(event_type)
            .push_bind(occurred_at)
            .push_bind(option_u64_to_i64(duration_ms));
    });
    qb.push(" ON CONFLICT (job_id, phase, event_type, occurred_at) DO NOTHING");
    qb.build().execute(&mut **tx).await.context("insert phase events")?;
    Ok(())
}

fn option_u64_to_i64(value: Option<u64>) -> Option<i64> {
    value.map(|value| value.min(i64::MAX as u64) as i64)
}

fn i64_to_u64(value: Option<i64>) -> Option<u64> {
    value.map(|value| value.max(0) as u64)
}

fn average_u64(values: &[u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    Some((values.iter().sum::<u64>() as f64 / values.len() as f64).round() as u64)
}

fn quantile_u64(values: &mut [u64], quantile: f64) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let rank = ((values.len() as f64 * quantile).ceil() as usize).saturating_sub(1);
    values.get(rank.min(values.len() - 1)).copied()
}

fn average_steps_per_second(jobs: &[JobHistoryJob]) -> Option<f64> {
    let rates = jobs
        .iter()
        .filter_map(|job| {
            let steps = job.executed_steps? as f64;
            let duration_ms = job.duration_ms? as f64;
            (duration_ms > 0.0).then_some(steps / (duration_ms / 1000.0))
        })
        .collect::<Vec<_>>();
    if rates.is_empty() {
        return None;
    }
    Some(rates.iter().sum::<f64>() / rates.len() as f64)
}

fn record_queue_depth<T>(tx: &mpsc::Sender<T>) {
    metrics::gauge!("coordinator_db_write_queue_depth")
        .set(tx.max_capacity().saturating_sub(tx.capacity()) as f64);
}

fn record_writer_queue_depth<T>(rx: &mpsc::Receiver<T>, batch: &[T]) {
    metrics::gauge!("coordinator_db_write_queue_depth").set((rx.len() + batch.len()) as f64);
}

fn record_dropped_event(event_type: &'static str) {
    record_dropped_events(event_type, 1);
}

fn record_dropped_events(event_type: &'static str, count: u64) {
    if count == 0 {
        return;
    }
    metrics::counter!("coordinator_db_write_dropped_total", "event_type" => event_type)
        .increment(count);
}

fn record_query_metric(op: &'static str, status: &'static str, started: Instant) {
    metrics::histogram!("coordinator_db_query_duration_seconds", "op" => op, "status" => status)
        .record(started.elapsed().as_secs_f64());
}

fn record_pool_metrics(pool: &PgPool) {
    metrics::gauge!("coordinator_db_pool_size", "state" => "active")
        .set(pool.size().saturating_sub(pool.num_idle() as u32) as f64);
    metrics::gauge!("coordinator_db_pool_size", "state" => "idle").set(pool.num_idle() as f64);
}

#[cfg(test)]
#[path = "../tests/unit/job_history_worker_error.rs"]
mod worker_error_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use zisk_cluster_common::{
        ComputeCapacity, DataId, HintsModeDto, InputsModeDto, JobExecutionMode, JobId, JobPhase,
        PhaseTimings, ProofKind, WorkerId,
    };

    fn test_job() -> Job {
        let worker = WorkerId::from("worker-a".to_owned());
        let mut job = Job::new(
            JobId::new(),
            DataId::default(),
            "hash-a".to_owned(),
            InputsModeDto::InputsNone,
            HintsModeDto::HintsNone,
            ComputeCapacity::from(1),
            ComputeCapacity::from(1),
            vec![worker.clone()],
            vec![vec![0]],
            JobExecutionMode::Standard,
            Default::default(),
            false,
            ProofKind::VadcopFinal,
        );
        let start = Utc::now() - chrono::Duration::seconds(2);
        job.phase_timings.insert(
            JobPhase::Prove,
            PhaseTimings {
                start_time: start,
                end_time: Some(start + chrono::Duration::milliseconds(1250)),
            },
        );
        job.duration_ms = Some(1250);
        job.terminated_at = Some(start + chrono::Duration::milliseconds(1250));
        job.executed_steps = Some(42);
        job.agg_worker_id = Some(worker);
        job
    }

    #[test]
    fn snapshot_from_job_preserves_named_phase_windows() {
        let snapshot = JobHistorySnapshot::from_job("coord-a", &test_job()).unwrap();
        assert_eq!(snapshot.coordinator_id, "coord-a");
        assert_eq!(snapshot.hash_id, "hash-a");
        assert_eq!(snapshot.program, "hash-a");
        assert_eq!(snapshot.failure_reason, None);
        assert_eq!(snapshot.phase_timings.len(), 1);
        assert_eq!(snapshot.phase_timings[0].phase, "Prove");
        assert_eq!(snapshot.phase_timings[0].duration_ms, Some(1250));
        assert_eq!(snapshot.executed_steps, Some(42));
    }

    #[test]
    fn derived_snapshot_events_use_snapshot_program_alias() {
        let mut snapshot = JobHistorySnapshot::from_job("coord-a", &test_job()).unwrap();
        snapshot.program = "hash-a1".to_owned();

        let events = derived_events_for_snapshot(&snapshot);

        assert!(!events.is_empty());
        assert!(events.iter().all(|event| event.payload["program"].as_str() == Some("hash-a1")));
    }

    #[test]
    fn running_job_reports_current_phase_and_ages() {
        let now = Utc::now();
        let started_at = now - chrono::Duration::seconds(90);
        let phase_started_at = now - chrono::Duration::seconds(30);
        let updated_at = now - chrono::Duration::seconds(5);
        let timings = vec![
            JobHistoryPhaseTiming {
                phase: "Contributions".to_owned(),
                start_at: started_at,
                end_at: Some(phase_started_at),
                duration_ms: Some(60_000),
            },
            JobHistoryPhaseTiming {
                phase: "Prove".to_owned(),
                start_at: phase_started_at,
                end_at: None,
                duration_ms: None,
            },
        ];

        assert_eq!(active_phase("Running (Prove)", &timings).unwrap().phase, "Prove");
        assert_eq!(job_age_seconds(Some(started_at), None, now), Some(90));
        assert_eq!(elapsed_seconds(phase_started_at, now), 30);
        assert_eq!(elapsed_seconds(updated_at, now), 5);
    }

    #[test]
    fn terminal_job_has_no_current_phase_but_keeps_total_age() {
        let now = Utc::now();
        let started_at = now - chrono::Duration::seconds(90);
        let completed_at = now - chrono::Duration::seconds(10);
        let timings = vec![JobHistoryPhaseTiming {
            phase: "Prove".to_owned(),
            start_at: started_at,
            end_at: Some(completed_at),
            duration_ms: Some(80_000),
        }];

        assert!(active_phase("Completed", &timings).is_none());
        assert_eq!(job_age_seconds(Some(started_at), Some(completed_at), now), Some(80));
    }

    #[test]
    fn short_job_label_is_stable_eight_hex_chars() {
        let job_id = Uuid::parse_str("25baf367-9ac2-49e5-857a-7384b4e11c28").unwrap();
        assert_eq!(short_job_label(job_id), "25baf367");
    }

    #[test]
    fn write_counts_preserve_failed_flush_drop_cardinality() {
        let snapshot = JobHistorySnapshot::from_job("coord-a", &test_job()).unwrap();
        let event = JobHistoryEvent::coordinator_event(
            "coord-a",
            "coordinator.test",
            Utc::now(),
            serde_json::json!({"ok": true}),
        );
        let writes = vec![
            JobHistoryWrite::Snapshot(Box::new(snapshot.clone())),
            JobHistoryWrite::Event(Box::new(JobHistoryEventEnvelope::event(event))),
            JobHistoryWrite::Snapshot(Box::new(snapshot)),
        ];

        assert_eq!(
            JobHistoryWriteCounts::from_writes(&writes),
            JobHistoryWriteCounts { events: 1, snapshots: 2 }
        );
    }
}
