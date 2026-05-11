//! Coordinator backend — calls `Arc<Coordinator>` directly with no gRPC hop.
//!
//! The coordinator runs in the same process as the coordinator server. Workers still connect
//! over gRPC to the coordinator's worker-facing port.

use std::{collections::HashMap, sync::Arc, time::Duration};

use async_stream::stream;
use async_trait::async_trait;
use chrono::Utc;
use tokio::{sync::RwLock, time::timeout};
use tokio_stream::StreamExt as _;
use tracing::warn;
use uuid::Uuid;
use zisk_cluster_common::{JobPhase, JobState};
use zisk_coordinator::{
    job_events::{CoordinatorExecutionStats, CoordinatorJobEvent, CoordinatorJobResult},
    Coordinator,
};

use super::{
    BackendService, DomainExecutionStats, DomainInputKind, DomainJobEvent, DomainJobEventCancelled,
    DomainJobEventCompleted, DomainJobEventFailed, DomainJobEventProgress, DomainJobEventQueued,
    DomainJobEventStarted, DomainJobEventWaitingForInput, DomainJobFailure, DomainJobKind,
    DomainJobKindResponse, DomainJobPhase, DomainJobStatus, DomainProof, DomainProofKind,
    InputChunkStream, JobEventStream, SubmitJobResult, WaitResult,
};
use crate::errors::{internal, ApiError, ApiResult};
use zisk_cluster_common::{
    DataId, HintsModeDto, InputStreamDataDto, InputsModeDto, LaunchProofRequestDto,
    LaunchWrapRequestDto, ProofKind,
};

pub struct CoordinatorBackend {
    coordinator: Arc<Coordinator>,
    /// job_id (UUID string) → hash_id: needed to populate `DomainProof.hash_id`.
    /// Entries are removed once the job reaches a terminal state.
    job_hash: Arc<RwLock<HashMap<String, String>>>,
}

impl CoordinatorBackend {
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
        started_at: Some(Utc::now()),
        completed_at: Some(Utc::now()),
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
        CoordinatorJobResult::Setup { vk } => DomainJobKindResponse::Setup { vk },
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
        DomainInputKind::StreamUri(uri) => InputsModeDto::InputsStream(uri.clone()),
    }
}

fn domain_hints_to_dto(hints: &Option<DomainInputKind>) -> HintsModeDto {
    match hints {
        Some(DomainInputKind::Inline(chunk)) => HintsModeDto::HintsData(hex::encode(&chunk.data)),
        Some(DomainInputKind::StreamUri(uri)) => HintsModeDto::HintsStream(uri.clone()),
        None => HintsModeDto::HintsNone,
    }
}

fn coord_err_to_api(e: zisk_coordinator::CoordinatorError) -> ApiError {
    use zisk_coordinator::CoordinatorError;
    match e {
        CoordinatorError::InsufficientCapacity => {
            ApiError::ClusterUnavailable { reason: "no workers connected" }
        }
        CoordinatorError::WorkersSettingUp => {
            ApiError::ClusterUnavailable { reason: "workers are setting up; retry shortly" }
        }
        CoordinatorError::WorkersNotSetup => ApiError::ClusterUnavailable {
            reason: "workers connected but setup not done; call setup() first",
        },
        CoordinatorError::NotFoundOrInaccessible => ApiError::Internal("resource not found".into()),
        CoordinatorError::ProgramNotFound(hash_id) => ApiError::ProgramNotFound(hash_id),
        CoordinatorError::InvalidArgument(msg) | CoordinatorError::InvalidRequest(msg) => {
            ApiError::InvalidJobState { reason: msg }
        }
        CoordinatorError::WorkerError(msg) | CoordinatorError::Internal(msg) => {
            ApiError::Internal(msg)
        }
    }
}

