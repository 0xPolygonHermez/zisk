mod common;

use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;
use zisk_cluster_common::{
    ComputeCapacity, CoordinatorMessageDto, JobId, JobPhase, JobState, PhaseTimings,
    ReconnectionDirectiveDto, WorkerErrorDto, WorkerId, WorkerReconnectRequestDto,
    WorkerRegisterRequestDto, WorkerState,
};
use zisk_distributed_coordinator::Coordinator;

use common::*;

/// Helper: Create a coordinator, register workers, insert a Running job,
/// mark workers as Computing, and return all handles.
async fn setup_running_job(
    n_workers: usize,
    phase: JobPhase,
    config_overrides: impl FnOnce(&mut zisk_distributed_coordinator::Config),
) -> SetupResult {
    let config = test_config(config_overrides);
    let coordinator = Arc::new(Coordinator::new(config));
    let workers = register_mock_workers(&coordinator, n_workers).await;

    let worker_ids: Vec<_> = workers.iter().map(|(id, _)| id.clone()).collect();
    let mut job = create_test_job(&worker_ids);
    job.change_state(JobState::Running(phase.clone()));
    let job_id = job.job_id.clone();

    // Mark workers as computing
    for wid in &worker_ids {
        coordinator
            .workers_pool()
            .mark_worker_with_state(wid, WorkerState::Computing((job_id.clone(), phase.clone())))
            .await
            .unwrap();
    }

    // Insert job into coordinator's
    coordinator.jobs().write().await.insert(job_id.clone(), Arc::new(RwLock::new(job)));

    SetupResult { coordinator, workers, job_id }
}

struct SetupResult {
    coordinator: Arc<Coordinator>,
    workers: Vec<(
        zisk_cluster_common::WorkerId,
        std::sync::Arc<std::sync::Mutex<Vec<CoordinatorMessageDto>>>,
    )>,
    job_id: zisk_cluster_common::JobId,
}

// ──────────────────────────────────────────────────────────────────────
// Phase timeout tests
// ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_phase1_timeout_aborts_job() {
    let s = setup_running_job(3, JobPhase::Contributions, |c| {
        c.coordinator.phase1_timeout_seconds = 1;
    })
    .await;

    // Backdate the start_time to 2 seconds ago
    {
        let entry = s.coordinator.jobs().read().await.get(&s.job_id).cloned().unwrap();
        let mut job = entry.write().await;
        job.phase_timings.insert(
            JobPhase::Contributions,
            PhaseTimings { start_time: Utc::now() - chrono::Duration::seconds(2), end_time: None },
        );
    }

    s.coordinator.run_monitor_sweep().await;

    assert_job_state(&s.coordinator, &s.job_id, JobState::Failed).await;

    // All workers should have received a JobCancelled message
    for (_, msgs) in &s.workers {
        assert!(get_cancellation_count(msgs) >= 1, "Expected at least one cancellation message");
    }
}

#[tokio::test]
async fn test_phase2_timeout_aborts_job() {
    let s = setup_running_job(2, JobPhase::Prove, |c| {
        c.coordinator.phase2_timeout_seconds = 1;
    })
    .await;

    {
        let entry = s.coordinator.jobs().read().await.get(&s.job_id).cloned().unwrap();
        let mut job = entry.write().await;
        job.phase_timings.insert(
            JobPhase::Prove,
            PhaseTimings { start_time: Utc::now() - chrono::Duration::seconds(2), end_time: None },
        );
    }

    s.coordinator.run_monitor_sweep().await;

    assert_job_state(&s.coordinator, &s.job_id, JobState::Failed).await;
    for (_, msgs) in &s.workers {
        assert!(get_cancellation_count(msgs) >= 1);
    }
}

#[tokio::test]
async fn test_phase3_timeout_aborts_job() {
    let s = setup_running_job(1, JobPhase::Aggregate, |c| {
        c.coordinator.phase3_timeout_seconds = 1;
    })
    .await;

    {
        let entry = s.coordinator.jobs().read().await.get(&s.job_id).cloned().unwrap();
        let mut job = entry.write().await;
        job.phase_timings.insert(
            JobPhase::Aggregate,
            PhaseTimings { start_time: Utc::now() - chrono::Duration::seconds(2), end_time: None },
        );
    }

    s.coordinator.run_monitor_sweep().await;

    assert_job_state(&s.coordinator, &s.job_id, JobState::Failed).await;
}

