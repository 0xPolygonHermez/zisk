//! Embedded coordinator backend — calls `Arc<Coordinator>` directly with no gRPC hop.
//!
//! Used when `backend.mode = "embedded"`: the coordinator runs in the same process as the
//! gateway. Workers still connect over gRPC to the coordinator's worker-facing port.

use std::{collections::HashMap, sync::Arc, time::Duration};

use async_stream::stream;
use async_trait::async_trait;
use chrono::Utc;
use tokio::{sync::RwLock, time::timeout};
use tracing::warn;
use uuid::Uuid;
use zisk_distributed_common::JobPhase;
use zisk_distributed_coordinator::{
    job_events::{CoordinatorExecutionStats, CoordinatorJobEvent, CoordinatorJobResult},
    Coordinator,
};

use super::{
    BackendService, DomainExecutionStats, DomainInputKind, DomainJobEvent, DomainJobEventCancelled,
    DomainJobEventCompleted, DomainJobEventFailed, DomainJobEventProgress, DomainJobEventQueued,
    DomainJobEventStarted, DomainJobEventWaitingForInput, DomainJobFailure, DomainJobKind,
    DomainJobKindResponse, DomainJobPhase, DomainJobStatus, DomainProof, DomainProofKind,
    InputChunkStream, JobEventStream, WaitResult,
};
use crate::errors::{internal, GatewayError, GatewayResult};
use zisk_distributed_common::{DataId, HintsModeDto, InputsModeDto, LaunchProofRequestDto};

pub struct EmbeddedCoordinatorBackend {
    coordinator: Arc<Coordinator>,
    /// job_id (UUID string) → hash_id: needed to populate `DomainProof.hash_id`.
    /// Entries are removed once the job reaches a terminal state.
    job_hash: Arc<RwLock<HashMap<String, String>>>,
}

impl EmbeddedCoordinatorBackend {
    pub fn new(coordinator: Arc<Coordinator>) -> Self {
        Self { coordinator, job_hash: Arc::new(RwLock::new(HashMap::new())) }
    }
}

// ── Type mapping helpers ─────────────────────────────────────────────────────

fn make_proof(hash_id: String, data: Vec<u8>) -> DomainProof {
    DomainProof {
        proof_id: Uuid::new_v4(),
        hash_id,
        verification_key: vec![],
        proof_kind: DomainProofKind::Stark,
        data,
        public_inputs: vec![],
        started_at: Utc::now(),
        completed_at: Utc::now(),
    }
}

fn coord_stats_to_domain(s: CoordinatorExecutionStats) -> DomainExecutionStats {
    DomainExecutionStats {
        steps: s.steps,
        duration_nanos: s.duration_nanos,
        main_cost: s.main_cost,
        opcode_cost: s.opcode_cost,
        memory_cost: s.memory_cost,
        precompile_cost: s.precompile_cost,
        tables_cost: s.tables_cost,
        other_cost: s.other_cost,
    }
}

fn coord_result_to_domain(result: CoordinatorJobResult, hash_id: &str) -> DomainJobKindResponse {
    match result {
        CoordinatorJobResult::Setup => DomainJobKindResponse::Setup,
        CoordinatorJobResult::Prove { proof_bytes, stats } => DomainJobKindResponse::Prove {
            proof: make_proof(hash_id.to_string(), proof_bytes),
            stats: coord_stats_to_domain(stats),
        },
        CoordinatorJobResult::Execute { stats, public_outputs } => {
            DomainJobKindResponse::Execute { stats: coord_stats_to_domain(stats), public_outputs }
        }
        CoordinatorJobResult::Wrap { proof_bytes } => {
            DomainJobKindResponse::Wrap(make_proof(hash_id.to_string(), proof_bytes))
        }
    }
}

fn coord_phase_to_domain(phase: &JobPhase) -> DomainJobPhase {
    match phase {
        JobPhase::Contributions
        | JobPhase::ContributionsInputsStream
        | JobPhase::ContributionsHintsStream
        | JobPhase::Execution => DomainJobPhase::Contributions,
        JobPhase::Prove => DomainJobPhase::Prove,
        JobPhase::Aggregate => DomainJobPhase::Aggregate,
    }
}

