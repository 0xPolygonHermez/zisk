use crate::grpc::user;
use crate::service::types::{
    CancelJobResult, JobInfo, JobKind, JobPhase, JobStatusCode, JobSummary, NodeVersionInfo,
    Proof, ProofKind, ProgramLookup, ProgramOrHashLookup, ProgramStatus, ProgramSummary,
    RegisterProgramParams, RegisterProgramResult, SetupInfo, UpdateProgramParams,
    UpdateProgramResult,
};

// ── Proto helpers ─────────────────────────────────────────────────────────────

pub(crate) fn ms_to_timestamp(ms: u64) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: (ms / 1000) as i64,
        nanos: ((ms % 1000) * 1_000_000) as i32,
    }
}

pub(crate) fn job_status_to_proto(
    code: &JobStatusCode,
    phase: &JobPhase,
) -> user::JobStatus {
    let code = match code {
        JobStatusCode::Queued => user::JobStatusCode::Queued,
        JobStatusCode::Running => user::JobStatusCode::Running,
        JobStatusCode::Completed => user::JobStatusCode::Completed,
        JobStatusCode::Failed => user::JobStatusCode::Failed,
        JobStatusCode::Cancelled => user::JobStatusCode::Cancelled,
        JobStatusCode::Unspecified => user::JobStatusCode::JobStatusUnspecified,
    } as i32;

    let phase = match phase {
        JobPhase::Contributions => user::JobPhase::Contributions,
        JobPhase::Prove => user::JobPhase::Prove,
        JobPhase::Aggregate => user::JobPhase::Aggregate,
    } as i32;

    user::JobStatus { code, phase }
}

fn program_status_to_proto(s: &ProgramStatus) -> i32 {
    match s {
        ProgramStatus::Provisioning => user::ProgramStatus::Provisioning as i32,
        ProgramStatus::Ready => user::ProgramStatus::Ready as i32,
        ProgramStatus::Failed => user::ProgramStatus::Failed as i32,
    }
}

// ── Request conversions: user proto → domain ──────────────────────────────────

impl From<user::RegisterGuestProgramRequest> for RegisterProgramParams {
    fn from(r: user::RegisterGuestProgramRequest) -> Self {
        Self {
            name: r.name,
            description: r.description,
            author: r.author,
            metadata: r.metadata,
            zisk_elf: r.zisk_elf,
        }
    }
}

impl From<user::UpdateGuestProgramRequest> for UpdateProgramParams {
    fn from(r: user::UpdateGuestProgramRequest) -> Self {
        Self {
            program_id: r.program_id,
            name: r.name,
            description: r.description,
            author: r.author,
            metadata: r.metadata,
            zisk_elf: r.zisk_elf,
        }
    }
}

impl From<user::get_guest_program_request::Lookup> for ProgramLookup {
    fn from(l: user::get_guest_program_request::Lookup) -> Self {
        match l {
            user::get_guest_program_request::Lookup::ProgramId(v) => Self::ProgramId(v),
            user::get_guest_program_request::Lookup::HashId(v) => Self::HashId(v),
            user::get_guest_program_request::Lookup::Name(v) => Self::Name(v),
        }
    }
}

impl From<user::delete_guest_program_request::Lookup> for ProgramOrHashLookup {
    fn from(l: user::delete_guest_program_request::Lookup) -> Self {
        match l {
            user::delete_guest_program_request::Lookup::ProgramId(v) => Self::ProgramId(v),
            user::delete_guest_program_request::Lookup::HashId(v) => Self::HashId(v),
        }
    }
}

// ── Response conversions: domain → user proto ─────────────────────────────────

fn proof_kind_to_proto(k: &ProofKind) -> i32 {
    match k {
        ProofKind::Stark => user::ProofKind::Stark as i32,
        ProofKind::StarkMinimal => user::ProofKind::StarkMinimal as i32,
        ProofKind::Plonk => user::ProofKind::Plonk as i32,
    }
}

impl From<JobKind> for user::JobKind {
    fn from(k: JobKind) -> Self {
        let kind = match k {
            JobKind::Prove(proof_kind) => {
                user::job_kind::Kind::Prove(proof_kind_to_proto(&proof_kind))
            }
        };
        Self { kind: Some(kind) }
    }
}

impl From<Proof> for user::Proof {
    fn from(p: Proof) -> Self {
        Self {
            proof_id: p.proof_id,
            program_id: p.program_id,
            verification_key: p.verification_key,
            proof_kind: proof_kind_to_proto(&p.proof_kind),
            data: p.data,
            public_inputs: p.public_inputs,
            started_at: p.started_at_ms.map(ms_to_timestamp),
            completed_at: p.completed_at_ms.map(ms_to_timestamp),
        }
    }
}

impl From<SetupInfo> for user::SetupInfo {
    fn from(s: SetupInfo) -> Self {
        Self {
            setup_id: s.setup_id,
            verifier_id: s.verifier_id,
            proof_kinds: s.proof_kinds.iter().map(proof_kind_to_proto).collect(),
        }
    }
}

impl From<NodeVersionInfo> for user::NodeInfo {
    fn from(v: NodeVersionInfo) -> Self {
        Self {
            zisk_version: v.zisk_version,
            available_setups: v.available_setups.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ProgramSummary> for user::GuestProgramSummary {
    fn from(p: ProgramSummary) -> Self {
        Self {
            program_id: p.program_id,
            hash_id: p.hash_id,
            name: p.name,
            description: p.description,
            author: p.author,
            metadata: p.metadata,
            created_at: p.created_at_ms.map(ms_to_timestamp),
            status: program_status_to_proto(&p.status),
        }
    }
}

impl From<RegisterProgramResult> for user::RegisterGuestProgramResponse {
    fn from(r: RegisterProgramResult) -> Self {
        Self {
            program_id: r.program_id,
            hash_id: r.hash_id,
            status: program_status_to_proto(&r.status),
        }
    }
}

impl From<UpdateProgramResult> for user::UpdateGuestProgramResponse {
    fn from(r: UpdateProgramResult) -> Self {
        Self {
            program_id: r.program_id,
            hash_id: r.hash_id,
            status: program_status_to_proto(&r.status),
        }
    }
}

impl From<JobSummary> for user::JobSummary {
    fn from(j: JobSummary) -> Self {
        Self {
            job_id: j.job_id,
            program_id: j.program_id,
            kind: j.kind.map(Into::into),
            status: Some(job_status_to_proto(&j.status_code, &j.phase)),
            created_at: Some(ms_to_timestamp(j.created_at_ms)),
        }
    }
}

impl From<JobInfo> for user::JobInfo {
    fn from(j: JobInfo) -> Self {
        let status = job_status_to_proto(&j.status_code, &j.phase);
        let completed_at = j.completed_at_ms.map(ms_to_timestamp);
        let result = j.result.map(|p| user::JobResult {
            result: Some(user::job_result::Result::Prove(p.into())),
        });

        Self {
            job_id: j.job_id,
            program_id: j.program_id,
            kind: j.kind.map(Into::into),
            status: Some(status),
            result,
            error: j.error,
            created_at: Some(ms_to_timestamp(j.created_at_ms)),
            completed_at,
        }
    }
}

impl From<CancelJobResult> for user::CancelJobResponse {
    fn from(r: CancelJobResult) -> Self {
        Self {
            job_id: r.job_id,
            job_status: Some(job_status_to_proto(&r.status_code, &r.phase)),
        }
    }
}
