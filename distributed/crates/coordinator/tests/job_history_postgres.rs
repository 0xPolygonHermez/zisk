use std::time::Duration;

use chrono::Utc;
use sqlx::{types::Uuid, PgPool};
use zisk_coordinator::{
    start_postgres_job_history, worker_error_reason, JobHistoryListQuery, JobHistoryPhaseTiming,
    JobHistorySnapshot, JobHistoryWorkerError, PostgresJobHistoryOptions, WorkerErrorQuery,
    WORKER_ERROR_MESSAGE_MAX_CHARS,
};

fn test_database_url() -> Option<String> {
    std::env::var("ZISK_COORDINATOR_TEST_DATABASE_URL")
        .ok()
        .or_else(|| std::env::var("ZISK_COORDINATOR_DATABASE_URL").ok())
}

#[tokio::test]
async fn postgres_job_history_round_trip() {
    let Some(database_url) = test_database_url() else {
        eprintln!("skipping Postgres job-history test; set ZISK_COORDINATOR_TEST_DATABASE_URL");
        return;
    };

    let store = start_postgres_job_history(
        &database_url,
        PostgresJobHistoryOptions {
            flush_interval: Duration::from_millis(10),
            batch_size: 1,
            ..Default::default()
        },
    )
    .await
    .expect("start Postgres job history");

    let pool = PgPool::connect(&database_url).await.expect("connect for assertions");
    let now = Utc::now();
    let job_id = Uuid::from_u128(now.timestamp_nanos_opt().unwrap_or_default() as u128);
    let phase_started = now - chrono::Duration::milliseconds(1500);
    let snapshot = JobHistorySnapshot {
        coordinator_id: "coord-pg-test".to_owned(),
        job_id,
        hash_id: "hash-pg-test".to_owned(),
        program: "program-pg-test".to_owned(),
        state: "Completed".to_owned(),
        failure_reason: None,
        proof_type: "VadcopFinal".to_owned(),
        received_at: Some(phase_started),
        completed_at: Some(now),
        duration_ms: Some(1500),
        workers: vec!["worker-a".to_owned()],
        agg_worker_id: Some("worker-a".to_owned()),
        phase_timings: vec![JobHistoryPhaseTiming {
            phase: "Prove".to_owned(),
            start_at: phase_started,
            end_at: Some(now),
            duration_ms: Some(1500),
        }],
        instances: Some(7),
        executed_steps: Some(42),
    };

    store.try_record_snapshot(snapshot.clone());
    tokio::time::sleep(Duration::from_millis(100)).await;

    let last_success = store
        .last_successful_proof_timestamp("coord-pg-test")
        .await
        .expect("read last success")
        .expect("last success present");
    assert!(last_success >= phase_started);
    assert!(last_success <= Utc::now());

    let page = store
        .list_recent_jobs(JobHistoryListQuery {
            coordinator_id: Some("coord-pg-test".to_owned()),
            limit: 10,
            ..Default::default()
        })
        .await
        .expect("list recent jobs");
    let job =
        page.data.iter().find(|job| job.job_id == job_id).expect("inserted job in recent page");
    assert_eq!(job.coordinator_id, "coord-pg-test");
    assert_eq!(job.program, "program-pg-test");
    assert_eq!(job.state, "Completed");
    assert_eq!(job.failure_reason, None);
    assert_eq!(job.workers, vec!["worker-a"]);
    assert_eq!(job.agg_worker_id.as_deref(), Some("worker-a"));
    assert_eq!(job.phase_timings.len(), 1);
    assert_eq!(job.phase_timings[0].phase, "Prove");
    assert_eq!(job.phase_timings[0].duration_ms, Some(1500));

    let fetched = store.get_job(job_id).await.expect("get job").expect("job exists");
    assert_eq!(fetched.job_id, job_id);
    assert_eq!(fetched.program, "program-pg-test");
    assert_eq!(fetched.executed_steps, Some(42));
    assert_eq!(fetched.failure_reason, None);

    let dashboard_program: String =
        sqlx::query_scalar("SELECT program FROM zisk_dashboard_job_summary WHERE job_id = $1")
            .bind(job_id)
            .fetch_one(&pool)
            .await
            .expect("read dashboard summary program alias");
    assert_eq!(dashboard_program, "program-pg-test");

    let filtered = store
        .list_recent_jobs(JobHistoryListQuery {
            coordinator_id: Some("coord-pg-test".to_owned()),
            job_id: Some(job_id),
            limit: 10,
            ..Default::default()
        })
        .await
        .expect("list recent jobs by job_id");
    assert_eq!(filtered.data.len(), 1);
    assert_eq!(filtered.data[0].job_id, job_id);

    let failed_job_id = Uuid::from_u128(job_id.as_u128().wrapping_add(1));
    let mut failed_snapshot = snapshot_template(
        failed_job_id,
        "Failed",
        Some("Phase Aggregate timed out after 100s".to_owned()),
        phase_started,
        now,
    );
    failed_snapshot.completed_at = Some(now);
    store.try_record_snapshot(failed_snapshot);
    tokio::time::sleep(Duration::from_millis(100)).await;

    let failed = store.get_job(failed_job_id).await.expect("get failed job").expect("job exists");
    assert_eq!(failed.state, "Failed");
    assert_eq!(failed.failure_reason.as_deref(), Some("Phase Aggregate timed out after 100s"));

    let worker_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM job_history_job_workers WHERE job_id = $1")
            .bind(job_id)
            .fetch_one(&pool)
            .await
            .expect("count worker rows");
    assert_eq!(worker_rows, 2);

    let phase_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM job_history_phase_events WHERE job_id = $1")
            .bind(job_id)
            .fetch_one(&pool)
            .await
            .expect("count phase rows");
    assert_eq!(phase_rows, 2);

    let event_types: Vec<String> = sqlx::query_scalar(
        "SELECT event_type FROM job_event_log WHERE job_id = $1 ORDER BY event_id",
    )
    .bind(job_id)
    .fetch_all(&pool)
    .await
    .expect("list event log rows");
    assert!(event_types.iter().any(|event| event == "job.snapshot"));
    assert!(event_types.iter().any(|event| event == "worker.assigned"));
    assert!(event_types.iter().any(|event| event == "phase.started"));
    assert!(event_types.iter().any(|event| event == "phase.completed"));

    store.try_record_snapshot(snapshot);
    tokio::time::sleep(Duration::from_millis(100)).await;
    let event_count_after_retry: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM job_event_log WHERE job_id = $1")
            .bind(job_id)
            .fetch_one(&pool)
            .await
            .expect("count event rows after retry");
    assert_eq!(event_count_after_retry, event_types.len() as i64);

    let cursor: i64 = sqlx::query_scalar(
        "SELECT last_event_id FROM projection_cursors WHERE name = 'job_history'",
    )
    .fetch_one(&pool)
    .await
    .expect("read job history projection cursor");
    assert!(cursor > 0);

    sqlx::query("DELETE FROM job_event_log WHERE job_id = $1")
        .bind(job_id)
        .execute(&pool)
        .await
        .expect("cleanup test event rows");
    sqlx::query("DELETE FROM job_event_log WHERE job_id = $1")
        .bind(failed_job_id)
        .execute(&pool)
        .await
        .expect("cleanup failed event rows");
    sqlx::query("DELETE FROM job_history_jobs WHERE job_id = $1")
        .bind(job_id)
        .execute(&pool)
        .await
        .expect("cleanup test row");
    sqlx::query("DELETE FROM job_history_jobs WHERE job_id = $1")
        .bind(failed_job_id)
        .execute(&pool)
        .await
        .expect("cleanup failed test row");
}

