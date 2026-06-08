use super::Coordinator;
use crate::{
    config::Config,
    job_events::{CoordinatorExecutionStats, CoordinatorJobEvent, CoordinatorJobResult},
    job_history::{
        JobHistoryEvent, JobHistoryEventEnvelope, JobHistoryJob, JobHistoryListQuery,
        JobHistoryPage, JobHistoryPagination, JobHistorySnapshot, JobHistoryStore,
        EVENT_TYPE_JOB_SUCCEEDED,
    },
};
use chrono::{DateTime, Utc};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use tokio::sync::RwLock;
use zisk_cluster_common::{
    ComputeCapacity, HintsModeDto, InputsModeDto, Job, JobExecutionMode, JobPhase, JobState,
    ProofKind, WorkerId,
};

const COLLIDING_PROGRAM_HASH_A: &str =
    "7f83b1650f8c4d2a91e6b7304c5a2d8f3b9e61007acdd15ef2438769b5c4a301";
const COLLIDING_PROGRAM_HASH_B: &str =
    "7f83b165c8d41e7a209fb35e6a1c4d908f7ab63215e0dc49a12f87643bd509ef";

#[derive(Default)]
struct RecordingJobHistoryEvents {
    events: Mutex<Vec<JobHistoryEvent>>,
}

impl JobHistoryStore for RecordingJobHistoryEvents {
    fn try_record_snapshot(&self, _snapshot: JobHistorySnapshot) {}

    fn try_record_event(&self, event: JobHistoryEvent) {
        self.events.lock().unwrap().push(event);
    }

    fn try_record_event_envelope(&self, envelope: JobHistoryEventEnvelope) {
        self.events.lock().unwrap().push(envelope.event);
    }

    fn list_recent_jobs<'a>(
        &'a self,
        _query: JobHistoryListQuery,
    ) -> futures::future::BoxFuture<'a, anyhow::Result<JobHistoryPage>> {
        Box::pin(async {
            Ok(JobHistoryPage {
                data: Vec::new(),
                pagination: JobHistoryPagination { limit: 0, next_cursor: None, has_more: false },
            })
        })
    }

    fn get_job<'a>(
        &'a self,
        _job_id: uuid::Uuid,
    ) -> futures::future::BoxFuture<'a, anyhow::Result<Option<JobHistoryJob>>> {
        Box::pin(async { Ok(None) })
    }

    fn last_successful_proof_timestamp<'a>(
        &'a self,
        _coordinator_id: &'a str,
    ) -> futures::future::BoxFuture<'a, anyhow::Result<Option<DateTime<Utc>>>> {
        Box::pin(async { Ok(None) })
    }
}

fn test_config() -> Config {
    Config::load(None, None, None, true, None).expect("failed to create default test config")
}

fn create_test_job(worker: WorkerId) -> Job {
    Job::new(
        Default::default(),
        String::new(),
        InputsModeDto::InputsNone,
        HintsModeDto::HintsNone,
        ComputeCapacity::from(1u32),
        ComputeCapacity::from(1u32),
        vec![worker],
        vec![vec![0]],
        JobExecutionMode::Standard,
        BTreeMap::new(),
        false,
        ProofKind::VadcopFinal,
    )
}

#[tokio::test]
async fn lifecycle_history_payload_uses_registered_program_alias() {
    let store = Arc::new(RecordingJobHistoryEvents::default());
    let coordinator =
        Coordinator::new_with_job_history(test_config(), "coord-test".to_owned(), store.clone());

    coordinator.register_program_alias(COLLIDING_PROGRAM_HASH_A);
    let program_alias = coordinator.register_program_alias(COLLIDING_PROGRAM_HASH_B);
    let default_alias = &COLLIDING_PROGRAM_HASH_B[..8];
    assert!(program_alias.starts_with(default_alias));
    assert_eq!(program_alias.len(), default_alias.len() + 1);

    let worker = WorkerId::from("worker-a".to_owned());
    let mut job = create_test_job(worker);
    job.hash_id = COLLIDING_PROGRAM_HASH_B.to_owned();
    job.change_state(JobState::Running(JobPhase::Prove));
    job.change_state(JobState::Completed);
    job.terminated_at = Some(Utc::now());
    let job_id = job.job_id.clone();
    coordinator.jobs.write().await.insert(job_id.clone(), Arc::new(RwLock::new(job)));
    coordinator.alloc_job_events(&job_id).await;

    coordinator
        .fire_job_event(
            &job_id,
            CoordinatorJobEvent::Completed(CoordinatorJobResult::Prove {
                proof_bytes: vec![],
                stats: CoordinatorExecutionStats::default(),
            }),
        )
        .await;

    let events = store.events.lock().unwrap();
    let completed_event = events
        .iter()
        .find(|event| event.event_type == EVENT_TYPE_JOB_SUCCEEDED)
        .expect("expected completed lifecycle event");
    assert_eq!(completed_event.payload["program"].as_str(), Some(program_alias.as_str()));
    assert_eq!(completed_event.payload["hash_id"].as_str(), Some(COLLIDING_PROGRAM_HASH_B));
}