#[tokio::test]
async fn test_phase_timeout_no_false_positive() {
    let s = setup_running_job(2, JobPhase::Contributions, |c| {
        c.coordinator.phase1_timeout_seconds = 300;
    })
    .await;

    // start_time is fresh — job should NOT be failed
    s.coordinator.run_monitor_sweep().await;

    assert_job_state(&s.coordinator, &s.job_id, JobState::Running(JobPhase::Contributions)).await;
}

// ──────────────────────────────────────────────────────────────────────
// Heartbeat staleness tests
// ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_stale_heartbeat_aborts_job() {
    let s = setup_running_job(2, JobPhase::Contributions, |c| {
        c.coordinator.heartbeat_interval_seconds = 30;
        c.coordinator.heartbeat_max_missed = 3;
        // Large phase timeout so it doesn't interfere
        c.coordinator.phase1_timeout_seconds = 9999;
    })
    .await;

    // Set worker 0's heartbeat to 100 seconds ago (stale with 30s*3=90s threshold)
    let old_time = Utc::now() - chrono::Duration::seconds(100);
    s.coordinator.workers_pool().set_last_heartbeat(&s.workers[0].0, old_time).await.unwrap();

    s.coordinator.run_monitor_sweep().await;

    assert_job_state(&s.coordinator, &s.job_id, JobState::Failed).await;

    // Both workers should get cancellations
    for (_, msgs) in &s.workers {
        assert!(get_cancellation_count(msgs) >= 1);
    }
}

#[tokio::test]
async fn test_healthy_heartbeat_no_abort() {
    let s = setup_running_job(2, JobPhase::Contributions, |c| {
        c.coordinator.heartbeat_interval_seconds = 30;
        c.coordinator.heartbeat_max_missed = 3;
        c.coordinator.phase1_timeout_seconds = 9999;
    })
    .await;

    // All heartbeats are fresh — no failures
    s.coordinator.run_monitor_sweep().await;

    assert_job_state(&s.coordinator, &s.job_id, JobState::Running(JobPhase::Contributions)).await;

    // No cancellation messages
    for (_, msgs) in &s.workers {
        assert_eq!(get_cancellation_count(msgs), 0);
    }
}

// ──────────────────────────────────────────────────────────────────────
// Worker disconnection tests
// ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_worker_disconnect_aborts_and_cancels() {
    let s = setup_running_job(3, JobPhase::Contributions, |c| {
        c.coordinator.phase1_timeout_seconds = 9999;
    })
    .await;

    let w0_id = &s.workers[0].0;
    s.coordinator.disconnect_worker(w0_id).await.unwrap();

    assert_job_state(&s.coordinator, &s.job_id, JobState::Failed).await;

    // Worker 0 is Disconnected, workers 1+2 should be Idle
    assert_worker_state(&s.coordinator, w0_id, WorkerState::Disconnected).await;
    assert_worker_state(&s.coordinator, &s.workers[1].0, WorkerState::Idle).await;
    assert_worker_state(&s.coordinator, &s.workers[2].0, WorkerState::Idle).await;

    // Workers 1+2 should have received cancellation messages
    assert!(get_cancellation_count(&s.workers[1].1) >= 1);
    assert!(get_cancellation_count(&s.workers[2].1) >= 1);
}

// ──────────────────────────────────────────────────────────────────────
// Idempotency tests
// ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_double_fail_is_idempotent() {
    let s = setup_running_job(2, JobPhase::Contributions, |c| {
        c.coordinator.phase1_timeout_seconds = 9999;
    })
    .await;

    // First fail
    s.coordinator.fail_job(&s.job_id, "first failure").await.unwrap();
    assert_job_state(&s.coordinator, &s.job_id, JobState::Failed).await;

    // Count cancellations after first fail
    let count_after_first: usize = s.workers.iter().map(|(_, m)| get_cancellation_count(m)).sum();

    // Second fail — should be a no-op
    s.coordinator.fail_job(&s.job_id, "second failure").await.unwrap();

    // No additional cancellation messages
    let count_after_second: usize = s.workers.iter().map(|(_, m)| get_cancellation_count(m)).sum();
    assert_eq!(
        count_after_first, count_after_second,
        "Second fail_job should not send additional cancellations"
    );
}

