use crate::grpc::user;
use crate::util::ms_to_timestamp;
use crate::service::types::{
    CancelJobResult, JobInfo, JobKind, JobPhase, JobStatus, JobSummary, NodeVersionInfo,
    ProgramLookup, ProgramOrHashLookup, ProgramStatus, ProgramSummary, Proof, ProofKind,
    RegisterProgramParams, RegisterProgramResult, SetupInfo, UpdateProgramParams,
    UpdateProgramResult,
};


impl From<JobPhase> for user::JobPhase {
    fn from(p: JobPhase) -> Self {
        match p {
            JobPhase::Contributions => Self::Contributions,
            JobPhase::Prove => Self::Prove,
            JobPhase::Aggregate => Self::Aggregate,
        }
    }
}

impl From<JobStatus> for user::JobStatus {
    fn from(status: JobStatus) -> Self {
        let (code, phase) = match status {
            JobStatus::Queued => (user::JobStatusCode::Queued, None),
            JobStatus::Running(p) => (user::JobStatusCode::Running, Some(user::JobPhase::from(p) as i32)),
            JobStatus::WaitingForInput => (user::JobStatusCode::WaitingForInput, None),
            JobStatus::Completed => (user::JobStatusCode::Completed, None),
            JobStatus::Failed => (user::JobStatusCode::Failed, None),
            JobStatus::Cancelled => (user::JobStatusCode::Cancelled, None),
        };
        Self { code: code as i32, phase }
    }
}

impl From<ProgramStatus> for user::ProgramStatus {
    fn from(s: ProgramStatus) -> Self {
        match s {
            ProgramStatus::Provisioning => Self::Provisioning,
            ProgramStatus::Ready => Self::Ready,
            ProgramStatus::Failed => Self::Failed,
        }
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

impl From<ProofKind> for user::ProofKind {
    fn from(k: ProofKind) -> Self {
        match k {
            ProofKind::Stark => Self::Stark,
            ProofKind::StarkMinimal => Self::StarkMinimal,
            ProofKind::Plonk => Self::Plonk,
        }
    }
}

impl From<JobKind> for user::JobKind {
    fn from(k: JobKind) -> Self {
        let kind = match k {
            JobKind::Prove(proof_kind) => {
                user::job_kind::Kind::Prove(user::ProofKind::from(proof_kind) as i32)
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
            proof_kind: user::ProofKind::from(p.proof_kind) as i32,
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
            proof_kinds: s
                .proof_kinds
                .into_iter()
                .map(|k| user::ProofKind::from(k) as i32)
                .collect(),
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
            status: user::ProgramStatus::from(p.status) as i32,
        }
    }
}

impl From<RegisterProgramResult> for user::RegisterGuestProgramResponse {
    fn from(r: RegisterProgramResult) -> Self {
        Self {
            program_id: r.program_id,
            hash_id: r.hash_id,
            status: user::ProgramStatus::from(r.status) as i32,
        }
    }
}

impl From<UpdateProgramResult> for user::UpdateGuestProgramResponse {
    fn from(r: UpdateProgramResult) -> Self {
        Self {
            program_id: r.program_id,
            hash_id: r.hash_id,
            status: user::ProgramStatus::from(r.status) as i32,
        }
    }
}

impl From<JobSummary> for user::JobSummary {
    fn from(j: JobSummary) -> Self {
        Self {
            job_id: j.job_id,
            program_id: j.program_id,
            kind: j.kind.map(Into::into),
            status: Some(j.status.into()),
            created_at: Some(ms_to_timestamp(j.created_at_ms)),
        }
    }
}

impl From<JobInfo> for user::JobInfo {
    fn from(j: JobInfo) -> Self {
        let status = j.status.into();
        let completed_at = j.completed_at_ms.map(ms_to_timestamp);
        let result = j
            .result
            .map(|p| user::JobResult { result: Some(user::job_result::Result::Prove(p.into())) });

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
        Self { job_id: r.job_id, job_status: Some(r.previous_status.into()) }
    }
}