fn coord_event_to_domain(
    event: CoordinatorJobEvent,
    job_id: Uuid,
    hash_id: &str,
) -> Option<DomainJobEvent> {
    let ts = Utc::now();
    match event {
        CoordinatorJobEvent::Queued => {
            Some(DomainJobEvent::Queued(DomainJobEventQueued { job_id, timestamp: ts }))
        }
        CoordinatorJobEvent::Started => {
            Some(DomainJobEvent::Started(DomainJobEventStarted { job_id, timestamp: ts }))
        }
        CoordinatorJobEvent::Progress(phase) => {
            Some(DomainJobEvent::Progress(DomainJobEventProgress {
                job_id,
                phase: coord_phase_to_domain(&phase),
                timestamp: ts,
            }))
        }
        CoordinatorJobEvent::WaitingForInput => {
            Some(DomainJobEvent::WaitingForInput(DomainJobEventWaitingForInput {
                job_id,
                timestamp: ts,
            }))
        }
        CoordinatorJobEvent::Completed(result) => {
            Some(DomainJobEvent::Completed(DomainJobEventCompleted {
                job_id,
                result: coord_result_to_domain(result, hash_id),
                timestamp: ts,
            }))
        }
        CoordinatorJobEvent::Failed(reason) => Some(DomainJobEvent::Failed(DomainJobEventFailed {
            job_id,
            failure: DomainJobFailure::Execution { reason },
            timestamp: ts,
        })),
        CoordinatorJobEvent::Cancelled => {
            Some(DomainJobEvent::Cancelled(DomainJobEventCancelled { job_id, timestamp: ts }))
        }
    }
}

fn domain_input_to_dto(input: &DomainInputKind) -> InputsModeDto {
    match input {
        DomainInputKind::Inline(chunk) => InputsModeDto::InputsData(hex::encode(&chunk.data)),
        DomainInputKind::StreamUri(uri) => InputsModeDto::InputsPath(uri.clone()),
    }
}

fn coord_err_to_gateway(e: zisk_distributed_coordinator::CoordinatorError) -> GatewayError {
    use zisk_distributed_coordinator::CoordinatorError;
    match e {
        CoordinatorError::InsufficientCapacity => {
            GatewayError::ClusterUnavailable { reason: "no workers connected" }
        }
        CoordinatorError::NotFoundOrInaccessible => {
            GatewayError::Internal("resource not found".into())
        }
        CoordinatorError::InvalidArgument(msg) | CoordinatorError::InvalidRequest(msg) => {
            GatewayError::InvalidJobState { reason: msg }
        }
        CoordinatorError::WorkerError(msg) | CoordinatorError::Internal(msg) => {
            GatewayError::Internal(msg)
        }
    }
}

fn is_terminal(event: &CoordinatorJobEvent) -> bool {
    matches!(
        event,
        CoordinatorJobEvent::Completed(_)
            | CoordinatorJobEvent::Failed(_)
            | CoordinatorJobEvent::Cancelled
    )
}

// ── BackendService impl ──────────────────────────────────────────────────────

#[async_trait]
impl BackendService for EmbeddedCoordinatorBackend {
    async fn register_guest_program(&self, elf: Vec<u8>) -> GatewayResult<String> {
        self.coordinator
            .register_guest_program(elf)
            .map_err(|e| internal(format!("register_guest_program: {e}")))
    }

    async fn submit_job(&self, kind: DomainJobKind) -> GatewayResult<Uuid> {
        match kind {
            DomainJobKind::Setup(r) => {
                let job_id_internal = self
                    .coordinator
                    .setup_program(&r.hash_id)
                    .await
                    .map_err(coord_err_to_gateway)?;
                let job_id = Uuid::parse_str(&job_id_internal.as_string())
                    .map_err(|e| internal(format!("invalid job_id: {e}")))?;
                Ok(job_id)
            }
            DomainJobKind::Prove(r) => {
                println!("** Submitting Prove job to coordinator with hash_id {}", r.hash_id);
                let hash_id = r.hash_id.clone();
                let response = self
                    .coordinator
                    .launch_proof(LaunchProofRequestDto {
                        data_id: DataId::new(),
                        compute_capacity: 10,
                        minimal_compute_capacity: 10,
                        inputs_mode: domain_input_to_dto(&r.input),
                        hints_mode: HintsModeDto::HintsNone,
                        simulated_node: None,
                        metadata: Default::default(),
                        execution_only: false,
                    })
                    .await
                    .map_err(coord_err_to_gateway)?;
                println!("** Coordinator responded with job_id {}", response.job_id.as_string());
                let job_id = Uuid::parse_str(&response.job_id.as_string())
                    .map_err(|e| internal(format!("invalid job_id: {e}")))?;
                self.job_hash.write().await.insert(job_id.to_string(), hash_id);
                Ok(job_id)
            }
            DomainJobKind::Execute(r) => {
                let hash_id = r.hash_id.clone();
                let response = self
                    .coordinator
                    .launch_proof(LaunchProofRequestDto {
                        data_id: DataId::new(),
                        compute_capacity: 10,
                        minimal_compute_capacity: 10,
                        inputs_mode: domain_input_to_dto(&r.input),
                        hints_mode: HintsModeDto::HintsNone,
                        simulated_node: None,
                        metadata: Default::default(),
                        execution_only: true,
                    })
                    .await
                    .map_err(coord_err_to_gateway)?;
                let job_id = Uuid::parse_str(&response.job_id.as_string())
                    .map_err(|e| internal(format!("invalid job_id: {e}")))?;
                self.job_hash.write().await.insert(job_id.to_string(), hash_id);
                Ok(job_id)
            }
            DomainJobKind::Wrap(_) => {
                Err(internal("Wrap jobs are not yet supported by the embedded coordinator backend"))
            }
        }
    }

