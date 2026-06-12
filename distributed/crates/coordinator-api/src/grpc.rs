//! gRPC transport layer.
//!
//! Contains the tonic-generated proto types and proto ↔ domain conversions.
//! The blocking client wrapper and job lifecycle types live in the SDK
//! (`sdk/src/remote/client.rs` and `sdk/src/remote/job.rs`).

pub mod proto {
    #![allow(clippy::large_enum_variant)]
    tonic::include_proto!("zisk.coordinator.v1");
}

pub use proto::zisk_coordinator_api_client::ZiskCoordinatorApiClient;
pub use proto::zisk_coordinator_api_server::{ZiskCoordinatorApi, ZiskCoordinatorApiServer};

use crate::dto::{
    DomainAggregateProofsRequest, DomainAggregationProgramSpec, DomainAirInstanceCount,
    DomainAsmExecution, DomainExecuteRequest, DomainExecutionStats, DomainExecutorTime,
    DomainInputChunk, DomainInputKind, DomainJobEvent, DomainJobEventCancelled,
    DomainJobEventCompleted, DomainJobEventFailed, DomainJobEventProgress, DomainJobEventQueued,
    DomainJobEventStarted, DomainJobEventWaitingForInput, DomainJobFailure, DomainJobKind,
    DomainJobKindResponse, DomainJobPhase, DomainJobStatus, DomainNormalizeGroup, DomainProof,
    DomainProofKind, DomainProveRequest, DomainSetupAggregationProgramRequest, DomainSetupRequest,
    DomainWrapRequest, RegisterAggregationProgramRequestDto, RegisterAggregationProgramResponseDto,
    RegisterGuestProgramRequestDto, RegisterGuestProgramResponseDto,
};
use anyhow::Result;
use prost_types::Timestamp;
use proto::*;
use uuid::Uuid;

impl From<RegisterGuestProgramRequestDto> for RegisterGuestProgramRequest {
    fn from(dto: RegisterGuestProgramRequestDto) -> Self {
        Self { zisk_elf: dto.zisk_elf }
    }
}

impl tonic::IntoRequest<RegisterGuestProgramRequest> for RegisterGuestProgramRequestDto {
    fn into_request(self) -> tonic::Request<RegisterGuestProgramRequest> {
        tonic::Request::new(self.into())
    }
}

impl From<RegisterGuestProgramRequest> for RegisterGuestProgramRequestDto {
    fn from(req: RegisterGuestProgramRequest) -> Self {
        Self { zisk_elf: req.zisk_elf }
    }
}

impl From<RegisterGuestProgramResponseDto> for RegisterGuestProgramResponse {
    fn from(dto: RegisterGuestProgramResponseDto) -> Self {
        Self { hash_id: dto.hash_id }
    }
}

impl From<RegisterGuestProgramResponse> for RegisterGuestProgramResponseDto {
    fn from(resp: RegisterGuestProgramResponse) -> Self {
        Self { hash_id: resp.hash_id }
    }
}

impl From<DomainAggregationProgramSpec> for AggregationProgramSpec {
    fn from(s: DomainAggregationProgramSpec) -> Self {
        Self {
            program_vks: s
                .program_vks
                .into_iter()
                .map(|[l0, l1, l2, l3]| ProgramVk { l0, l1, l2, l3 })
                .collect(),
            normalize_groups: s
                .normalize_groups
                .into_iter()
                .map(|g| NormalizeGroup {
                    member_indices: g.member_indices,
                    body: g.body,
                    n_free_inputs: g.n_free_inputs,
                })
                .collect(),
            aggregate_publics_body: s.aggregate_publics_body,
        }
    }
}