/// Converts a coordinator event into the `(status, result)` pair used by
/// `wait_job_result`. Returns `(Running, None)` for non-terminal events.
fn wait_result_from_event(
    event: CoordinatorJobEvent,
    hash_id: &str,
) -> (DomainJobStatus, Option<DomainJobKindResponse>) {
    match event {
        CoordinatorJobEvent::Completed(r) => {
            (DomainJobStatus::Completed, Some(coord_result_to_domain(r, hash_id)))
        }
        CoordinatorJobEvent::Failed(reason) => {
            (DomainJobStatus::Failed(DomainJobFailure::Execution { reason }), None)
        }
        CoordinatorJobEvent::Cancelled => (DomainJobStatus::Cancelled, None),
        _ => (DomainJobStatus::Running(None), None),
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

/// Synthesize domain events the watcher missed before subscribing.
///
/// Queued and Started fire atomically at job creation, before `submit_job`
/// returns. Any client calling `watch_job` after submission has always missed
/// them. For jobs already past Contributions, the phase-transition Progress
/// events are also synthesized. The terminal event itself is NOT synthesized
/// here — callers should fetch the stashed real event via
/// `Coordinator::get_terminal_event` and append it separately.
fn catchup_events(state: &JobState, job_id: Uuid) -> Vec<DomainJobEvent> {
    let ts = Utc::now();
    let queued = DomainJobEvent::Queued(DomainJobEventQueued { job_id, timestamp: ts });
    let started = DomainJobEvent::Started(DomainJobEventStarted { job_id, timestamp: ts });
    let progress =
        |phase| DomainJobEvent::Progress(DomainJobEventProgress { job_id, phase, timestamp: ts });

    match state {
        JobState::Created => vec![queued],
        JobState::Running(phase) => {
            let mut events = vec![queued, started];
            // Synthesize Progress events for phases already past.
            // Progress(Prove) fires when Prove starts; Progress(Aggregate) when Aggregate starts.
            match phase {
                JobPhase::Prove => events.push(progress(DomainJobPhase::Prove)),
                JobPhase::Aggregate => {
                    events.push(progress(DomainJobPhase::Prove));
                    events.push(progress(DomainJobPhase::Aggregate));
                }
                _ => {}
            }
            events
        }
        // Terminal: emit only the queued/started pre-roll. The real terminal
        // event is fetched from the Coordinator's stash and appended by the caller.
        JobState::Completed | JobState::Failed | JobState::Cancelled => vec![queued, started],
    }
}

// ── BackendService impl ──────────────────────────────────────────────────────

#[async_trait]
impl BackendService for CoordinatorBackend {
    async fn register_guest_program(&self, elf: Vec<u8>) -> ApiResult<String> {
        self.coordinator
            .register_guest_program(elf)
            .map_err(|e| internal(format!("register_guest_program: {e}")))
    }

    async fn submit_job(&self, kind: DomainJobKind) -> ApiResult<SubmitJobResult> {
        match kind {
            DomainJobKind::Setup(r) => {
                let job_id_internal = self
                    .coordinator
                    .setup_program(&r.hash_id, r.program_name, r.with_hints)
                    .await
                    .map_err(coord_err_to_api)?;
                let job_id = Uuid::parse_str(&job_id_internal.as_string())
                    .map_err(|e| internal(format!("invalid job_id: {e}")))?;
                Ok(SubmitJobResult { job_id })
            }
            DomainJobKind::Prove(r) => {
                let hash_id = r.hash_id.clone();
                let proof_type = match r.proof_dest {
                    DomainProofKind::StarkMinimal => ProofKind::VadcopFinalMinimal,
                    DomainProofKind::Plonk => ProofKind::Plonk,
                    _ => ProofKind::VadcopFinal,
                };
                let hints_mode = domain_hints_to_dto(&r.hints);
                let response = self
                    .coordinator
                    .launch_proof(LaunchProofRequestDto {
                        data_id: DataId::new(),
                        hash_id: hash_id.clone(),
                        compute_capacity: None,
                        minimal_compute_capacity: None,
                        inputs_mode: domain_input_to_dto(&r.input),
                        hints_mode,
                        simulated_node: None,
                        metadata: Default::default(),
                        execution_only: false,
                        proof_type,
                    })
                    .await
                    .map_err(coord_err_to_api)?;
                let job_id = Uuid::parse_str(&response.job_id.as_string())
                    .map_err(|e| internal(format!("invalid job_id: {e}")))?;
                self.job_hash.write().await.insert(job_id.to_string(), hash_id);
                Ok(SubmitJobResult { job_id })
            }
            DomainJobKind::Execute(r) => {
                let hash_id = r.hash_id.clone();
                let hints_mode = domain_hints_to_dto(&r.hints);
                let response = self
                    .coordinator
                    .launch_proof(LaunchProofRequestDto {
                        data_id: DataId::new(),
                        hash_id: hash_id.clone(),
                        compute_capacity: None,
                        minimal_compute_capacity: None,
                        inputs_mode: domain_input_to_dto(&r.input),
                        hints_mode,
                        simulated_node: None,
                        metadata: Default::default(),
                        execution_only: true,
                        proof_type: ProofKind::VadcopFinal,
                    })
                    .await
                    .map_err(coord_err_to_api)?;
                let job_id = Uuid::parse_str(&response.job_id.as_string())
                    .map_err(|e| internal(format!("invalid job_id: {e}")))?;
                self.job_hash.write().await.insert(job_id.to_string(), hash_id);
                Ok(SubmitJobResult { job_id })
            }
            DomainJobKind::Wrap(r) => {
                let proof_dest = match r.proof_dest {
                    DomainProofKind::StarkMinimal => 1,
                    DomainProofKind::Plonk => 2,
                    DomainProofKind::Stark => 1,
                };
                let response = self
                    .coordinator
                    .launch_wrap(LaunchWrapRequestDto { proof_data: r.proof.data, proof_dest })
                    .await
                    .map_err(coord_err_to_api)?;
                let job_id = Uuid::parse_str(&response.job_id.as_string())
                    .map_err(|e| internal(format!("invalid job_id: {e}")))?;
                Ok(SubmitJobResult { job_id })
            }
        }
    }

    async fn wait_job_result(&self, job_id: Uuid, timeout_dur: Duration) -> ApiResult<WaitResult> {
        let job_id_internal = zisk_cluster_common::JobId::from(job_id.to_string());

        let job_id_str = job_id.to_string();
        let hash_id = self.job_hash.read().await.get(&job_id_str).cloned().unwrap_or_default();

        // Existence is sourced from the event channel, not the `jobs` map:
        // setup jobs live in `setup_pending` (not `jobs`) but DO have event
        // channels — sourcing from `jobs` would falsely 404 them.
        //
        // Subscribe BEFORE checking the stash so a terminal event that fires
        // between our two reads is captured by the receiver.
        let rx_opt = self.coordinator.subscribe_job_events(&job_id_internal).await;

        if let Some(terminal) = self.coordinator.get_terminal_event(&job_id_internal).await {
            let (status, kind_result) = wait_result_from_event(terminal, &hash_id);
            self.job_hash.write().await.remove(&job_id_str);
            return Ok(WaitResult { job_id, job_status: status, result: kind_result });
        }

        // Neither stash nor live channel → job does not exist (or already evicted).
        let mut rx = rx_opt.ok_or(ApiError::JobNotFound(job_id))?;

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

        let (job_status, kind_result) = match result {
            Ok(Some(event)) => wait_result_from_event(event, &hash_id),
            _ => (DomainJobStatus::Running(None), None),
        };

        if job_status.is_terminal() {
            self.job_hash.write().await.remove(&job_id_str);
        }

        Ok(WaitResult { job_id, job_status, result: kind_result })
    }

    async fn watch_job(&self, job_id: Uuid) -> ApiResult<JobEventStream> {
        let job_id_internal = zisk_cluster_common::JobId::from(job_id.to_string());

        // Existence is sourced from the event channel, not the `jobs` map:
        // setup jobs live in `setup_pending` (not `jobs`) but DO have event
        // channels — sourcing from `jobs` would falsely 404 them.
        //
        // Subscribe BEFORE checking the stash so we don't miss a terminal event
        // that fires between our two reads.
        let rx_opt = self.coordinator.subscribe_job_events(&job_id_internal).await;
        let stashed_terminal = self.coordinator.get_terminal_event(&job_id_internal).await;

        // Both None → job is unknown (or already evicted by retention sweep).
        if rx_opt.is_none() && stashed_terminal.is_none() {
            return Err(ApiError::JobNotFound(job_id));
        }

        // Best-effort catchup from the Job snapshot. Setup jobs aren't in
        // `jobs` (they use `setup_pending`) — degrade to a Created-state
        // catchup (just Queued) in that case.
        let job_state = self
            .coordinator
            .jobs()
            .read()
            .await
            .get(&job_id_internal)
            .cloned();
        let state = match job_state {
            Some(arc) => arc.read().await.state.clone(),
            None => JobState::Created,
        };
        let catchup = catchup_events(&state, job_id);

        // If the job is already terminal we won't drain the receiver — drop it.
        let rx = if stashed_terminal.is_some() { None } else { rx_opt };

        let job_id_str = job_id.to_string();
        let job_hash = self.job_hash.clone();

        let output = stream! {
            let hash_id = job_hash.read().await.get(&job_id_str).cloned().unwrap_or_default();

            for event in catchup {
                yield Ok(event);
            }

            // Already terminal: emit the stashed terminal event and close.
            if let Some(event) = stashed_terminal {
                if let Some(domain) = coord_event_to_domain(event, job_id, &hash_id) {
                    yield Ok(domain);
                }
                job_hash.write().await.remove(&job_id_str);
                return;
            }

            let Some(mut rx) = rx else {
                return;
            };

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

    async fn push_job_input(&self, job_id: Uuid, mut chunks: InputChunkStream) -> ApiResult<()> {
        let job_id_internal = zisk_cluster_common::JobId::from(job_id.to_string());

        // Look up the job and grab the worker list.
        let workers = {
            let jobs = self.coordinator.jobs().read().await;
            let job_arc = jobs
                .get(&job_id_internal)
                .ok_or_else(|| ApiError::Internal(format!("job {} not found", job_id)))?;
            let job = job_arc.read().await;
            job.workers.clone()
        };

        if workers.is_empty() {
            return Err(internal(format!("job {} has no assigned workers", job_id)));
        }

        // Drain the input stream and forward each chunk to every worker.
        while let Some(chunk_result) = chunks.next().await {
            let chunk = chunk_result.map_err(|e| internal(format!("input stream error: {e}")))?;

            for worker_id in &workers {
                let msg = zisk_cluster_common::CoordinatorMessageDto::InputStreamData(
                    InputStreamDataDto {
                        job_id: job_id_internal.clone(),
                        payload: chunk.data.clone(),
                    },
                );
                self.coordinator.workers_pool().send_message(worker_id, msg).await.map_err(
                    |e| internal(format!("failed to send input to worker {}: {}", worker_id, e)),
                )?;
            }
        }

        Ok(())
    }

    async fn push_job_hints_input(
        &self,
        job_id: Uuid,
        mut chunks: InputChunkStream,
    ) -> ApiResult<()> {
        let job_id_internal = zisk_cluster_common::JobId::from(job_id.to_string());

        // Feed each chunk into the coordinator's per-job relay channel.
        // The channel feeds into PrecompileHintsRelay which parses the hint
        // format and dispatches StreamData messages to workers.
        while let Some(chunk_result) = futures::StreamExt::next(&mut chunks).await {
            let chunk: zisk_coordinator_api::dto::DomainInputChunk =
                chunk_result.map_err(|e| internal(format!("hints stream error: {e}")))?;
            if !chunk.data.is_empty() {
                self.coordinator
                    .push_hints_grpc_data(&job_id_internal, chunk.data)
                    .await
                    .map_err(|e| internal(format!("hints relay error: {e}")))?;
            }
        }
        // Signal EOF so the relay thread exits cleanly.
        self.coordinator.finish_hints_grpc_stream(&job_id_internal).await;

        Ok(())
    }

    async fn cancel_job(&self, job_id: Uuid) -> ApiResult<bool> {
        let job_id_internal = zisk_cluster_common::JobId::from(job_id.to_string());
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