#[tokio::test]
async fn test_monitor_sweep_does_not_fail_completed_jobs() {
    let config = test_config(|c| {
        c.coordinator.phase1_timeout_seconds = 1;
    });
    let coordinator = Arc::new(Coordinator::new(config));
    let workers = register_mock_workers(&coordinator, 2).await;

    let worker_ids: Vec<_> = workers.iter().map(|(id, _)| id.clone()).collect();
    let mut job = create_test_job(&worker_ids);

    // Set job to Completed state
    job.change_state(JobState::Running(JobPhase::Contributions));
    job.change_state(JobState::Completed);
    let job_id = job.job_id.clone();
    coordinator.jobs().write().await.insert(job_id.clone(), Arc::new(RwLock::new(job)));

    coordinator.run_monitor_sweep().await;

    // Job stays Completed
    assert_job_state(&coordinator, &job_id, JobState::Completed).await;
}

// ──────────────────────────────────────────────────────────────────────
// Reconnection race / generation tests
// ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_microcut_reconnection_preserves_connection() {
    let config = test_config(|_| {});
    let coordinator = Arc::new(Coordinator::new(config));

    // Register worker (gen 0)
    let workers = register_mock_workers(&coordinator, 1).await;
    let w0_id = &workers[0].0;
    assert_eq!(coordinator.workers_pool().connection_generation(w0_id).await, Some(0));

    // Disconnect (simulates micro-cut)
    coordinator.workers_pool().disconnect_worker(w0_id).await.unwrap();

    // Re-register (gen bumps to 1)
    let (sender, _msgs) = MockMessageSender::new();
    coordinator
        .workers_pool()
        .register_worker(w0_id.clone(), 1u32, Box::new(sender))
        .await
        .unwrap();
    assert_eq!(coordinator.workers_pool().connection_generation(w0_id).await, Some(1));

    // Stale guard fires with gen 0 — should be a no-op
    coordinator.workers_pool().disconnect_worker_if_generation(w0_id, 0).await.unwrap();

    // Worker should still be Idle (not Disconnected)
    assert_worker_state(&coordinator, w0_id, WorkerState::Idle).await;
}

// ──────────────────────────────────────────────────────────────────────
// Stale disconnected cleanup
// ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_stale_disconnected_cleanup() {
    let config = test_config(|_| {});
    let coordinator = Arc::new(Coordinator::new(config));
    let workers = register_mock_workers(&coordinator, 1).await;
    let w0_id = &workers[0].0;

    coordinator.workers_pool().disconnect_worker(w0_id).await.unwrap();

    // Set heartbeat to 10 minutes ago
    let old_time = Utc::now() - chrono::Duration::seconds(600);
    coordinator.workers_pool().set_last_heartbeat(w0_id, old_time).await.unwrap();

    coordinator.run_monitor_sweep().await;

    // Worker entry should be removed
    assert_eq!(coordinator.workers_pool().num_workers().await, 0);
}

// ──────────────────────────────────────────────────────────────────────
// Worker error tests
// ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_worker_error_aborts_and_cancels() {
    let s = setup_running_job(3, JobPhase::Contributions, |c| {
        c.coordinator.phase1_timeout_seconds = 9999;
    })
    .await;

    let w0_id = &s.workers[0].0;

    // Worker 0 reports an error
    let error_dto = WorkerErrorDto {
        worker_id: w0_id.clone(),
        job_id: s.job_id.clone(),
        error_message: "computation failed".to_string(),
    };
    // handle_stream_error calls fail_job internally
    s.coordinator.handle_stream_error(error_dto).await.unwrap();

    assert_job_state(&s.coordinator, &s.job_id, JobState::Failed).await;

    // Workers 1+2 should have received JobCancelled
    assert!(get_cancellation_count(&s.workers[1].1) >= 1);
    assert!(get_cancellation_count(&s.workers[2].1) >= 1);

    // All workers should be Idle after cleanup
    for (wid, _) in &s.workers {
        assert_worker_state(&s.coordinator, wid, WorkerState::Idle).await;
    }
}

// ──────────────────────────────────────────────────────────────────────
// Reconciliation protocol tests
// ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_reconnect_idle_no_directive() {
    let config = test_config(|_| {});
    let coordinator = Arc::new(Coordinator::new(config));

    let (sender, _msgs) = MockMessageSender::new();
    let req = WorkerRegisterRequestDto {
        worker_id: WorkerId::from("w1".to_string()),
        compute_capacity: ComputeCapacity::from(1u32),
    };
    let (accepted, _msg) = coordinator.handle_stream_registration(req, Box::new(sender)).await;

    assert!(accepted);
    // Registration (not reconnection) never produces a directive
}

