//! Job handlers and proto↔domain conversions.
//!
//! Each `handle_*` function is responsible for:
//! 1. Validating and converting the proto request to domain types.
//! 2. Calling the [`BackendService`].
//! 3. Converting the domain result back to a proto response.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

const WAIT_TIMEOUT_DEFAULT_SECS: u32 = 5;
const WAIT_TIMEOUT_MIN_SECS: u32 = 1;
const WAIT_TIMEOUT_MAX_SECS: u32 = 3600;

use futures::{Stream, StreamExt};
use prost_types::Timestamp;
use tonic::{Status, Streaming};
use tracing::instrument;
use uuid::Uuid;

use crate::backend::{
    BackendService, DomainExecuteRequest, DomainExecutionStats, DomainInputChunk, DomainInputKind,
    DomainJobEvent, DomainJobFailure, DomainJobKind, DomainJobKindResponse, DomainJobPhase,
    DomainJobStatus, DomainProof, DomainProofKind, DomainProveRequest, DomainSetupRequest,
    DomainWrapRequest,
};
use crate::errors::GatewayError;
use crate::proto::*;

// ── Stream type for WatchJob ──────────────────────────────────────────────────

pub type WatchJobStream = Pin<Box<dyn Stream<Item = Result<JobEvent, Status>> + Send + 'static>>;

// ── JobRequest ────────────────────────────────────────────────────────────────

#[instrument(skip(backend, req))]
pub async fn handle_job_request<B: BackendService>(
    backend: &Arc<B>,
    req: JobRequestMessage,
) -> Result<JobResponse, Status> {
    let kind = proto_job_kind_to_domain(req.job_kind)?;
    let job_id = backend.submit_job(kind).await.map_err(into_status)?;
    Ok(JobResponse { job_id: job_id.to_string() })
}

// ── WaitJobResult ─────────────────────────────────────────────────────────────

#[instrument(skip(backend, req), fields(job_id = %req.job_id))]
pub async fn handle_wait_job_result<B: BackendService>(
    backend: &Arc<B>,
    req: WaitJobResultRequest,
) -> Result<WaitJobResultResponse, Status> {
    let job_id = parse_uuid(&req.job_id)?;
    let timeout_secs = req
        .timeout_seconds
        .unwrap_or(WAIT_TIMEOUT_DEFAULT_SECS)
        .clamp(WAIT_TIMEOUT_MIN_SECS, WAIT_TIMEOUT_MAX_SECS);
    let timeout = Duration::from_secs(timeout_secs as u64);

    let wait = backend.wait_job_result(job_id, timeout).await.map_err(into_status)?;

    Ok(WaitJobResultResponse {
        job_id: wait.job_id.to_string(),
        job_status: Some(domain_job_status_to_proto(&wait.job_status)),
        result: wait.result.map(domain_job_kind_response_to_proto),
    })
}

// ── WatchJob ──────────────────────────────────────────────────────────────────

#[instrument(skip(backend, req), fields(job_id = %req.job_id))]
pub async fn handle_watch_job<B: BackendService>(
    backend: &Arc<B>,
    req: WatchJobRequest,
) -> Result<WatchJobStream, Status> {
    let job_id = parse_uuid(&req.job_id)?;
    let event_stream = backend.watch_job(job_id).await.map_err(into_status)?;

    let proto_stream =
        event_stream.map(|result| result.map(domain_job_event_to_proto).map_err(into_status));

    Ok(Box::pin(proto_stream))
}

// ── PushJobInput ──────────────────────────────────────────────────────────────