impl From<AggregationProgramSpec> for DomainAggregationProgramSpec {
    fn from(s: AggregationProgramSpec) -> Self {
        Self {
            program_vks: s.program_vks.into_iter().map(|vk| [vk.l0, vk.l1, vk.l2, vk.l3]).collect(),
            normalize_groups: s
                .normalize_groups
                .into_iter()
                .map(|g| DomainNormalizeGroup {
                    member_indices: g.member_indices,
                    body: g.body,
                    n_free_inputs: g.n_free_inputs,
                })
                .collect(),
            aggregate_publics_body: s.aggregate_publics_body,
        }
    }
}

impl From<RegisterAggregationProgramRequestDto> for RegisterAggregationProgramRequest {
    fn from(dto: RegisterAggregationProgramRequestDto) -> Self {
        Self { recurser_id: dto.recurser_id, spec: Some(dto.spec.into()) }
    }
}

impl tonic::IntoRequest<RegisterAggregationProgramRequest>
    for RegisterAggregationProgramRequestDto
{
    fn into_request(self) -> tonic::Request<RegisterAggregationProgramRequest> {
        tonic::Request::new(self.into())
    }
}

impl TryFrom<RegisterAggregationProgramRequest> for RegisterAggregationProgramRequestDto {
    type Error = String;

    fn try_from(req: RegisterAggregationProgramRequest) -> std::result::Result<Self, Self::Error> {
        let spec = req.spec.ok_or_else(|| "register_recurser.spec must be set".to_string())?;
        Ok(Self { recurser_id: req.recurser_id, spec: spec.into() })
    }
}

impl From<RegisterAggregationProgramResponseDto> for RegisterAggregationProgramResponse {
    fn from(dto: RegisterAggregationProgramResponseDto) -> Self {
        Self { recurser_id: dto.recurser_id }
    }
}

impl From<RegisterAggregationProgramResponse> for RegisterAggregationProgramResponseDto {
    fn from(resp: RegisterAggregationProgramResponse) -> Self {
        Self { recurser_id: resp.recurser_id }
    }
}

impl tonic::IntoRequest<JobRequestMessage> for DomainJobKind {
    fn into_request(self) -> tonic::Request<JobRequestMessage> {
        tonic::Request::new(JobRequestMessage { job_kind: Some(self.into()) })
    }
}

fn datetime_to_ts(dt: chrono::DateTime<chrono::Utc>) -> Timestamp {
    Timestamp { seconds: dt.timestamp(), nanos: dt.timestamp_subsec_nanos() as i32 }
}

fn ts_to_datetime(ts: Timestamp) -> Option<chrono::DateTime<chrono::Utc>> {
    use chrono::TimeZone;
    chrono::Utc.timestamp_opt(ts.seconds, ts.nanos as u32).single()
}

fn parse_uuid(s: &str) -> Result<Uuid> {
    Uuid::parse_str(s).map_err(|e| anyhow::anyhow!("invalid UUID '{}': {e}", s))
}

impl From<DomainProofKind> for ProofKind {
    fn from(kind: DomainProofKind) -> Self {
        match kind {
            DomainProofKind::Stark => ProofKind::Stark,
            DomainProofKind::StarkMinimal => ProofKind::StarkMinimal,
            DomainProofKind::Plonk => ProofKind::Plonk,
        }
    }
}

impl TryFrom<i32> for DomainProofKind {
    type Error = i32;

    fn try_from(value: i32) -> std::result::Result<Self, Self::Error> {
        match ProofKind::try_from(value).unwrap_or(ProofKind::Unspecified) {
            ProofKind::Stark => Ok(DomainProofKind::Stark),
            ProofKind::StarkMinimal => Ok(DomainProofKind::StarkMinimal),
            ProofKind::Plonk => Ok(DomainProofKind::Plonk),
            _ => Err(value),
        }
    }
}

impl From<DomainJobPhase> for JobPhase {
    fn from(phase: DomainJobPhase) -> Self {
        match phase {
            DomainJobPhase::Contributions => JobPhase::Contributions,
            DomainJobPhase::Prove => JobPhase::Prove,
            DomainJobPhase::Recurse => JobPhase::Recurse,
        }
    }
}