#[tokio::test]
async fn postgres_worker_error_round_trip() {
    let Some(database_url) = test_database_url() else {
        eprintln!("skipping Postgres worker-error test; set ZISK_COORDINATOR_TEST_DATABASE_URL");
        return;
    };

    let store = start_postgres_job_history(
        &database_url,
        PostgresJobHistoryOptions {
            flush_interval: Duration::from_millis(10),
            batch_size: 1,
            ..Default::default()
        },
    )
    .await
    .expect("start Postgres job history");

    let pool = PgPool::connect(&database_url).await.expect("connect for assertions");
    let now = Utc::now();
    let job_id = Uuid::from_u128(now.timestamp_nanos_opt().unwrap_or_default() as u128);
    let worker_id = format!("worker-err-{}", now.timestamp_nanos_opt().unwrap_or_default());
    let program = format!("prog-err-{}", now.timestamp_nanos_opt().unwrap_or_default());

    let event = JobHistoryWorkerError {
        coordinator_id: "coord-pg-test".to_owned(),
        worker_id: worker_id.clone(),
        job_id,
        hash_id: "hash-pg-test".to_owned(),
        program: program.clone(),
        reason: worker_error_reason::HEARTBEAT_LOST.to_owned(),
        message: Some("x".repeat(WORKER_ERROR_MESSAGE_MAX_CHARS + 200)),
        occurred_at: now,
    };
    store.record_worker_error(event).await.expect("record worker error");

    let recent = store
        .recent_worker_errors(WorkerErrorQuery {
            limit: 10,
            worker_id: Some(worker_id.clone()),
            job_id: Some(job_id),
            program: None,
            programs: vec![program.clone()],
            since: Some(now - chrono::Duration::seconds(60)),
        })
        .await
        .expect("query recent worker errors");
    assert_eq!(recent.len(), 1);
    let row = &recent[0];
    assert_eq!(row.worker_id, worker_id);
    assert_eq!(row.program, program);
    assert_eq!(row.reason, worker_error_reason::HEARTBEAT_LOST);
    assert_eq!(
        row.message.as_deref().map(str::chars).map(Iterator::count),
        Some(WORKER_ERROR_MESSAGE_MAX_CHARS),
    );

    let event_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM job_event_log WHERE worker_id = $1 AND event_type = 'worker.error'",
    )
    .bind(&worker_id)
    .fetch_one(&pool)
    .await
    .expect("count worker error event rows");
    assert_eq!(event_count, 1);

    sqlx::query("DELETE FROM job_event_log WHERE worker_id = $1")
        .bind(&worker_id)
        .execute(&pool)
        .await
        .expect("cleanup worker error event rows");
    sqlx::query("DELETE FROM job_history_worker_errors WHERE worker_id = $1")
        .bind(&worker_id)
        .execute(&pool)
        .await
        .expect("cleanup worker error rows");
}