pub async fn handle_push_job_input<B: BackendService>(
    backend: &Arc<B>,
    mut stream: Streaming<PushJobInputRequest>,
) -> Result<(), Status> {
    // Peek at the first message to get the job_id
    let first = stream
        .next()
        .await
        .ok_or_else(|| Status::invalid_argument("PushJobInput stream must not be empty"))?
        .map_err(|e| Status::internal(format!("stream read error: {e}")))?;

    let job_id = parse_uuid(&first.job_id)?;

    // Build a domain chunk stream from the first message + the rest of the stream
    let first_chunk = proto_input_chunk_to_domain(
        first.chunk.ok_or_else(|| Status::invalid_argument("chunk must be set"))?,
    );

    let chunk_stream =
        futures::stream::once(async move { Ok(first_chunk) }).chain(stream.map(|result| {
            result.map_err(|e| GatewayError::Internal(format!("stream read error: {e}"))).and_then(
                |msg| {
                    msg.chunk
                        .ok_or_else(|| GatewayError::InvalidJobState {
                            reason: "PushJobInput message missing chunk field".into(),
                        })
                        .map(proto_input_chunk_to_domain)
                },
            )
        }));

    backend.push_job_input(job_id, Box::pin(chunk_stream)).await.map_err(into_status)
}

// ── CancelJob ─────────────────────────────────────────────────────────────────

#[instrument(skip(backend, req), fields(job_id = %req.job_id))]
pub async fn handle_cancel_job<B: BackendService>(
    backend: &Arc<B>,
    req: CancelJobRequest,
) -> Result<CancelJobResponse, Status> {
    let job_id = parse_uuid(&req.job_id)?;
    let cancelled = backend.cancel_job(job_id).await.map_err(into_status)?;
    Ok(CancelJobResponse { job_id: job_id.to_string(), cancelled })
}

// ── Proto → Domain conversions ────────────────────────────────────────────────

fn proto_job_kind_to_domain(kind: Option<JobKind>) -> Result<DomainJobKind, Status> {
    let kind = kind.ok_or_else(|| Status::invalid_argument("job_kind must be set"))?;
    let inner = kind.kind.ok_or_else(|| Status::invalid_argument("job_kind.kind must be set"))?;

    match inner {
        job_kind::Kind::Setup(r) => {
            Ok(DomainJobKind::Setup(DomainSetupRequest { hash_id: r.hash_id }))
        }
        job_kind::Kind::Prove(r) => {
            let input = proto_input_kind_to_domain(r.input)?;
            let proof_timeout = r.proof_timeout.map(ts_to_datetime);
            Ok(DomainJobKind::Prove(DomainProveRequest {
                hash_id: r.hash_id,
                input,
                proof_timeout,
                compute_constraints: None,
            }))
        }
        job_kind::Kind::Wrap(r) => {
            let proof = proto_proof_to_domain(
                r.proof.ok_or_else(|| Status::invalid_argument("wrap.proof must be set"))?,
            )?;
            let proof_dest = proto_proof_kind_to_domain(r.proof_dest);
            let wrap_timeout = r.wrap_timeout.map(ts_to_datetime);
            Ok(DomainJobKind::Wrap(DomainWrapRequest { proof, proof_dest, wrap_timeout }))
        }
        job_kind::Kind::Execute(r) => {
            let input = proto_input_kind_to_domain(r.input)?;
            let execute_timeout = r.execute_timeout.map(ts_to_datetime);
            Ok(DomainJobKind::Execute(DomainExecuteRequest {
                hash_id: r.hash_id,
                input,
                execute_timeout,
                compute_constraints: None,
            }))
        }
    }
}

fn proto_input_kind_to_domain(input: Option<InputKind>) -> Result<DomainInputKind, Status> {
    let input = input.ok_or_else(|| Status::invalid_argument("input must be set"))?;
    let kind = input.kind.ok_or_else(|| Status::invalid_argument("input.kind must be set"))?;
    match kind {
        input_kind::Kind::Inline(chunk) => {
            Ok(DomainInputKind::Inline(proto_input_chunk_to_domain(chunk)))
        }
        input_kind::Kind::StreamUri(uri) => Ok(DomainInputKind::StreamUri(uri)),
    }
}

fn proto_input_chunk_to_domain(chunk: InputChunk) -> DomainInputChunk {
    DomainInputChunk { data: chunk.data, is_last: chunk.is_last }
}