impl DomainInputKind {
    /// Maximum size for inline input payloads. Both the coordinator server and
    /// SDK client are configured with the same limit via `max_decoding/encoding_message_size`.
    const MAX_INLINE_BYTES: usize = 128 * 1024 * 1024;

    /// Wrap `data` as an inline input chunk, enforcing the gRPC message size limit.
    pub fn try_inline(data: Vec<u8>) -> anyhow::Result<Self> {
        if data.len() > Self::MAX_INLINE_BYTES {
            anyhow::bail!(
                "input is {} bytes which exceeds the {} byte inline limit; \
                 use a StreamUri input source for large payloads",
                data.len(),
                Self::MAX_INLINE_BYTES
            );
        }
        Ok(DomainInputKind::Inline(DomainInputChunk { data }))
    }
}

impl From<InputChunk> for DomainInputChunk {
    fn from(val: InputChunk) -> Self {
        DomainInputChunk { data: val.data }
    }
}

impl From<DomainInputChunk> for InputChunk {
    fn from(chunk: DomainInputChunk) -> Self {
        InputChunk { data: chunk.data }
    }
}

impl TryFrom<InputKind> for DomainInputKind {
    type Error = String;

    fn try_from(input: InputKind) -> std::result::Result<Self, Self::Error> {
        let kind = input.kind.ok_or_else(|| "input.kind must be set".to_string())?;
        match kind {
            input_kind::Kind::Inline(chunk) => Ok(DomainInputKind::Inline(chunk.into())),
            input_kind::Kind::StreamUri(uri) => Ok(DomainInputKind::StreamUri(uri)),
        }
    }
}

impl From<DomainInputKind> for InputKind {
    fn from(domain: DomainInputKind) -> Self {
        match domain {
            DomainInputKind::Inline(chunk) => {
                InputKind { kind: Some(input_kind::Kind::Inline(chunk.into())) }
            }
            DomainInputKind::StreamUri(uri) => {
                InputKind { kind: Some(input_kind::Kind::StreamUri(uri)) }
            }
        }
    }
}

impl From<DomainProof> for Proof {
    fn from(proof: DomainProof) -> Self {
        Proof {
            proof_id: proof.proof_id.to_string(),
            hash_id: proof.hash_id,
            verification_key: proof.verification_key,
            proof_kind: ProofKind::from(proof.proof_kind).into(),
            data: proof.data,
            public_inputs: proof.public_inputs,
            started_at: proof.started_at.map(datetime_to_ts),
            completed_at: proof.completed_at.map(datetime_to_ts),
        }
    }
}

impl TryFrom<Proof> for DomainProof {
    type Error = String;

    fn try_from(p: Proof) -> std::result::Result<Self, Self::Error> {
        Ok(DomainProof {
            proof_id: parse_uuid(&p.proof_id).map_err(|e| format!("invalid proof_id: {e}"))?,
            hash_id: p.hash_id,
            verification_key: p.verification_key,
            proof_kind: DomainProofKind::try_from(p.proof_kind)
                .map_err(|_| format!("invalid proof_kind {}", p.proof_kind))?,
            data: p.data,
            public_inputs: p.public_inputs,
            started_at: p.started_at.and_then(ts_to_datetime),
            completed_at: p.completed_at.and_then(ts_to_datetime),
        })
    }
}

impl TryFrom<JobKind> for DomainJobKind {
    type Error = String;

