# ZisK User API — Reference

*Version 1.0 · Creation date: 17-03-2026 · Last update: 17-03-2026*

## Summary

gRPC protocol buffer definition: [`zisk_user_api.proto`](../../distributed/crates/node/proto/zisk_user_api.proto)

| Method | Category | Description |
|--------|----------|-------------|
| [`GetNodeInfo`](#getnodeinfo) | Node | Query node version and proof capabilities |
| [`ListGuestPrograms`](#listguestprograms) | Program | List all programs registered in the cluster |
| [`GetGuestProgram`](#getguestprogram) | Program | Get details of a single program (binary fields excluded) |
| [`RegisterGuestProgram`](#registerguestprogram) | Program | Register a new program |
| [`WaitGuestProgram`](#waitguestprogram) | Program | Block until a program reaches a terminal state and return the result |
| [`UpdateGuestProgram`](#updateguestprogram) | Program | Update mutable fields of an existing program |
| [`DeleteGuestProgram`](#deleteguestprogram) | Program | Remove a program from the cluster |
| [`Prove`](#prove) | Proof | Submit a `prove` job; returns `job_id` immediately |
| [`WatchJob`](#watchjob) | Runtime | Subscribe to state events for a job (reconnectable) |
| [`ListJobs`](#listjobs) | Runtime | List jobs with optional filters |
| [`GetJob`](#getjob) | Runtime | Get full details and current status of a job |
| [`WaitJobResult`](#waitjobresult) | Runtime | Block until a job reaches a terminal state and return the result |
| [`PushJobInput`](#pushjobinput) | Runtime | Push input data to a job waiting for input |
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
    available_setups: Vec<SetupInfo>,
}

struct SetupInfo {
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

A **GuestProgram** is a ZisK program registered in the cluster. It holds the ELF binary
needed to execute and prove. Each program has a stable `program_id` (UUID) assigned at
registration time, and a `hash_id` (hash of `zisk_elf`) that changes when the ELF is
updated. Use `program_id` as your long-lived handle; `hash_id` identifies a specific ELF
version.

### `ListGuestPrograms`

List all programs registered in the cluster, with optional filters. Returns lightweight summaries
without binary fields.

```
ListGuestProgramsRequest → Vec<GuestProgramSummary>
```

```rust
struct ListGuestProgramsRequest {
    name:   Option<String>, // filter by name (substring match)
    author: Option<String>, // filter by author
}

// binary fields are omitted
struct GuestProgramSummary {
    program_id:  String,         // UUID
    hash_id:     String,         // hash of zisk_elf; changes when ELF is updated
    name:        String,
    description: Option<String>,
    author:      Option<String>,
    status:      ProgramStatus,
    metadata:    Option<String>, // JSON
    created_at:  DateTime<Utc>,
}

enum ProgramStatus {
    Provisioning, // program is being prepared; not yet available for proving
    Ready,        // program is ready to accept proof jobs
    Failed,       // program preparation failed; cannot be used for proving
}
```

### `GetGuestProgram`

Get details of a single program (binary fields excluded). Supports exact-match lookup by
`program_id`, `hash_id`, or `name`.

```
GetGuestProgramRequest → GuestProgramSummary
```

```rust
struct GetGuestProgramRequest {
    // exactly one of program_id, hash_id, or name must be supplied
    program_id: Option<String>,
    hash_id:    Option<String>,
    name:       Option<String>,
}
```

### `RegisterGuestProgram`

Register a new program in the cluster. A UUID `program_id` is assigned at registration time
and a `hash_id` is derived from `zisk_elf`. The call returns immediately with status
`Provisioning` while the cluster prepares the program in the background; once ready, status
transitions to `Ready` and the program can be used for proving. Registration is idempotent —
re-uploading the same ELF returns the existing `program_id` and `hash_id` without
re-triggering preparation.

```
RegisterGuestProgramRequest → RegisterGuestProgramResponse
```

```rust
struct RegisterGuestProgramRequest {
    name:        String,
    description: Option<String>,
    author:      Option<String>,
    zisk_elf:    Vec<u8>,
    metadata:    Option<String>, // JSON
}

struct RegisterGuestProgramResponse {
    program_id: String,        // UUID
    hash_id:    String,        // hash of zisk_elf
    status:     ProgramStatus,
}
```

### `WaitGuestProgram`

The intended primitive for polling a program to readiness. The server holds the response for
up to 5 seconds: if the program reaches a terminal state (`Ready` or `Failed`) within that
window, it returns immediately; if the 5 seconds elapse first, it returns with the current
status and the client can re-issue.

This design means the caller can loop on `WaitGuestProgram` without any sleep or rate-limiting
logic.

```
WaitGuestProgramRequest → GuestProgramSummary
```

```rust
struct WaitGuestProgramRequest {
    program_id: String,
}
```

### `UpdateGuestProgram`

Update mutable fields of an existing program. Supplying a new `zisk_elf` recomputes `hash_id`
and re-prepares the program on the cluster; status returns to `Provisioning` until complete.
The `program_id` is unchanged.

```
UpdateGuestProgramRequest → UpdateGuestProgramResponse
```

```rust
struct UpdateGuestProgramRequest {
    program_id:  String,          // UUID
    name:        Option<String>,
    description: Option<String>,
    author:      Option<String>,
    zisk_elf:    Option<Vec<u8>>, // if set: recomputes hash_id and re-prepares the program
    metadata:    Option<String>,
}

struct UpdateGuestProgramResponse {
    program_id: String,
    hash_id:    String,        // new hash_id if zisk_elf was updated, otherwise unchanged
    status:     ProgramStatus,
}
```

### `DeleteGuestProgram`

Remove a program from the cluster.

```
DeleteGuestProgramRequest → ()
```

```rust
struct DeleteGuestProgramRequest {
    // exactly one of program_id or hash_id must be supplied
    program_id: Option<String>,
    hash_id:    Option<String>,
}
```

## Proof Requests

### `Prove`

Submit a proof job against a registered program. Returns immediately with a `job_id`; proof
generation runs independently on workers regardless of client connectivity. Use `WatchJob`,
`GetJob`, or `WaitJobResult` to observe the job.

```
ProveRequest → ProveResponse
```

```rust
struct ProveRequest {
    program_id:    String,             // GuestProgram ID
    setup:         Option<ProveSetup>, // if not provided, the server uses its default
    input:         InputKind,
    job_kind:      JobKind,
    webhook_url:   Option<String>,     // if set, the server POSTs JobEventCompleted,
                                       // JobEventFailed, or JobEventCancelled to this URL when the
                                       // job reaches a terminal state.
    proof_timeout: Option<Duration>,   // max duration to generate the proof; server default if omitted
}

enum ProveSetup {
    SetupId(String),
    VerifierId(String),   // hash of the setup's verification key
    VerifierKey(Vec<u8>), // raw verification key
}

enum InputKind {
    Inline(InputChunk), // first chunk; if is_last=true no further PushJobInput calls needed
    Inputs(String),     // file path or http(s):// URL
    Stream(String),     // file:// socket:// quic://
}

struct InputChunk {
    data:    Vec<u8>,
    is_last: bool, // if true, input is complete; if false, send remaining chunks via PushJobInput
}

enum JobKind {
    Prove(ProofKind), // generate a proof
}

struct ProveResponse {
    job_id: String,
}
```

## Runtime Management

### `WatchJob`

Subscribe to state events for an existing job. The server sends the current state immediately
on connect, then streams each transition until a terminal state (`Completed`, `Failed`, or
`Cancelled`), then closes.

Safe to call after a job has already finished — the server synthesises the terminal event from
stored state and the stream closes immediately. This makes `WatchJob` reconnectable: call it
any time after `Prove` returns `job_id`, even after a network gap or client restart.

Consecutive `Running` events with the same phase are deduplicated server-side.

```
WatchJobRequest → stream JobEvent
```

```rust
struct WatchJobRequest {
    job_id: String,
}

enum JobEvent {
    Started(JobEventStarted),
    Progress(JobEventProgress),
    Completed(JobEventCompleted),
    Cancelled(JobEventCancelled),
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

struct JobEventCancelled {
    job_id:    String,
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
    Aggregate,
}

enum JobResult {
    Prove(Proof),
}

struct Proof {
    proof_id:         String,        // unique proof identifier (UUID)
    program_id:       String,        // GuestProgram ID used to generate this proof
    verification_key: Vec<u8>,       // raw verification key
    program_verification_key: Vec<u8>,
    proof_kind:       ProofKind,
    data:             Vec<u8>,       // serialized proof bytes
    public_inputs:    Vec<u8>,
    started_at:       DateTime<Utc>,
    completed_at:     DateTime<Utc>,
}
```

### `ListJobs`

List jobs with optional filters on time range.

```
ListJobsRequest → Vec<JobSummary>
```

```rust
struct ListJobsRequest {
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
    Running(JobPhase), // running includes the current phase
    WaitingForInput,   // waiting for input
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
    job_id: String,
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
up to `timeout_seconds` (default 5s, minimum 1s): if the job reaches a terminal state
(Completed, Failed, or Cancelled) within that window, it returns immediately with the final
`JobInfo`; otherwise it returns with the current status and the client can re-issue.

This design means the caller can loop on `WaitJobResult` without any sleep or rate-limiting
logic.

Clients **must** set a gRPC deadline greater than `timeout_seconds`.

```
WaitJobResultRequest → JobInfo
```

```rust
struct WaitJobResultRequest {
    job_id:          String,
    timeout_seconds: Option<u32>, // server-side hold duration; min 1s
}
```

### `PushJobInput`

Push raw input data to a job that is in `WaitingForInput` status. Only valid for jobs
submitted with `InputKind::Inline`. Set
`is_last: true` on the final chunk to signal end of input.

```
stream PushJobInputRequest → ()
```

```rust
struct PushJobInputRequest {
    job_id: String,
    chunk:  InputChunk,
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
    job_status: JobStatus, // state the job was in before cancellation.
}
```

