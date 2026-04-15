//! Coordinator backend — connects the gateway to a real `zisk-coordinator`
//! over the internal `ZiskCoordinatorApi` gRPC service.

use std::{collections::HashMap, sync::Arc, time::Duration};

use async_stream::stream;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use tonic::transport::{Channel, Endpoint};
use uuid::Uuid;
use zisk_distributed_grpc_api::coordinator_api::{
    coord_input_kind, coord_job_event, coord_job_kind_result, coord_submit_job_request,
    zisk_coordinator_api_client, CoordCancelJobRequest, CoordComputeConstraints,
    CoordExecuteRequest, CoordInputChunk, CoordInputKind, CoordProveRequest,
    CoordRegisterGuestProgramRequest, CoordSetupProgramRequest, CoordSubmitJobRequest,
    CoordWaitJobResultRequest, CoordWatchJobRequest, CoordWrapRequest,
};
use zisk_distributed_grpc_api::coordinator_api::{CoordJobPhase, CoordJobStatus};

use super::{
    BackendService, DomainComputeConstraints, DomainExecutionStats, DomainInputKind,
    DomainJobEvent, DomainJobEventCancelled, DomainJobEventCompleted, DomainJobEventFailed,
    DomainJobEventProgress, DomainJobEventQueued, DomainJobEventStarted,
    DomainJobEventWaitingForInput, DomainJobFailure, DomainJobKind, DomainJobKindResponse,
    DomainJobPhase, DomainJobStatus, DomainProof, DomainProofKind, InputChunkStream,
    JobEventStream, WaitResult,
};
use crate::errors::{internal, GatewayResult};

type CoordClient = zisk_coordinator_api_client::ZiskCoordinatorApiClient<Channel>;

pub struct CoordinatorBackend {
    /// Tonic clients are Clone (shared channel), so no locking needed.
    client: CoordClient,
    /// job_id (string) → hash_id: needed to populate DomainProof.hash_id.
    /// Entries are removed when the job reaches a terminal state.
    job_hash: Arc<RwLock<HashMap<String, String>>>,
}

