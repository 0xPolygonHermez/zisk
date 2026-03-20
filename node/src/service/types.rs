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

pub enum JobStatusCode {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    Unspecified,
}

pub enum JobPhase {
    Contributions,
    Prove,
    Aggregate,
}

pub struct JobSummary {
    pub job_id: String,
    pub program_id: String,
    pub kind: Option<JobKind>,
    pub status_code: JobStatusCode,
    pub phase: JobPhase,
    pub created_at_ms: u64,
}

pub struct JobInfo {
    pub job_id: String,
    pub program_id: String,
    pub kind: Option<JobKind>,
    pub status_code: JobStatusCode,
    pub phase: JobPhase,
    pub created_at_ms: u64,
    pub completed_at_ms: Option<u64>,
    pub result: Option<Proof>,
    pub error: Option<String>,
}

pub struct CancelJobResult {
    pub job_id: String,
    pub status_code: JobStatusCode,
    pub phase: JobPhase,
}