    fn try_from(kind: JobKind) -> std::result::Result<Self, Self::Error> {
        let inner = kind.kind.ok_or_else(|| "job_kind.kind must be set".to_string())?;

        match inner {
            job_kind::Kind::Setup(r) => Ok(DomainJobKind::Setup(DomainSetupRequest {
                hash_id: r.hash_id,
                program_name: r.program_name,
                with_hints: r.with_hints,
                emulator_only: r.emulator_only,
            })),
            job_kind::Kind::Prove(r) => {
                let input = r
                    .input
                    .ok_or_else(|| "input must be set".to_string())?
                    .try_into()
                    .map_err(|e: String| e)?;
                let proof_timeout = r.proof_timeout.and_then(ts_to_datetime);
                let proof_dest =
                    DomainProofKind::try_from(r.proof_dest).unwrap_or(DomainProofKind::Stark);
                let hints = r.hints.map(|h| h.try_into()).transpose().map_err(|e: String| e)?;
                Ok(DomainJobKind::Prove(DomainProveRequest {
                    hash_id: r.hash_id,
                    input,
                    hints,
                    proof_timeout,
                    proof_dest,
                }))
            }
            job_kind::Kind::Wrap(r) => {
                let proof = DomainProof::try_from(
                    r.proof.ok_or_else(|| "wrap.proof must be set".to_string())?,
                )
                .map_err(|e| format!("invalid wrap.proof: {e}"))?;
                let proof_dest = DomainProofKind::try_from(r.proof_dest)
                    .map_err(|_| "invalid proof_dest".to_string())?;
                let wrap_timeout = r.wrap_timeout.and_then(ts_to_datetime);
                Ok(DomainJobKind::Wrap(DomainWrapRequest { proof, proof_dest, wrap_timeout }))
            }
            job_kind::Kind::Execute(r) => {
                let input = r
                    .input
                    .ok_or_else(|| "input must be set".to_string())?
                    .try_into()
                    .map_err(|e: String| e)?;
                let execute_timeout = r.execute_timeout.and_then(ts_to_datetime);
                let hints = r.hints.map(|h| h.try_into()).transpose().map_err(|e: String| e)?;
                Ok(DomainJobKind::Execute(DomainExecuteRequest {
                    hash_id: r.hash_id,
                    input,
                    hints,
                    execute_timeout,
                }))
            }
            job_kind::Kind::SetupAggregationProgram(r) => {
                Ok(DomainJobKind::SetupAggregationProgram(DomainSetupAggregationProgramRequest {
                    recurser_id: r.recurser_id,
                }))
            }
            job_kind::Kind::AggregateProofs(r) => {
                let root_c = if r.root_c_recurser_agg.is_empty() {
                    None
                } else if r.root_c_recurser_agg.len() == 4 {
                    Some([
                        r.root_c_recurser_agg[0],
                        r.root_c_recurser_agg[1],
                        r.root_c_recurser_agg[2],
                        r.root_c_recurser_agg[3],
                    ])
                } else {
                    return Err(format!(
                        "aggregate_proofs.root_c_recurser_agg must be 0 or 4 limbs; got {}",
                        r.root_c_recurser_agg.len()
                    ));
                };
                Ok(DomainJobKind::AggregateProofs(DomainAggregateProofsRequest {
                    recurser_id: r.recurser_id,
                    proof_a: r.proof_a,
                    proof_b: r.proof_b,
                    free_inputs_a: r.free_inputs_a,
                    free_inputs_b: r.free_inputs_b,
                    root_c_recurser_agg: root_c,
                }))
            }
        }
    }
}