impl CoordinatorBackend {
    /// Create a backend connected to the coordinator at `url`.
    ///
    /// The channel is established **lazily**: the gateway starts immediately even if the
    /// coordinator is temporarily unreachable. The first RPC will fail if the coordinator
    /// is still down, returning an error to the caller rather than preventing startup.
    pub fn new(
        url: String,
        connect_timeout: Duration,
        request_timeout: Duration,
    ) -> GatewayResult<Self> {
        let channel = Endpoint::from_shared(url)
            .map_err(|e| internal(format!("invalid coordinator url: {e}")))?
            .connect_timeout(connect_timeout)
            .timeout(request_timeout)
            .connect_lazy();

        Ok(Self {
            client: CoordClient::new(channel),
            job_hash: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn ts_to_datetime(ts: prost_types::Timestamp) -> DateTime<Utc> {
    DateTime::from_timestamp(ts.seconds, ts.nanos as u32).unwrap_or_else(Utc::now)
}

fn opt_ts_to_datetime(ts: Option<prost_types::Timestamp>) -> DateTime<Utc> {
    ts.map(ts_to_datetime).unwrap_or_else(Utc::now)
}

fn map_stats(
    s: Option<zisk_distributed_grpc_api::coordinator_api::CoordExecutionStats>,
) -> DomainExecutionStats {
    let Some(stats) = s else { return DomainExecutionStats::default() };
    let cost = stats.cost_per_type.unwrap_or_default();
    DomainExecutionStats {
        steps: stats.steps,
        duration_nanos: stats.duration_nanos,
        main_cost: cost.main,
        opcode_cost: cost.opcode,
        memory_cost: cost.memory,
        precompile_cost: cost.precompile,
        tables_cost: cost.tables,
        other_cost: cost.other,
    }
}

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

fn map_kind_result(
    r: zisk_distributed_grpc_api::coordinator_api::CoordJobKindResult,
    hash_id: String,
) -> Option<DomainJobKindResponse> {
    match r.result? {
        coord_job_kind_result::Result::Prove(p) => Some(DomainJobKindResponse::Prove {
            proof: make_proof(hash_id, p.proof_data),
            stats: map_stats(p.stats),
        }),
        coord_job_kind_result::Result::Execute(e) => Some(DomainJobKindResponse::Execute {
            stats: map_stats(e.stats),
            public_outputs: e.public_outputs,
        }),
        coord_job_kind_result::Result::Setup(_) => Some(DomainJobKindResponse::Setup),
        coord_job_kind_result::Result::Wrap(w) => {
            Some(DomainJobKindResponse::Wrap(make_proof(hash_id, w.proof_data)))
        }
    }
}

fn map_wait_status(status_i32: i32) -> DomainJobStatus {
    match CoordJobStatus::try_from(status_i32).unwrap_or(CoordJobStatus::Running) {
        CoordJobStatus::Completed => DomainJobStatus::Completed,
        CoordJobStatus::Failed => DomainJobStatus::Failed(DomainJobFailure::Execution {
            reason: "job failed".to_string(),
        }),
        CoordJobStatus::Cancelled => DomainJobStatus::Cancelled,
        CoordJobStatus::WaitingForInput => DomainJobStatus::WaitingForInput,
        CoordJobStatus::Queued => DomainJobStatus::Queued,
        _ => DomainJobStatus::Running(None),
    }
}

fn map_phase(phase_i32: i32) -> DomainJobPhase {
    match CoordJobPhase::try_from(phase_i32).unwrap_or(CoordJobPhase::Contributions) {
        CoordJobPhase::Prove => DomainJobPhase::Prove,
        CoordJobPhase::Aggregate => DomainJobPhase::Aggregate,
        _ => DomainJobPhase::Contributions,
    }
}

fn coord_event_to_domain(
    event: zisk_distributed_grpc_api::coordinator_api::CoordJobEvent,
    job_id: Uuid,
    hash_id: &str,
) -> Option<DomainJobEvent> {
    match event.event? {
        coord_job_event::Event::Queued(e) => Some(DomainJobEvent::Queued(DomainJobEventQueued {
            job_id,
            timestamp: opt_ts_to_datetime(e.timestamp),
        })),
        coord_job_event::Event::Started(e) => {
            Some(DomainJobEvent::Started(DomainJobEventStarted {
                job_id,
                timestamp: opt_ts_to_datetime(e.timestamp),
            }))
        }
        coord_job_event::Event::Progress(e) => {
            Some(DomainJobEvent::Progress(DomainJobEventProgress {
                job_id,
                phase: map_phase(e.phase),
                timestamp: opt_ts_to_datetime(e.timestamp),
            }))
        }
        coord_job_event::Event::WaitingForInput(e) => {
            Some(DomainJobEvent::WaitingForInput(DomainJobEventWaitingForInput {
                job_id,
                timestamp: opt_ts_to_datetime(e.timestamp),
            }))
        }
        coord_job_event::Event::Completed(e) => {
            let ts = opt_ts_to_datetime(e.timestamp);
            let result = e
                .result
                .and_then(|r| map_kind_result(r, hash_id.to_string()))
                .unwrap_or(DomainJobKindResponse::Setup);
            Some(DomainJobEvent::Completed(DomainJobEventCompleted {
                job_id,
                result,
                timestamp: ts,
            }))
        }
        coord_job_event::Event::Failed(e) => Some(DomainJobEvent::Failed(DomainJobEventFailed {
            job_id,
            failure: DomainJobFailure::Execution { reason: e.reason },
            timestamp: opt_ts_to_datetime(e.timestamp),
        })),
        coord_job_event::Event::Cancelled(e) => {
            Some(DomainJobEvent::Cancelled(DomainJobEventCancelled {
                job_id,
                timestamp: opt_ts_to_datetime(e.timestamp),
            }))
        }
    }
}

fn domain_input_to_coord(input: &DomainInputKind) -> CoordInputKind {
    match input {
        DomainInputKind::Inline(chunk) => CoordInputKind {
            source: Some(coord_input_kind::Source::Inline(CoordInputChunk {
                data: chunk.data.clone(),
                is_last: chunk.is_last,
            })),
        },
        DomainInputKind::StreamUri(uri) => {
            CoordInputKind { source: Some(coord_input_kind::Source::StreamUri(uri.clone())) }
        }
    }
}

impl From<DomainComputeConstraints> for CoordComputeConstraints {
    fn from(c: DomainComputeConstraints) -> Self {
        CoordComputeConstraints { requested: c.requested, minimum: c.minimum }
    }
}

// ── BackendService impl ──────────────────────────────────────────────────────

#[async_trait]
impl BackendService for CoordinatorBackend {
    async fn register_guest_program(&self, elf: Vec<u8>) -> GatewayResult<String> {
        let response = self
            .client
            .clone()
            .register_guest_program(CoordRegisterGuestProgramRequest { elf_bytes: elf })
            .await
            .map_err(|e| internal(format!("register_guest_program failed: {e}")))?;
        Ok(response.into_inner().hash_id)
    }

    async fn submit_job(&self, kind: DomainJobKind) -> GatewayResult<Uuid> {
        match kind {
            DomainJobKind::Setup(r) => {
                let response = self
                    .client
                    .clone()
                    .setup_program(CoordSetupProgramRequest { hash_id: r.hash_id })
                    .await
                    .map_err(|e| internal(format!("setup_program failed: {e}")))?;
                parse_job_id(response.into_inner().job_id)
            }
            DomainJobKind::Prove(r) => {
                let hash_id = r.hash_id.clone();
                let response = self
                    .client
                    .clone()
                    .submit_job(CoordSubmitJobRequest {
                        job_kind: Some(coord_submit_job_request::JobKind::Prove(
                            CoordProveRequest {
                                hash_id: r.hash_id,
                                input: Some(domain_input_to_coord(&r.input)),
                                proof_timeout: None,
                                constraints: r.compute_constraints.map(Into::into),
                            },
                        )),
                    })
                    .await
                    .map_err(|e| internal(format!("submit_job failed: {e}")))?;
                let job_id = parse_job_id(response.into_inner().job_id)?;
                self.job_hash.write().await.insert(job_id.to_string(), hash_id);
                Ok(job_id)
            }
            DomainJobKind::Execute(r) => {
                let hash_id = r.hash_id.clone();
                let response = self
                    .client
                    .clone()
                    .submit_job(CoordSubmitJobRequest {
                        job_kind: Some(coord_submit_job_request::JobKind::Execute(
                            CoordExecuteRequest {
                                hash_id: r.hash_id,
                                input: Some(domain_input_to_coord(&r.input)),
                                execute_timeout: None,
                                constraints: r.compute_constraints.map(Into::into),
                            },
                        )),
                    })
                    .await
                    .map_err(|e| internal(format!("submit_job failed: {e}")))?;
                let job_id = parse_job_id(response.into_inner().job_id)?;
                self.job_hash.write().await.insert(job_id.to_string(), hash_id);
                Ok(job_id)
            }
            DomainJobKind::Wrap(r) => {
                let proof_dest = match r.proof_dest {
                    DomainProofKind::StarkMinimal => 1,
                    DomainProofKind::Plonk => 2,
                    DomainProofKind::Stark => 1,
                };
                let response = self
                    .client
                    .clone()
                    .submit_job(CoordSubmitJobRequest {
                        job_kind: Some(coord_submit_job_request::JobKind::Wrap(CoordWrapRequest {
                            proof_data: r.proof.data,
                            proof_dest,
                            wrap_timeout: None,
                        })),
                    })
                    .await
                    .map_err(|e| internal(format!("submit_job wrap failed: {e}")))?;
                let job_id = parse_job_id(response.into_inner().job_id)?;
                Ok(job_id)
            }
        }
    }

    async fn wait_job_result(&self, job_id: Uuid, timeout: Duration) -> GatewayResult<WaitResult> {
        let job_id_str = job_id.to_string();
        let timeout_secs = timeout.as_secs().max(1) as u32;

        let response = self
            .client
            .clone()
            .wait_job_result(CoordWaitJobResultRequest {
                job_id: job_id_str.clone(),
                timeout_seconds: Some(timeout_secs),
            })
            .await
            .map_err(|e| internal(format!("wait_job_result failed: {e}")))?
            .into_inner();

        let job_status = map_wait_status(response.job_status);

        let result = if matches!(job_status, DomainJobStatus::Completed) {
            let hash_id = self.job_hash.read().await.get(&job_id_str).cloned().unwrap_or_default();
            response.result.and_then(|r| map_kind_result(r, hash_id))
        } else {
            None
        };

        // Clean up hash_id mapping once the job is terminal
        if job_status.is_terminal() {
            self.job_hash.write().await.remove(&job_id_str);
        }

        Ok(WaitResult { job_id, job_status, result })
    }

    async fn watch_job(&self, job_id: Uuid) -> GatewayResult<JobEventStream> {
        let job_id_str = job_id.to_string();

        let mut stream = self
            .client
            .clone()
            .watch_job(CoordWatchJobRequest { job_id: job_id_str.clone() })
            .await
            .map_err(|e| internal(format!("watch_job failed: {e}")))?
            .into_inner();

        let job_hash = self.job_hash.clone();

        let output = stream! {
            let hash_id = job_hash.read().await.get(&job_id_str).cloned().unwrap_or_default();
            loop {
                match stream.message().await {
                    Ok(Some(event)) => {
                        let is_terminal = matches!(
                            event.event,
                            Some(zisk_distributed_grpc_api::coordinator_api::coord_job_event::Event::Completed(_))
                            | Some(zisk_distributed_grpc_api::coordinator_api::coord_job_event::Event::Failed(_))
                            | Some(zisk_distributed_grpc_api::coordinator_api::coord_job_event::Event::Cancelled(_))
                        );
                        if let Some(domain_event) = coord_event_to_domain(event, job_id, &hash_id) {
                            yield Ok(domain_event);
                        }
                        if is_terminal {
                            job_hash.write().await.remove(&job_id_str);
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        yield Err(internal(format!("watch_job stream error: {e}")));
                        break;
                    }
                }
            }
        };

        Ok(Box::pin(output))
    }

    async fn push_job_input(&self, _job_id: Uuid, _chunks: InputChunkStream) -> GatewayResult<()> {
        Err(internal("push_job_input is not yet supported by the coordinator backend"))
    }

    async fn cancel_job(&self, job_id: Uuid) -> GatewayResult<bool> {
        let job_id_str = job_id.to_string();
        let response = self
            .client
            .clone()
            .cancel_job(CoordCancelJobRequest { job_id: job_id_str.clone() })
            .await
            .map_err(|e| internal(format!("cancel_job failed: {e}")))?;
        self.job_hash.write().await.remove(&job_id_str);
        Ok(response.into_inner().cancelled)
    }
}

fn parse_job_id(s: String) -> GatewayResult<Uuid> {
    Uuid::parse_str(&s).map_err(|e| internal(format!("invalid job_id from coordinator: {e}")))
}