fn proto_proof_kind_to_domain(kind: i32) -> DomainProofKind {
    match ProofKind::try_from(kind).unwrap_or(ProofKind::Unspecified) {
        ProofKind::StarkMinimal => DomainProofKind::StarkMinimal,
        ProofKind::Plonk => DomainProofKind::Plonk,
        _ => DomainProofKind::Stark,
    }
}

fn proto_proof_to_domain(p: Proof) -> Result<DomainProof, Status> {
    Ok(DomainProof {
        proof_id: parse_uuid(&p.proof_id)?,
        hash_id: p.hash_id,
        verification_key: p.verification_key,
        proof_kind: proto_proof_kind_to_domain(p.proof_kind),
        data: p.data,
        public_inputs: p.public_inputs,
        started_at: p.started_at.map(ts_to_datetime).unwrap_or_else(chrono::Utc::now),
        completed_at: p.completed_at.map(ts_to_datetime).unwrap_or_else(chrono::Utc::now),
    })
}

// ── Domain → Proto conversions ────────────────────────────────────────────────

fn domain_job_status_to_proto(status: &DomainJobStatus) -> JobStatus {
    let s = match status {
        DomainJobStatus::Queued => job_status::Status::Queued(JobStatusQueued {}),
        DomainJobStatus::Running(phase) => job_status::Status::Running(JobStatusRunning {
            phase: phase.as_ref().map(|p| domain_job_phase_to_proto(p) as i32),
        }),
        DomainJobStatus::WaitingForInput => {
            job_status::Status::WaitingForInput(JobStatusWaitingForInput {})
        }
        DomainJobStatus::Completed => job_status::Status::Completed(JobStatusCompleted {}),
        DomainJobStatus::Failed(f) => job_status::Status::Failed(JobStatusFailed {
            failure: Some(domain_job_failure_to_proto(f)),
        }),
        DomainJobStatus::Cancelled => job_status::Status::Cancelled(JobStatusCancelled {}),
    };
    JobStatus { status: Some(s) }
}

fn domain_job_phase_to_proto(phase: &DomainJobPhase) -> JobPhase {
    match phase {
        DomainJobPhase::Contributions => JobPhase::Contributions,
        DomainJobPhase::Prove => JobPhase::Prove,
        DomainJobPhase::Aggregate => JobPhase::Aggregate,
    }
}

fn domain_job_failure_to_proto(failure: &DomainJobFailure) -> JobFailure {
    use job_failure::Kind;
    let kind = match failure {
        DomainJobFailure::Timeout { phase, limit } => Kind::Timeout(JobFailureTimeout {
            phase: phase.as_ref().map(|p| domain_job_phase_to_proto(p) as i32),
            limit: Some(prost_types::Duration { seconds: limit.as_secs() as i64, nanos: 0 }),
        }),
        DomainJobFailure::Input { reason } => {
            Kind::Input(JobFailureInput { reason: reason.clone() })
        }
        DomainJobFailure::Execution { reason } => {
            Kind::Execution(JobFailureExecution { reason: reason.clone() })
        }
        DomainJobFailure::Internal { trace_id } => {
            Kind::Internal(JobFailureInternal { trace_id: trace_id.clone() })
        }
        DomainJobFailure::Cancelled => Kind::Cancelled(JobFailureCancelled {}),
    };
    JobFailure { kind: Some(kind) }
}

fn domain_job_kind_response_to_proto(resp: DomainJobKindResponse) -> JobKindResponse {
    use job_kind_response::Kind;
    let kind = match resp {
        DomainJobKindResponse::Setup => Kind::Setup(SetupResponse {}),
        DomainJobKindResponse::Prove { proof, stats } => Kind::Prove(ProveResponse {
            proof: Some(domain_proof_to_proto(proof)),
            stats: Some(domain_stats_to_proto(stats)),
        }),
        DomainJobKindResponse::Wrap(proof) => {
            Kind::Wrap(WrapResponse { proof: Some(domain_proof_to_proto(proof)) })
        }
        DomainJobKindResponse::Execute { stats, public_outputs } => {
            Kind::Execute(ExecuteResponse {
                stats: Some(domain_stats_to_proto(stats)),
                public_outputs,
            })
        }
    };
    JobKindResponse { kind: Some(kind) }
}