impl From<DomainJobKind> for JobKind {
    fn from(domain: DomainJobKind) -> Self {
        use job_kind::Kind;
        let kind = match domain {
            DomainJobKind::Setup(r) => Kind::Setup(SetupRequest {
                hash_id: r.hash_id,
                program_name: r.program_name,
                with_hints: r.with_hints,
                emulator_only: r.emulator_only,
            }),
            DomainJobKind::Prove(r) => Kind::Prove(ProveRequest {
                hash_id: r.hash_id,
                input: Some(InputKind::from(r.input)),
                proof_timeout: r.proof_timeout.map(datetime_to_ts),
                proof_dest: ProofKind::from(r.proof_dest).into(),
                hints: r.hints.map(InputKind::from),
            }),
            DomainJobKind::Wrap(r) => Kind::Wrap(WrapRequest {
                proof: Some(r.proof.into()),
                proof_dest: ProofKind::from(r.proof_dest).into(),
                wrap_timeout: r.wrap_timeout.map(datetime_to_ts),
            }),
            DomainJobKind::Execute(r) => Kind::Execute(ExecuteRequest {
                hash_id: r.hash_id,
                input: Some(InputKind::from(r.input)),
                execute_timeout: r.execute_timeout.map(datetime_to_ts),
                hints: r.hints.map(InputKind::from),
            }),
            DomainJobKind::SetupAggregationProgram(r) => {
                Kind::SetupAggregationProgram(SetupAggregationProgramRequest {
                    recurser_id: r.recurser_id,
                })
            }
            DomainJobKind::AggregateProofs(r) => Kind::AggregateProofs(AggregateProofsRequest {
                recurser_id: r.recurser_id,
                proof_a: r.proof_a,
                proof_b: r.proof_b,
                free_inputs_a: r.free_inputs_a,
                free_inputs_b: r.free_inputs_b,
                root_c_recurser_agg: r.root_c_recurser_agg.map(|l| l.to_vec()).unwrap_or_default(),
            }),
        };
        JobKind { kind: Some(kind) }
    }
}

impl From<DomainExecutionStats> for ExecutionStats {
    fn from(stats: DomainExecutionStats) -> Self {
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
            executor_time: Some(ExecutorTime {
                total_duration: stats.executor_time.total_duration,
                execution_duration: stats.executor_time.execution_duration,
                count_and_plan_duration: stats.executor_time.count_and_plan_duration,
                count_and_plan_mo_duration: stats.executor_time.count_and_plan_mo_duration,
                asm: stats.executor_time.asm.map(|a| AsmExecution { time: a.time, mhz: a.mhz }),
            }),
            plan: stats
                .plan
                .into_iter()
                .map(|p| AirInstanceCount {
                    airgroup_id: p.airgroup_id as u32,
                    air_id: p.air_id as u32,
                    count: p.count,
                })
                .collect(),
        }
    }
}

impl From<DomainJobKindResponse> for JobKindResponse {
    fn from(value: DomainJobKindResponse) -> Self {
        use job_kind_response::Kind;
        let kind = match value {
            DomainJobKindResponse::Setup { vk, hash_mode } => {
                Kind::Setup(SetupResponse { vk, hash_mode })
            }
            DomainJobKindResponse::Prove { proof, stats } => {
                Kind::Prove(ProveResponse { proof: Some(proof.into()), stats: Some(stats.into()) })
            }
            DomainJobKindResponse::Wrap(proof) => {
                Kind::Wrap(WrapResponse { proof: Some(proof.into()) })
            }
            DomainJobKindResponse::Execute { stats, public_outputs } => {
                Kind::Execute(ExecuteResponse { stats: Some(stats.into()), public_outputs })
            }
            DomainJobKindResponse::SetupAggregationProgram { vk, hash_mode } => {
                Kind::SetupAggregationProgram(SetupAggregationProgramResponse { vk, hash_mode })
            }
            DomainJobKindResponse::AggregateProofs(proof) => {
                Kind::AggregateProofs(AggregateProofsResponse { proof: Some(proof.into()) })
            }
        };
        JobKindResponse { kind: Some(kind) }
    }
}

impl From<&DomainJobStatus> for JobStatus {
    fn from(status: &DomainJobStatus) -> Self {
        let s = match status {
            DomainJobStatus::Queued => job_status::Status::Queued(JobStatusQueued {}),
            DomainJobStatus::Running(phase) => job_status::Status::Running(JobStatusRunning {
                phase: phase.as_ref().map(|p| JobPhase::from(p.clone()).into()),
            }),
            DomainJobStatus::WaitingForInput => {
                job_status::Status::WaitingForInput(JobStatusWaitingForInput {})
            }
            DomainJobStatus::Completed => job_status::Status::Completed(JobStatusCompleted {}),
            DomainJobStatus::Failed(f) => {
                job_status::Status::Failed(JobStatusFailed { failure: Some(f.into()) })
            }
            DomainJobStatus::Cancelled => job_status::Status::Cancelled(JobStatusCancelled {}),
        };
        JobStatus { status: Some(s) }
    }
}