#[tokio::test]
async fn test_reconnect_unknown_job_gets_cancel() {
    let config = test_config(|_| {});
    let coordinator = Arc::new(Coordinator::new(config));

    // Register worker first so pool accepts reconnection
    let workers = register_mock_workers(&coordinator, 1).await;
    let w0_id = &workers[0].0;
    coordinator.workers_pool().disconnect_worker(w0_id).await.unwrap();

    let (sender, _msgs) = MockMessageSender::new();
    let req = WorkerReconnectRequestDto {
        worker_id: w0_id.clone(),
        compute_capacity: ComputeCapacity::from(1u32),
        last_known_job_id: Some(JobId::from("nonexistent-job".to_string())),
    };
    let (accepted, _msg, directive) =
        coordinator.handle_stream_reconnection(req, Box::new(sender)).await;

    assert!(accepted);
    assert_eq!(directive, Some(ReconnectionDirectiveDto::CancelStaleJob),);
}

#[tokio::test]
async fn test_reconnect_terminal_job_gets_cancel() {
    let s = setup_running_job(2, JobPhase::Contributions, |c| {
        c.coordinator.phase1_timeout_seconds = 9999;
    })
    .await;

    // Fail the job (terminal state)
    s.coordinator.fail_job(&s.job_id, "terminated").await.unwrap();

    // Worker is already Idle from fail_job cleanup; disconnect it
    let w0_id = &s.workers[0].0;
    s.coordinator.workers_pool().disconnect_worker(w0_id).await.unwrap();

    let (sender, _msgs) = MockMessageSender::new();
    let req = WorkerReconnectRequestDto {
        worker_id: w0_id.clone(),
        compute_capacity: ComputeCapacity::from(1u32),
        last_known_job_id: Some(s.job_id.clone()),
    };
    let (accepted, _msg, directive) =
        s.coordinator.handle_stream_reconnection(req, Box::new(sender)).await;

    assert!(accepted);
    assert_eq!(directive, Some(ReconnectionDirectiveDto::CancelStaleJob),);
    // Job state unchanged (still Failed)
    assert_job_state(&s.coordinator, &s.job_id, JobState::Failed).await;
}

#[tokio::test]
async fn test_reconnect_active_job_resumes() {
    let s = setup_running_job(2, JobPhase::Contributions, |c| {
        c.coordinator.phase1_timeout_seconds = 9999;
    })
    .await;

    // Pool-level disconnect only (don't trigger coordinator-level fail_job)
    let w0_id = &s.workers[0].0;
    s.coordinator.workers_pool().disconnect_worker(w0_id).await.unwrap();

    // Manually reset job to Running to simulate the case where guard hasn't fired yet
    {
        let entry = s.coordinator.jobs().read().await.get(&s.job_id).cloned().unwrap();
        let mut job = entry.write().await;
        job.state = JobState::Running(JobPhase::Contributions);
    }

    let (sender, _msgs) = MockMessageSender::new();
    let req = WorkerReconnectRequestDto {
        worker_id: w0_id.clone(),
        compute_capacity: ComputeCapacity::from(1u32),
        last_known_job_id: Some(s.job_id.clone()),
    };
    let (accepted, _msg, directive) =
        s.coordinator.handle_stream_reconnection(req, Box::new(sender)).await;

    assert!(accepted);
    assert_eq!(directive, Some(ReconnectionDirectiveDto::KeepComputing));
    // Job should remain running — worker keeps computing
    assert_job_state(&s.coordinator, &s.job_id, JobState::Running(JobPhase::Contributions)).await;
}

#[tokio::test]
async fn test_reconnect_no_stale_job_no_directive() {
    let config = test_config(|_| {});
    let coordinator = Arc::new(Coordinator::new(config));

    let workers = register_mock_workers(&coordinator, 1).await;
    let w0_id = &workers[0].0;
    coordinator.workers_pool().disconnect_worker(w0_id).await.unwrap();

    let (sender, _msgs) = MockMessageSender::new();
    let req = WorkerReconnectRequestDto {
        worker_id: w0_id.clone(),
        compute_capacity: ComputeCapacity::from(1u32),
        last_known_job_id: None,
    };
    let (accepted, _msg, directive) =
        coordinator.handle_stream_reconnection(req, Box::new(sender)).await;

    assert!(accepted);
    assert!(directive.is_none(), "Reconnect without stale job should have no directive");
}