fn domain_stats_to_proto(stats: DomainExecutionStats) -> ExecutionStats {
    ExecutionStats {
        steps: stats.steps,
        duration_nanos: stats.duration_nanos,
        cost_per_type: Some(CostPerType {
            main: stats.main_cost,
            opcode: stats.opcode_cost,
            memory: stats.memory_cost,
            precompile: stats.precompile_cost,
            tables: stats.tables_cost,
            other: stats.other_cost,
        }),
    }
}

fn domain_proof_to_proto(proof: DomainProof) -> Proof {
    Proof {
        proof_id: proof.proof_id.to_string(),
        hash_id: proof.hash_id,
        verification_key: proof.verification_key,
        proof_kind: domain_proof_kind_to_proto(&proof.proof_kind) as i32,
        data: proof.data,
        public_inputs: proof.public_inputs,
        started_at: Some(datetime_to_ts(proof.started_at)),
        completed_at: Some(datetime_to_ts(proof.completed_at)),
    }
}

fn domain_proof_kind_to_proto(kind: &DomainProofKind) -> ProofKind {
    match kind {
        DomainProofKind::Stark => ProofKind::Stark,
        DomainProofKind::StarkMinimal => ProofKind::StarkMinimal,
        DomainProofKind::Plonk => ProofKind::Plonk,
    }
}

fn domain_job_event_to_proto(event: DomainJobEvent) -> JobEvent {
    use job_event::Event;
    let inner = match event {
        DomainJobEvent::Queued(e) => Event::Queued(JobEventQueued {
            job_id: e.job_id.to_string(),
            timestamp: Some(datetime_to_ts(e.timestamp)),
        }),
        DomainJobEvent::Started(e) => Event::Started(JobEventStarted {
            job_id: e.job_id.to_string(),
            timestamp: Some(datetime_to_ts(e.timestamp)),
        }),
        DomainJobEvent::Progress(e) => Event::Progress(JobEventProgress {
            job_id: e.job_id.to_string(),
            phase: domain_job_phase_to_proto(&e.phase) as i32,
            timestamp: Some(datetime_to_ts(e.timestamp)),
        }),
        DomainJobEvent::WaitingForInput(e) => Event::WaitingForInput(JobEventWaitingForInput {
            job_id: e.job_id.to_string(),
            timestamp: Some(datetime_to_ts(e.timestamp)),
        }),
        DomainJobEvent::Completed(e) => Event::Completed(JobEventCompleted {
            job_id: e.job_id.to_string(),
            result: Some(domain_job_kind_response_to_proto(e.result)),
            timestamp: Some(datetime_to_ts(e.timestamp)),
        }),
        DomainJobEvent::Cancelled(e) => Event::Cancelled(JobEventCancelled {
            job_id: e.job_id.to_string(),
            timestamp: Some(datetime_to_ts(e.timestamp)),
        }),
        DomainJobEvent::Failed(e) => Event::Failed(JobEventFailed {
            job_id: e.job_id.to_string(),
            failure: Some(domain_job_failure_to_proto(&e.failure)),
            timestamp: Some(datetime_to_ts(e.timestamp)),
        }),
    };
    JobEvent { event: Some(inner) }
}

// ── Timestamp helpers ─────────────────────────────────────────────────────────

fn ts_to_datetime(ts: Timestamp) -> chrono::DateTime<chrono::Utc> {
    use chrono::TimeZone;
    chrono::Utc.timestamp_opt(ts.seconds, ts.nanos as u32).single().unwrap_or_else(chrono::Utc::now)
}

fn datetime_to_ts(dt: chrono::DateTime<chrono::Utc>) -> Timestamp {
    Timestamp { seconds: dt.timestamp(), nanos: dt.timestamp_subsec_nanos() as i32 }
}

// ── Misc helpers ──────────────────────────────────────────────────────────────

fn parse_uuid(s: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("invalid UUID: {s}")))
}

fn into_status(e: GatewayError) -> Status {
    e.into()
}
