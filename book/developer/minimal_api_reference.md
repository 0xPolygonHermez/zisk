# ZisK User API — Reference

_Version 1.0 · Creation date: 17-03-2026 · Last update: 17-03-2026_

## Summary

gRPC protocol buffer definition: [`zisk_user_api.proto`](../../distributed/crates/node/proto/zisk_user_api.proto)

| Method                                             | Category | Description                                                      |
| -------------------------------------------------- | -------- | ---------------------------------------------------------------- |
| [`RegisterGuestProgram`](#registerguestprogram) \* | Program  | Register a new program                                           |
| [`SetupGuestProgram`](#setupguestprogram)          | Program  | Prepares the guest to generate proofs                            |
| [`Prove`](#prove) \*                               | Proof    | Submit a `prove` job; returns `job_id` immediately               |
| [`WatchJob`](#watchjob) \*                         | Runtime  | Subscribe to state events for a job (reconnectable)              |
| [`ListJobs`](#listjobs)                            | Runtime  | List jobs with optional filters                                  |
| [`GetJob`](#getjob)                                | Runtime  | Get full details and current status of a job                     |
| [`WaitJobResult`](#waitjobresult) \*               | Runtime  | Block until a job reaches a terminal state and return the result |
| [`PushJobInput`](#pushjobinput)                    | Runtime  | Push input data to a job waiting for input                       |
| [`CancelJob`](#canceljob)                          | Runtime  | Cancel a queued or running job                                   |

### Common data types

```rust
enum ProofKind {
    Stark,
    StarkMinimal,
    Plonk,
}
```

## Program Management

### `RegisterGuestProgram`

Register a new program in the cluster. A content-addressed `hash_id` is derived from `zisk_elf`.

```
RegisterGuestProgramRequest → RegisterGuestProgramResponse
```

```rust
struct RegisterGuestProgramRequest {
    zisk_elf:    Vec<u8>,
}

struct RegisterGuestProgramResponse {
    hash_id:    String,        // hash of zisk_elf
}
```

### `SetupGuestProgram`

```
SetupGuestProgramRequest → SetupGuestProgramResponse
```

```rust
struct SetupGuestProgramRequest {
    hash_id:   String,        // hash of zisk_elf
}

struct SetupGuestProgramResponse {
    job_id:    uuid,
}
```

## Jobs Management

### `JobRequest` and `JobResponse`

`JobRequest -> JobResponse`

```rust
struct JobRequest {
    job_kind: JobKind,
}

struct JobResponse {
    job_id: uuid
}

enum JobKind {
    ProveRequest(ProveRequest),
    WrapRequest(WrapRequest),
    AggregateRequest(AggregateRequest),
    ExecuteRequest(ExecuteRequest),
}

enum JobKindResponse {
    ProveResponse(ProveResponse),
    WrapResponse(WrapResponse),
    AggregateResponse(AggregateResponse),
    ExecuteResponse(ExecuteResponse),
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
    job_id:          uuid,
    timeout_seconds: Option<u32>, // server-side hold duration; min 1s
}

struct WaitJobResultResponse {
    job_id:     uuid,
    job_status: JobStatus,
    result:     Option<JobKindResponse>, // present if job_status is Completed
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

### `Prove`

Creates a proof job against a registered program. Use `WatchJob` or `WaitJobResult` to observe the job.

```rust
struct ProveRequest {
    hash_id:       String,            // Elf hash id,
    input:         InputKind,
    proof_timeout: Option<Duration>,  // max duration to generate the proof; server default if omitted
}

enum InputKind {
    Inline(InputChunk), // first chunk; if is_last=true no further PushJobInput calls needed
    Stream(String),     // file:// socket:// quic://
}

struct InputChunk {
    data:    Vec<u8>,
    is_last: bool, // if true, input is complete; if false, send remaining chunks via PushJobInput
}

struct ProveResponse {
    proof: Proof,
}

struct Proof {
    proof_id:         String,        // unique proof identifier (UUID)
    hash_id:          String,        // Guest program hash ID used to generate this proof
    verification_key: Vec<u8>,       // raw verification key
    program_verification_key: Vec<u8>,
    proof_kind:       ProofKind,
    data:             Vec<u8>,       // serialized proof bytes
    public_inputs:    Vec<u8>,
    started_at:       DateTime<Utc>,
    completed_at:     DateTime<Utc>,
}

```

### Wrap 

```rust
struct WrapRequest {
    proof_dest: ProofKind,
    proof: Proof,
    wrap_timeout: Option<Duration>, // max duration to generate the proof; server default if omitted
}
```

### Aggregate
// TODO To be defined
struct AggregateRequest {
proof: Vec<Proof>,
aggregate_timeout: Option<Duration>,
}

### Execute

```rust
struct ExecuteRequest {
    hash_id:         String,           // Elf hash id,
    input:           InputKind,
    execute_timeout: Option<Duration>, // max duration to generate the proof; server default if omitted
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
    job_id: uuid,
}

enum JobEvent {
    Started(JobEventStarted),
    Progress(JobEventProgress),
    Completed(JobEventCompleted),
    Cancelled(JobEventCancelled),
    Failed(JobEventFailed),
}

struct JobEventStarted {
    job_id:    uuid,
    timestamp: DateTime<Utc>,
}

struct JobEventProgress {
    job_id:    uuid,
    phase:     JobPhase,
    timestamp: DateTime<Utc>,
}

struct JobEventCompleted {
    job_id:    uuid,
    timestamp: DateTime<Utc>,
}

struct JobEventCancelled {
    job_id:    uuid,
    timestamp: DateTime<Utc>,
}

struct JobEventFailed {
    job_id:    uuid,
    error:     String,
    timestamp: DateTime<Utc>,
}

enum JobPhase {
    Contributions,
    Prove,
    Aggregate,
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
    job_id: uuid,
    chunk:  InputChunk,
}
```