    async fn wait_job_result(
        &self,
        job_id: Uuid,
        timeout_dur: Duration,
    ) -> GatewayResult<WaitResult> {
        let job_id_internal = zisk_distributed_common::JobId::from(job_id.to_string());
        let mut rx = self
            .coordinator
            .subscribe_job_events(&job_id_internal)
            .await
            .ok_or_else(|| internal(format!("job {} not found", job_id)))?;

        let result = timeout(timeout_dur, async {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        if is_terminal(&event) {
                            return Some(event);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("wait_job_result lagged {} events for job {}", n, job_id);
                    }
                }
            }
        })
        .await;

        let job_id_str = job_id.to_string();
        let hash_id = self.job_hash.read().await.get(&job_id_str).cloned().unwrap_or_default();

        let (job_status, kind_result) = match result {
            Ok(Some(CoordinatorJobEvent::Completed(r))) => {
                let kr = coord_result_to_domain(r, &hash_id);
                (DomainJobStatus::Completed, Some(kr))
            }
            Ok(Some(CoordinatorJobEvent::Failed(reason))) => {
                (DomainJobStatus::Failed(DomainJobFailure::Execution { reason }), None)
            }
            Ok(Some(CoordinatorJobEvent::Cancelled)) => (DomainJobStatus::Cancelled, None),
            _ => (DomainJobStatus::Running(None), None),
        };

        if job_status.is_terminal() {
            self.job_hash.write().await.remove(&job_id_str);
        }

        Ok(WaitResult { job_id, job_status, result: kind_result })
    }

    async fn watch_job(&self, job_id: Uuid) -> GatewayResult<JobEventStream> {
        let job_id_internal = zisk_distributed_common::JobId::from(job_id.to_string());
        let rx = self
            .coordinator
            .subscribe_job_events(&job_id_internal)
            .await
            .ok_or_else(|| internal(format!("job {} not found", job_id)))?;

        let job_id_str = job_id.to_string();
        let job_hash = self.job_hash.clone();

        let output = stream! {
            let mut rx = rx;
            let hash_id = job_hash.read().await.get(&job_id_str).cloned().unwrap_or_default();
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        let terminal = is_terminal(&event);
                        if let Some(domain) = coord_event_to_domain(event, job_id, &hash_id) {
                            yield Ok(domain);
                        }
                        if terminal {
                            job_hash.write().await.remove(&job_id_str);
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("watch_job lagged {} events for job {}", n, job_id_str);
                    }
                }
            }
        };

        Ok(Box::pin(output))
    }

    async fn push_job_input(&self, _job_id: Uuid, _chunks: InputChunkStream) -> GatewayResult<()> {
        Err(internal("push_job_input is not yet supported by the embedded coordinator backend"))
    }

    async fn cancel_job(&self, job_id: Uuid) -> GatewayResult<bool> {
        let job_id_internal = zisk_distributed_common::JobId::from(job_id.to_string());
        let cancelled = self
            .coordinator
            .cancel_job(&job_id_internal)
            .await
            .map_err(|e| internal(format!("cancel_job: {e}")))?;
        if cancelled {
            self.job_hash.write().await.remove(&job_id.to_string());
        }
        Ok(cancelled)
    }
}
