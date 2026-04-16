use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use zisk_cluster_common::{
    ComputeCapacity, CoordinatorMessageDto, HintsModeDto, InputsModeDto, Job, JobExecutionMode,
    JobId, JobState, WorkerId, WorkerState,
};
use zisk_coordinator::{Config, Coordinator, CoordinatorResult, MessageSender};

/// A mock message sender that records all sent messages for test assertions.
pub struct MockMessageSender {
    pub messages: Arc<Mutex<Vec<CoordinatorMessageDto>>>,
}

impl MockMessageSender {
    pub fn new() -> (Self, Arc<Mutex<Vec<CoordinatorMessageDto>>>) {
        let messages = Arc::new(Mutex::new(Vec::new()));
        (Self { messages: messages.clone() }, messages)
    }
}

impl MessageSender for MockMessageSender {
    fn send(&self, msg: CoordinatorMessageDto) -> CoordinatorResult<()> {
        self.messages.lock().unwrap().push(msg);
        Ok(())
    }
}

/// Create a test Config with optional overrides.
pub fn test_config(overrides: impl FnOnce(&mut Config)) -> Config {
    let mut config = Config::load(None, None, None, true, false, None)
        .expect("Failed to create default test config");
    overrides(&mut config);
    config
}

/// Register N mock workers on the given coordinator's workers pool.
/// Returns (worker_ids, message_buffers) for each worker.
pub async fn register_mock_workers(
    coordinator: &Coordinator,
    n: usize,
) -> Vec<(WorkerId, Arc<Mutex<Vec<CoordinatorMessageDto>>>)> {
    let mut workers = Vec::with_capacity(n);
    for i in 0..n {
        let worker_id = WorkerId::from(format!("test-worker-{}", i));
        let (sender, messages) = MockMessageSender::new();
        coordinator
            .workers_pool()
            .register_worker(worker_id.clone(), 1u32, Box::new(sender))
            .await
            .unwrap();
        workers.push((worker_id, messages));
    }
    workers
}

/// Create a minimal test Job with the given worker IDs.
pub fn create_test_job(workers: &[WorkerId]) -> Job {
    let partitions: Vec<Vec<u32>> =
        workers.iter().enumerate().map(|(i, _)| vec![i as u32]).collect();
    Job::new(
        Default::default(),
        InputsModeDto::InputsNone,
        HintsModeDto::HintsNone,
        ComputeCapacity::from(workers.len() as u32),
        ComputeCapacity::from(1u32),
        workers.to_vec(),
        partitions,
        JobExecutionMode::Standard,
        BTreeMap::new(),
        false,
    )
}

/// Assert a job has the expected state.
pub async fn assert_job_state(coordinator: &Coordinator, job_id: &JobId, expected: JobState) {
    let job_entry = coordinator.jobs().read().await.get(job_id).cloned().expect("Job not found");
    let job = job_entry.read().await;
    assert_eq!(
        job.state, expected,
        "Expected job {} to be in state {:?}, got {:?}",
        job_id, expected, job.state
    );
}

/// Assert a worker has the expected state.
pub async fn assert_worker_state(
    coordinator: &Coordinator,
    worker_id: &WorkerId,
    expected: WorkerState,
) {
    let state = coordinator.workers_pool().worker_state(worker_id).await;
    assert_eq!(
        state,
        Some(expected.clone()),
        "Expected worker {} to be in state {:?}, got {:?}",
        worker_id,
        expected,
        state
    );
}

/// Get all messages sent to a worker (from the mock buffer).
pub fn get_cancellation_count(messages: &Arc<Mutex<Vec<CoordinatorMessageDto>>>) -> usize {
    messages
        .lock()
        .unwrap()
        .iter()
        .filter(|m| matches!(m, CoordinatorMessageDto::JobCancelled(_)))
        .count()
}