impl From<&DomainJobFailure> for JobFailure {
    fn from(failure: &DomainJobFailure) -> Self {
        use job_failure::Kind;
        let kind = match failure {
            DomainJobFailure::Timeout { phase, limit } => Kind::Timeout(JobFailureTimeout {
                phase: phase.as_ref().map(|p| JobPhase::from(p.clone()).into()),
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
}

impl From<DomainJobFailure> for JobFailure {
    fn from(failure: DomainJobFailure) -> Self {
        use job_failure::Kind;
        let kind = match failure {
            DomainJobFailure::Timeout { phase, limit } => Kind::Timeout(JobFailureTimeout {
                phase: phase.map(|p| JobPhase::from(p).into()),
                limit: Some(prost_types::Duration { seconds: limit.as_secs() as i64, nanos: 0 }),
            }),
            DomainJobFailure::Input { reason } => Kind::Input(JobFailureInput { reason }),
            DomainJobFailure::Execution { reason } => {
                Kind::Execution(JobFailureExecution { reason })
            }
            DomainJobFailure::Internal { trace_id } => {
                Kind::Internal(JobFailureInternal { trace_id })
            }
            DomainJobFailure::Cancelled => Kind::Cancelled(JobFailureCancelled {}),
        };
        JobFailure { kind: Some(kind) }
    }
}

impl From<DomainJobEvent> for JobEvent {
    fn from(event: DomainJobEvent) -> Self {
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
                phase: JobPhase::from(e.phase).into(),
                timestamp: Some(datetime_to_ts(e.timestamp)),
            }),
            DomainJobEvent::WaitingForInput(e) => Event::WaitingForInput(JobEventWaitingForInput {
                job_id: e.job_id.to_string(),
                timestamp: Some(datetime_to_ts(e.timestamp)),
            }),
            DomainJobEvent::Completed(e) => Event::Completed(JobEventCompleted {
                job_id: e.job_id.to_string(),
                result: Some(e.result.into()),
                timestamp: Some(datetime_to_ts(e.timestamp)),
            }),
            DomainJobEvent::Cancelled(e) => Event::Cancelled(JobEventCancelled {
                job_id: e.job_id.to_string(),
                timestamp: Some(datetime_to_ts(e.timestamp)),
            }),
            DomainJobEvent::Failed(e) => Event::Failed(JobEventFailed {
                job_id: e.job_id.to_string(),
                failure: Some(e.failure.into()),
                timestamp: Some(datetime_to_ts(e.timestamp)),
            }),
        };
        JobEvent { event: Some(inner) }
    }
}

impl From<ExecutionStats> for DomainExecutionStats {
    fn from(stats: ExecutionStats) -> Self {
        let cost = stats.cost_per_type.unwrap_or_default();
        let et = stats.executor_time.unwrap_or_default();
        DomainExecutionStats {
            steps: stats.steps,
            duration_nanos: stats.duration_nanos,
            main_cost: cost.main,
            opcode_cost: cost.opcode,
            memory_cost: cost.memory,
            precompile_cost: cost.precompile,
            tables_cost: cost.tables,
            other_cost: cost.other,
            executor_time: DomainExecutorTime {
                total_duration: et.total_duration,
                execution_duration: et.execution_duration,
                count_and_plan_duration: et.count_and_plan_duration,
                count_and_plan_mo_duration: et.count_and_plan_mo_duration,
                asm: et.asm.map(|a| DomainAsmExecution { time: a.time, mhz: a.mhz }),
            },
            plan: stats
                .plan
                .into_iter()
                .map(|p| DomainAirInstanceCount {
                    airgroup_id: p.airgroup_id as usize,
                    air_id: p.air_id as usize,
                    count: p.count,
                })
                .collect(),
        }
    }
}

impl TryFrom<i32> for DomainJobPhase {
    type Error = String;

