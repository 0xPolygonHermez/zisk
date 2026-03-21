// ── Lookup keys ───────────────────────────────────────────────────────────────

pub enum ProgramLookup {
    ProgramId(String),
    HashId(String),
    Name(String),
}

pub enum ProgramOrHashLookup {
    ProgramId(String),
    HashId(String),
}

// ── Program request/response types ───────────────────────────────────────────

pub struct RegisterProgramParams {
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub metadata: Option<String>,
    pub zisk_elf: Vec<u8>,
}

pub struct UpdateProgramParams {
    pub program_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub metadata: Option<String>,
    pub zisk_elf: Option<Vec<u8>>,
}

pub enum ProgramStatus {
    Provisioning,
    Ready,
    Failed,
}

impl TryFrom<i32> for ProgramStatus {
    type Error = crate::errors::NodeError;

    fn try_from(raw: i32) -> Result<Self, Self::Error> {
        use zisk_distributed_grpc_api::ProgramStatus as CoordStatus;
        match CoordStatus::try_from(raw) {
            Ok(CoordStatus::Provisioning) => Ok(Self::Provisioning),
            Ok(CoordStatus::Ready) => Ok(Self::Ready),
            Ok(CoordStatus::Failed) => Ok(Self::Failed),
            Err(_) => {
                tracing::error!(status = raw, "received unknown program status from coordinator");
                Err(crate::errors::NodeError::InvalidCoordinatorResponse(format!(
                    "unknown program status: {raw}"
                )))
            }
        }
    }
}

pub struct ProgramSummary {
    pub program_id: String,
    pub hash_id: String,
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub metadata: Option<String>,
    pub created_at_ms: Option<u64>,
    pub status: ProgramStatus,
}

pub struct RegisterProgramResult {
    pub program_id: String,
    pub hash_id: String,
    pub status: ProgramStatus,
}

pub struct UpdateProgramResult {
    pub program_id: String,
    pub hash_id: String,
    pub status: ProgramStatus,
}

// ── Node info ─────────────────────────────────────────────────────────────────

pub struct SetupInfo {
    pub setup_id: String,
    pub verifier_id: String,
    pub proof_kinds: Vec<ProofKind>,
}

pub enum ProofKind {
    Stark,
    StarkMinimal,
    Plonk,
}

pub struct NodeVersionInfo {
    pub zisk_version: String,
    pub available_setups: Vec<SetupInfo>,
}

// ── Job types ─────────────────────────────────────────────────────────────────

pub enum JobKind {
    Prove(ProofKind),
}

pub struct Proof {
    pub proof_id: String,
    pub program_id: String,
    pub verification_key: Vec<u8>,
    pub proof_kind: ProofKind,
    pub data: Vec<u8>,
    pub public_inputs: Vec<u8>,
    pub started_at_ms: Option<u64>,
    pub completed_at_ms: Option<u64>,
}

pub enum JobPhase {
    Contributions,
    Prove,
    Aggregate,
}

impl TryFrom<&str> for JobPhase {
    type Error = crate::errors::NodeError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "Contributions" => Ok(Self::Contributions),
            "Prove" => Ok(Self::Prove),
            "Aggregate" => Ok(Self::Aggregate),
            _ => {
                tracing::error!(phase = s, "received unknown job phase from coordinator");
                Err(crate::errors::NodeError::InvalidCoordinatorResponse(format!(
                    "unknown job phase: {s}"
                )))
            }
        }
    }
}

pub enum JobStatus {
    Queued,
    Running(JobPhase),
    WaitingForInput,
    Completed,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn from_coordinator(state: &str, phase: Option<&str>) -> crate::errors::NodeResult<Self> {
        match state {
            "Created" => Ok(Self::Queued),
            s if s.starts_with("Running") => {
                let p = phase.ok_or_else(|| {
                    crate::errors::NodeError::InvalidCoordinatorResponse(
                        "Running job has no phase".into(),
                    )
                })?;
                Ok(Self::Running(JobPhase::try_from(p)?))
            }
            "Completed" => Ok(Self::Completed),
            "Failed" => Ok(Self::Failed),
            "Cancelled" => Ok(Self::Cancelled),
            _ => {
                tracing::error!(state, "received unknown job state from coordinator");
                Err(crate::errors::NodeError::InvalidCoordinatorResponse(format!(
                    "unknown job state: {state}"
                )))
            }
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

pub struct JobSummary {
    pub job_id: String,
    pub program_id: String,
    pub kind: Option<JobKind>,
    pub status: JobStatus,
    pub created_at_ms: u64,
}

pub struct JobInfo {
    pub job_id: String,
    pub program_id: String,
    pub kind: Option<JobKind>,
    pub status: JobStatus,
    pub created_at_ms: u64,
    pub completed_at_ms: Option<u64>,
    pub result: Option<Proof>,
    pub error: Option<String>,
}

pub struct CancelJobResult {
    pub job_id: String,
    pub previous_status: JobStatus,
}
