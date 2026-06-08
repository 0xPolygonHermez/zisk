use super::{
    normalize_worker_error_limit, worker_error_reason, JobHistoryWorkerError, WorkerErrorQuery,
    WORKER_ERROR_MESSAGE_MAX_CHARS, WORKER_ERROR_QUERY_LIMIT_DEFAULT, WORKER_ERROR_QUERY_LIMIT_MAX,
};
use chrono::Utc;
use sqlx::types::Uuid;

#[test]
fn worker_error_query_deserializes_with_optional_fields() {
    let query: WorkerErrorQuery = serde_json::from_str("{}").unwrap();
    assert_eq!(query.limit, 0);
    assert!(query.worker_id.is_none());
    assert!(query.job_id.is_none());
    assert!(query.program.is_none());
    assert!(query.programs.is_empty());
    assert!(query.since.is_none());

    let job_id = Uuid::new_v4();
    let query: WorkerErrorQuery = serde_json::from_str(&format!(
        r#"{{"limit":42,"worker_id":"w-1","job_id":"{job_id}","program":"prog","programs":["other"],"since":"2026-05-21T00:00:00Z"}}"#
    ))
    .unwrap();
    assert_eq!(query.limit, 42);
    assert_eq!(query.worker_id.as_deref(), Some("w-1"));
    assert_eq!(query.job_id, Some(job_id));
    assert_eq!(query.program.as_deref(), Some("prog"));
    assert_eq!(query.programs, vec!["other"]);
    assert!(query.since.is_some());
}

#[test]
fn worker_error_query_limit_is_clamped() {
    assert_eq!(normalize_worker_error_limit(0), WORKER_ERROR_QUERY_LIMIT_DEFAULT);
    assert_eq!(normalize_worker_error_limit(50), 50);
    assert_eq!(
        normalize_worker_error_limit(WORKER_ERROR_QUERY_LIMIT_MAX + 7),
        WORKER_ERROR_QUERY_LIMIT_MAX,
    );
}

#[test]
fn worker_error_message_is_truncated_to_bound() {
    let mut event = JobHistoryWorkerError {
        coordinator_id: "c".into(),
        worker_id: "w".into(),
        job_id: Uuid::nil(),
        hash_id: "h".into(),
        program: "p".into(),
        reason: worker_error_reason::HEARTBEAT_LOST.into(),
        message: Some("x".repeat(WORKER_ERROR_MESSAGE_MAX_CHARS + 250)),
        occurred_at: Utc::now(),
    };
    event.truncate_message();
    assert_eq!(
        event.message.as_deref().map(str::chars).map(Iterator::count),
        Some(WORKER_ERROR_MESSAGE_MAX_CHARS),
    );

    // Idempotent on subsequent calls.
    event.truncate_message();
    assert_eq!(
        event.message.as_deref().map(str::chars).map(Iterator::count),
        Some(WORKER_ERROR_MESSAGE_MAX_CHARS),
    );

    // Multi-byte safe: doesn't panic on char boundaries.
    let mut event_utf8 = JobHistoryWorkerError {
        coordinator_id: "c".into(),
        worker_id: "w".into(),
        job_id: Uuid::nil(),
        hash_id: "h".into(),
        program: "p".into(),
        reason: worker_error_reason::UNKNOWN.into(),
        message: Some("漢".repeat(WORKER_ERROR_MESSAGE_MAX_CHARS + 10)),
        occurred_at: Utc::now(),
    };
    event_utf8.truncate_message();
    assert_eq!(
        event_utf8.message.as_deref().map(str::chars).map(Iterator::count),
        Some(WORKER_ERROR_MESSAGE_MAX_CHARS),
    );
}