    fn try_from(value: i32) -> std::result::Result<Self, Self::Error> {
        match JobPhase::try_from(value) {
            Ok(JobPhase::Contributions) => Ok(DomainJobPhase::Contributions),
            Ok(JobPhase::Prove) => Ok(DomainJobPhase::Prove),
            Ok(JobPhase::Recurse) => Ok(DomainJobPhase::Recurse),
            _ => Err(format!("invalid job phase: {value}")),
        }
    }
}

impl TryFrom<JobPhase> for DomainJobPhase {
    type Error = String;

    fn try_from(phase: JobPhase) -> std::result::Result<Self, Self::Error> {
        match phase {
            JobPhase::Contributions => Ok(DomainJobPhase::Contributions),
            JobPhase::Prove => Ok(DomainJobPhase::Prove),
            JobPhase::Recurse => Ok(DomainJobPhase::Recurse),
            _ => Err(format!("invalid job phase: {:?}", phase)),
        }
    }
}

impl TryFrom<JobFailure> for DomainJobFailure {
    type Error = String;

    fn try_from(failure: JobFailure) -> std::result::Result<Self, Self::Error> {
        use job_failure::Kind;
        match failure.kind.ok_or_else(|| "job_failure.kind must be set".to_string())? {
            Kind::Timeout(t) => {
                let phase = t.phase.map(DomainJobPhase::try_from).transpose()?;
                let limit = t
                    .limit
                    .map(|d| std::time::Duration::new(d.seconds as u64, d.nanos as u32))
                    .unwrap_or_default();
                Ok(DomainJobFailure::Timeout { phase, limit })
            }
            Kind::Input(i) => Ok(DomainJobFailure::Input { reason: i.reason }),
            Kind::Execution(e) => Ok(DomainJobFailure::Execution { reason: e.reason }),
            Kind::Internal(i) => Ok(DomainJobFailure::Internal { trace_id: i.trace_id }),
            Kind::Cancelled(_) => Ok(DomainJobFailure::Cancelled),
        }
    }
}

impl TryFrom<JobStatus> for DomainJobStatus {
    type Error = String;

    fn try_from(status: JobStatus) -> std::result::Result<Self, Self::Error> {
        use job_status::Status;
        match status.status.ok_or_else(|| "job_status.status must be set".to_string())? {
            Status::Queued(_) => Ok(DomainJobStatus::Queued),
            Status::Running(r) => {
                let phase = r.phase.map(DomainJobPhase::try_from).transpose()?;
                Ok(DomainJobStatus::Running(phase))
            }
            Status::WaitingForInput(_) => Ok(DomainJobStatus::WaitingForInput),
            Status::Completed(_) => Ok(DomainJobStatus::Completed),
            Status::Failed(f) => {
                let failure = f
                    .failure
                    .ok_or_else(|| "failed status must have failure".to_string())?
                    .try_into()?;
                Ok(DomainJobStatus::Failed(failure))
            }
            Status::Cancelled(_) => Ok(DomainJobStatus::Cancelled),
        }
    }
}

impl TryFrom<JobKindResponse> for DomainJobKindResponse {
    type Error = String;

