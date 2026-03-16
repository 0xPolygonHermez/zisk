# ZisK user API — Reference

## Summary

| Method | Category | Description |
|--------|----------|-------------|
| [`GetNodeInfo`](#getnodeinfo) | Node | Query node version and proof capabilities |
| [`ListGuestPrograms`](#listguestprograms) | Program | List all programs registered in the cluster |
| [`GetGuestProgram`](#getguestprogram) | Program | Get full details of a single program |
| [`AddGuestProgram`](#addguestprogram) | Program | Register a new program |
| [`UpdateGuestProgram`](#updateguestprogram) | Program | Update mutable fields of an existing program |
| [`DeleteGuestProgram`](#deleteguestprogram) | Program | Remove a program from the cluster |
| [`Prove`](#prove) | Proof | Submit a `prove` job |
| [`ListJobs`](#listjobs) | Runtime | List jobs with optional filters |
| [`GetJob`](#getjob) | Runtime | Get full details and current status of a job |
| [`WaitJobResult`](#waitjobresult) | Runtime | Block until a job reaches a terminal state and return the result |
| [`CancelJob`](#canceljob) | Runtime | Cancel a queued or running job |

## Node Management

### `GetNodeInfo`

Query the node's ZisK version and the proof capabilities derived from its installed setups.

```
GetNodeInfoRequest → NodeInfo
```

```rust
struct GetNodeInfoRequest {}

struct NodeInfo {
    zisk_version:     String,
    supported_proofs: Vec<SetupCapabilities>,
}

struct SetupCapabilities {
    setup_id:    String,
    verifier_id: String, // hash of the setup's verification key
    proof_kinds: Vec<ProofKind>,
}

enum ProofKind {
    Stark,
    StarkMinimal,
    Plonk,
}
```

## Program Management

A **GuestProgram** is a ZisK program registered in the cluster. It holds the ELF binaries and
metadata needed to execute and prove. The `hash_id` is derived from `zisk_elf` at
registration time and serves as the content-addressed identifier.

### `ListGuestPrograms`

List all programs registered in the cluster, with optional filters. Returns lightweight summaries
without binary fields.

```
ListGuestProgramsRequest → Vec<GuestProgramSummary>
```

```rust
struct ListGuestProgramsRequest {
    name:   Option<String>,  // filter by name (substring match)
    author: Option<String>,  // filter by author
}

// lightweight — no binary fields
struct GuestProgramSummary {
    program_id:  String,          // program id (UUID); immutable, opaque identifier
    hash_id:     String,          // derived from zisk_elf; content-addressed
    name:        String,
    description: Option<String>,
    author:      Option<String>,
    metadata:    Option<String>,  // JSON
    created_at:  DateTime<Utc>,
    updated_at:  DateTime<Utc>,
}
```

### `GetGuestProgram`

Get details of a single program. Supports lookup by `program_id`, `hash_id`, or `name`.

```
GetGuestProgramRequest → GuestProgramSummary
```

```rust
struct GetGuestProgramRequest {
    program_id: Option<String>,
    hash_id:    Option<String>,
    name:       Option<String>, // substring match; may return multiple results if not unique
} // one of program_id, hash_id, or name must be supplied
```

### `AddGuestProgram`

Register a new program in the cluster. `hash_id` is computed from `zisk_elf` at registration time.

```
AddGuestProgramRequest → AddGuestProgramResponse
```

```rust
struct AddGuestProgramRequest {
    name:        String,
    description: Option<String>,
    author:      Option<String>,
    zisk_elf:    Vec<u8>,
    native_elf:  HashMap<ElfKind, Vec<u8>>,
    metadata:    Option<String>,
}

enum ElfKind {
    X86_64,
    Arm,
}

struct AddGuestProgramResponse {
    program_id: String, // program id (UUID)
    hash_id:    String, // derived from zisk_elf
}
```

### `UpdateGuestProgram`

Update mutable fields of an existing program. Supplying a new `zisk_elf` triggers recomputation
of `hash_id`.

```
UpdateGuestProgramRequest → UpdateGuestProgramResponse
```

```rust
struct UpdateGuestProgramRequest {
    program_id:  String,          // program UUID
    name:        Option<String>,
    description: Option<String>,
    author:      Option<String>,
    zisk_elf:    Option<Vec<u8>>, // triggers hash_id recomputation
    native_elf:  Option<HashMap<ElfKind, Vec<u8>>>,
    metadata:    Option<String>,
}

struct UpdateGuestProgramResponse {
    program_id: String, // program UUID
    hash_id:    String, // derived from zisk_elf (updated if zisk_elf was supplied)
}
```

### `DeleteGuestProgram`

Remove a program from the cluster.

```
DeleteGuestProgramRequest → ()
```

```rust
struct DeleteGuestProgramRequest {
    program_id: Option<String>,
    hash_id:    Option<String>,
} // one of program_id or hash_id must be supplied
```

## Proof Requests

### `Prove`

Submit a proof job against a registered program. Streams `JobEvent` values back to the caller
until the job reaches a terminal state.

```
ProveRequest → stream JobEvent
```

```rust
struct ProveRequest {
    program_id:    String,             // GuestProgram ID
    setup_id:      Option<ProveSetup>, // Setup ID to use for this job
    input:         InputKind,
    job_kind:      JobKind,
    webhook_url:   Option<String>,     // if set, POST JobEvent::Completed / JobEvent::Failed here
    proof_timeout: Option<Duration>,   // max duration to generate the proof; server default applies if omitted
}

enum ProveSetup {
    SetupId(String),
    VerifierId(String),   // hash of the setup's verification key
    VerifierKey(Vec<u8>), // raw verification key
}

enum InputKind {
    Raw(Vec<u8>),
    Inputs(String), // file path or http:// URL
    Stream(String), // file:// socket:// quic://
}

enum JobKind {
    Prove(ProofKind),  // generate a proof
}

enum JobEvent {
    Started(JobEventStarted),
    Progress(JobEventProgress),
    Completed(JobEventCompleted),
    Failed(JobEventFailed),
}

struct JobEventStarted {
    job_id:    String,
    timestamp: DateTime<Utc>,
}

struct JobEventProgress {
    job_id:    String,
    phase:     JobPhase,
    timestamp: DateTime<Utc>,
}

struct JobEventCompleted {
    job_id:    String,
    result:    JobResult,
    timestamp: DateTime<Utc>,
}

struct JobEventFailed {
    job_id:    String,
    error:     String,
    timestamp: DateTime<Utc>,
}

enum JobPhase {
    Contributions,
    Prove,
    Aggregate
}

// result payload varies by JobKind
enum JobResult {
    Prove(Proof),
}

struct Proof {
    proof_id:         String,        // unique proof identifier (UUID)
    program_id:       String,        // GuestProgram ID used to generate this proof
    verification_key: Vec<u8>,       // raw verification key
    proof_kind:       ProofKind,
    data:             Vec<u8>,       // serialized proof
    public_inputs:    Vec<u8>,
    started_at:       DateTime<Utc>,
    completed_at:     DateTime<Utc>,
}
```

## Runtime Management

### `ListJobs`

List jobs with optional filters on status and time range.

```
ListJobsRequest → Vec<JobSummary>
```

```rust
struct ListJobsRequest {
    status: Option<JobStatus>,
    since:  Option<DateTime<Utc>>,
    until:  Option<DateTime<Utc>>,
}

struct JobSummary {
    job_id:     String,
    program_id: String,
    kind:       JobKind,
    status:     JobStatus,
    created_at: DateTime<Utc>,
}

enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}
```

### `GetJob`

Get full details and current status of a job. Returns immediately with the current state.

```
GetJobRequest → JobInfo
```

```rust
struct GetJobRequest {
    id: String,
}

struct JobInfo {
    job_id:       String,
    program_id:   String,
    kind:         JobKind,
    status:       JobStatus,
    result:       Option<JobResult>,       // present when status is Completed
    error:        Option<String>,          // present when status is Failed
    created_at:   DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}
```

### `WaitJobResult`

The intended primitive for polling a proof to completion. The server holds the response for
up to 5 seconds: if the job reaches a terminal state (Completed, Failed, or Cancelled)
within that window, it returns immediately with the final `JobInfo`; if the 5 seconds elapse
first, it returns with the current status (e.g. Running) and the client could re-issue.

This design means the caller can loop on `WaitJobResult` without any sleep or rate-limiting
logic — the server-side hold ensures at most 12 requests per minute per job regardless of
how tight the loop is. Completion is detected with 0–5 s latency (on average ~2.5 s), which
is imperceptible for jobs that take tens of seconds to minutes.

```
WaitJobResultRequest → JobInfo
```

```rust
struct WaitJobResultRequest {
    job_id: String,
}
```

### `CancelJob`

Cancel a queued or running job.

```
CancelJobRequest → CancelJobResponse
```

```rust
struct CancelJobRequest {
    job_id: String,
}

struct CancelJobResponse {
    job_id:     String,
    job_status: JobStatus,
}
```