#[tokio::test]
async fn postgres_reconciles_interrupted_running_jobs() {
    let Some(database_url) = test_database_url() else {
        eprintln!("skipping Postgres reconciliation test; set ZISK_COORDINATOR_TEST_DATABASE_URL");
        return;
    };

    let store = start_postgres_job_history(
        &database_url,
        PostgresJobHistoryOptions {
            flush_interval: Duration::from_millis(10),
            batch_size: 1,
            ..Default::default()
        },
    )
    .await
    .expect("start Postgres job history");

    let pool = PgPool::connect(&database_url).await.expect("connect for assertions");
    let process_started_at = Utc::now();
    let received_at = process_started_at - chrono::Duration::minutes(7);
    let updated_at = process_started_at - chrono::Duration::minutes(5);
    let job_id = Uuid::from_u128(
        process_started_at.timestamp_nanos_opt().unwrap_or_default().wrapping_abs() as u128,
    );
    let other_coord_job_id = Uuid::from_u128(job_id.as_u128().wrapping_add(1));

    sqlx::query(
        r#"
        INSERT INTO job_history_jobs (
            job_id, coordinator_id, hash_id, program, state, proof_type, received_at, updated_at
        )
        VALUES
            ($1, 'coord-reconcile-test', 'hash-reconcile-test', 'program-reconcile-test',
             'Running (Prove)', 'VadcopFinal', $2, $3),
            ($4, 'other-coord-reconcile-test', 'hash-reconcile-test', 'program-reconcile-test',
             'Running (Prove)', 'VadcopFinal', $2, $3)
        ON CONFLICT (job_id) DO NOTHING
        "#,
    )
    .bind(job_id)
    .bind(received_at)
    .bind(updated_at)
    .bind(other_coord_job_id)
    .execute(&pool)
    .await
    .expect("insert stale running rows");

    let reconciled = store
        .reconcile_interrupted_jobs(
            "coord-reconcile-test",
            process_started_at,
            "coordinator_restarted_mid_run",
        )
        .await
        .expect("reconcile interrupted rows");
    assert_eq!(reconciled, 1);

    let row: (String, Option<String>, Option<chrono::DateTime<Utc>>, Option<i64>) = sqlx::query_as(
        r#"
        SELECT state, failure_reason, completed_at, duration_ms
        FROM job_history_jobs
        WHERE job_id = $1
        "#,
    )
    .bind(job_id)
    .fetch_one(&pool)
    .await
    .expect("read reconciled row");
    assert_eq!(row.0, "Failed");
    assert_eq!(row.1.as_deref(), Some("coordinator_restarted_mid_run"));
    assert!(row.2.is_some());
    assert!(row.3.is_some_and(|duration_ms| duration_ms >= 420_000));

    let other_state: String =
        sqlx::query_scalar("SELECT state FROM job_history_jobs WHERE job_id = $1")
            .bind(other_coord_job_id)
            .fetch_one(&pool)
            .await
            .expect("read unrelated coordinator row");
    assert_eq!(other_state, "Running (Prove)");

    sqlx::query("DELETE FROM job_history_jobs WHERE job_id IN ($1, $2)")
        .bind(job_id)
        .bind(other_coord_job_id)
        .execute(&pool)
        .await
        .expect("cleanup reconciliation rows");
}

fn snapshot_template(
    job_id: Uuid,
    state: &str,
    failure_reason: Option<String>,
    phase_started: chrono::DateTime<Utc>,
    completed_at: chrono::DateTime<Utc>,
) -> JobHistorySnapshot {
    JobHistorySnapshot {
        coordinator_id: "coord-pg-test".to_owned(),
        job_id,
        hash_id: "hash-pg-test".to_owned(),
        program: "program-pg-test".to_owned(),
        state: state.to_owned(),
        failure_reason,
        proof_type: "VadcopFinal".to_owned(),
        received_at: Some(phase_started),
        completed_at: Some(completed_at),
        duration_ms: Some(1500),
        workers: vec!["worker-a".to_owned()],
        agg_worker_id: Some("worker-a".to_owned()),
        phase_timings: vec![JobHistoryPhaseTiming {
            phase: "Prove".to_owned(),
            start_at: phase_started,
            end_at: Some(completed_at),
            duration_ms: Some(1500),
        }],
        instances: Some(7),
        executed_steps: Some(42),
    }
}