    fn try_from(resp: JobKindResponse) -> std::result::Result<Self, Self::Error> {
        use job_kind_response::Kind;
        match resp.kind.ok_or_else(|| "job_kind_response.kind must be set".to_string())? {
            Kind::Setup(r) => Ok(DomainJobKindResponse::Setup { vk: r.vk, hash_mode: r.hash_mode }),
            Kind::Prove(r) => {
                let proof =
                    r.proof.ok_or_else(|| "prove.proof must be set".to_string())?.try_into()?;
                let stats = r.stats.map(DomainExecutionStats::from).unwrap_or_default();
                Ok(DomainJobKindResponse::Prove { proof, stats })
            }
            Kind::Wrap(r) => {
                let proof =
                    r.proof.ok_or_else(|| "wrap.proof must be set".to_string())?.try_into()?;
                Ok(DomainJobKindResponse::Wrap(proof))
            }
            Kind::Execute(r) => {
                let stats = r.stats.map(DomainExecutionStats::from).unwrap_or_default();
                Ok(DomainJobKindResponse::Execute { stats, public_outputs: r.public_outputs })
            }
            Kind::SetupAggregationProgram(r) => {
                Ok(DomainJobKindResponse::SetupAggregationProgram {
                    vk: r.vk,
                    hash_mode: r.hash_mode,
                })
            }
            Kind::AggregateProofs(r) => {
                let proof = r
                    .proof
                    .ok_or_else(|| "aggregate_proofs.proof must be set".to_string())?
                    .try_into()?;
                Ok(DomainJobKindResponse::AggregateProofs(proof))
            }
        }
    }
}

impl TryFrom<JobEvent> for DomainJobEvent {
    type Error = String;

    fn try_from(event: JobEvent) -> std::result::Result<Self, Self::Error> {
        use job_event::Event;
        match event.event.ok_or_else(|| "job_event.event must be set".to_string())? {
            Event::Queued(e) => Ok(DomainJobEvent::Queued(DomainJobEventQueued {
                job_id: parse_uuid(&e.job_id).map_err(|e| format!("{e}"))?,
                timestamp: e.timestamp.and_then(ts_to_datetime).unwrap_or_else(chrono::Utc::now),
            })),
            Event::Started(e) => Ok(DomainJobEvent::Started(DomainJobEventStarted {
                job_id: parse_uuid(&e.job_id).map_err(|e| format!("{e}"))?,
                timestamp: e.timestamp.and_then(ts_to_datetime).unwrap_or_else(chrono::Utc::now),
            })),
            Event::Progress(e) => Ok(DomainJobEvent::Progress(DomainJobEventProgress {
                job_id: parse_uuid(&e.job_id).map_err(|e| format!("{e}"))?,
                phase: DomainJobPhase::try_from(e.phase())?,
                timestamp: e.timestamp.and_then(ts_to_datetime).unwrap_or_else(chrono::Utc::now),
            })),
            Event::WaitingForInput(e) => {
                Ok(DomainJobEvent::WaitingForInput(DomainJobEventWaitingForInput {
                    job_id: parse_uuid(&e.job_id).map_err(|e| format!("{e}"))?,
                    timestamp: e
                        .timestamp
                        .and_then(ts_to_datetime)
                        .unwrap_or_else(chrono::Utc::now),
                }))
            }
            Event::Completed(e) => {
                let result = e
                    .result
                    .ok_or_else(|| "completed event must have result".to_string())?
                    .try_into()?;
                Ok(DomainJobEvent::Completed(DomainJobEventCompleted {
                    job_id: parse_uuid(&e.job_id).map_err(|err| format!("{err}"))?,
                    result,
                    timestamp: e
                        .timestamp
                        .and_then(ts_to_datetime)
                        .unwrap_or_else(chrono::Utc::now),
                }))
            }
            Event::Cancelled(e) => Ok(DomainJobEvent::Cancelled(DomainJobEventCancelled {
                job_id: parse_uuid(&e.job_id).map_err(|e| format!("{e}"))?,
                timestamp: e.timestamp.and_then(ts_to_datetime).unwrap_or_else(chrono::Utc::now),
            })),
            Event::Failed(e) => {
                let failure = e
                    .failure
                    .ok_or_else(|| "failed event must have failure".to_string())?
                    .try_into()?;
                Ok(DomainJobEvent::Failed(DomainJobEventFailed {
                    job_id: parse_uuid(&e.job_id).map_err(|err| format!("{err}"))?,
                    failure,
                    timestamp: e
                        .timestamp
                        .and_then(ts_to_datetime)
                        .unwrap_or_else(chrono::Utc::now),
                }))
            }
        }
    }
}
