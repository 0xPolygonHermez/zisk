# ZisK User API — Reference

_Version 1.0 · Creation date: 17-03-2026 · Last update: 17-03-2026_

## Summary

| Method                                          | Description                                                      |
| ----------------------------------------------- | ---------------------------------------------------------------- |
| [`RegisterGuestProgram`](#registerguestprogram) | Register a new program                                           |
| [`JobRequest`](#jobrequest)                     | Submit a new job                                                 |
| [`WaitJobResult`](#waitjobresult)               | Block until a job reaches a terminal state and return the result |
| [`WatchJob`](#watchjob)                         | Subscribe to state events for a job (reconnectable)              |
| [`PushJobInput`](#pushjobinput)                 | Push input data to a job waiting for input                       |

**Job Kinds** (submitted via [`JobRequest`](#jobrequest)):

| Kind                                              | Description                           |
| ------------------------------------------------- | ------------------------------------- |
| [`SetupGuestProgram`](#setupguestprogram)         | Prepare the guest program for proving |
| [`Prove`](#prove)                                 | Generate a proof                      |
| [`Wrap`](#wrap)                                   | Wrap a proof into a target format     |
| [`Aggregate`](#aggregate)                         | Aggregate multiple proofs             |
| [`Execute`](#execute)                             | Execute without generating a proof    |

### Common data types

```rust
enum ProofKind {
    Stark,
    StarkMinimal,
    Plonk,
}

enum InputKind {
    Inline(InputChunk), // first chunk; if is_last=true no further PushJobInput calls needed
    Stream(String),     // file:// socket:// quic://
}

struct InputChunk {
    data:    Vec<u8>,
    is_last: bool, // if true, input is complete; if false, send remaining chunks via PushJobInput
}

struct Proof {
    proof_id:                 uuid,          // unique proof identifier
    hash_id:                  String,        // guest program hash ID used to generate this proof
    verification_key:         Vec<u8>,       // proof verification key
    program_verification_key: Vec<u8>,       // guest program circuit verification key
    proof_kind:               ProofKind,     // format of the proof data
    data:                     Vec<u8>,       // serialized proof bytes
    public_inputs:            Vec<u8>,       // serialized public inputs committed to by the proof
    started_at:               DateTime<Utc>, // when the job started executing
    completed_at:             DateTime<Utc>, // when the proof was finalized
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
    hash_id:    String, // hash of zisk_elf
}
```

## Jobs Management

### `JobRequest`

`JobRequest -> JobResponse`

```rust
struct JobRequest {
    job_kind: JobKind,
}

struct JobResponse {
    job_id: uuid,
}

enum JobKind {
    SetupRequest(SetupRequest),
    ProveRequest(ProveRequest),
    WrapRequest(WrapRequest),
    AggregateRequest(AggregateRequest),
    ExecuteRequest(ExecuteRequest),
}

enum JobKindResponse {
    SetupResponse(SetupResponse),
    ProveResponse(ProveResponse),
    WrapResponse(WrapResponse),
    AggregateResponse(AggregateResponse),
    ExecuteResponse(ExecuteResponse),
}
```

### `WaitJobResult`

The intended primitive for polling a job to completion. The server holds the response for
up to `timeout_seconds` (default 5s, minimum 1s): if the job reaches a terminal state
(Completed, Failed, or Cancelled) within that window, it returns immediately with the final
`WaitJobResultResponse`; otherwise it returns with the current status and the client can re-issue.

This design means the caller can loop on `WaitJobResult` without any sleep or rate-limiting
logic.

Clients **must** set a gRPC deadline greater than `timeout_seconds`.

```
WaitJobResultRequest → WaitJobResultResponse
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

### `SetupGuestProgram`

Prepares the guest program to generate proofs. Submit via `JobRequest`; use `WatchJob` or `WaitJobResult` to observe completion.

```rust
struct SetupRequest {
    hash_id: String, // hash of zisk_elf
}

struct SetupResponse {
    // no payload; completion signals the program is ready for proving
}
```

### `Prove`

Creates a proof job against a registered program. Use `WatchJob` or `WaitJobResult` to observe the job.

```rust
struct ProveRequest {
    hash_id:       String,           // Elf hash id
    input:         InputKind,
    proof_timeout: Option<Duration>, // max duration to generate the proof; server default if omitted
}

struct ProveResponse {
    proof: Proof,
}
```

### `Wrap`

Converts an existing `Proof` to the format specified by `proof_dest`. Typically used to produce a `Plonk` or `StarkMinimal` proof from an initial `Stark` proof. Not all source → destination combinations are valid. Submit via `JobRequest`; use `WatchJob` or `WaitJobResult` to observe the job.

```rust
struct WrapRequest {
    proof:        Proof,
    proof_dest:   ProofKind,         // target format
    wrap_timeout: Option<Duration>,  // max duration for wrapping; server default if omitted
}

struct WrapResponse {
    proof: Proof,
}
```

### `Aggregate`

<!-- TODO: To be defined -->

<!--
```rust
struct AggregateRequest {
    proofs:            Vec<Proof>,
    aggregate_timeout: Option<Duration>,
}

struct AggregateResponse {
    // TODO: To be defined
}
```
-->

### `Execute`

```rust
struct ExecuteRequest {
    hash_id:         String,           // Elf hash id
    input:           InputKind,
    execute_timeout: Option<Duration>, // max duration to execute the program; server default if omitted
}

struct ExecuteResponse {
    // TODO: To be defined
}
```

## Runtime Management

### `WatchJob`

Subscribe to state events for an existing job. The server sends the current state immediately
on connect, then streams each transition until a terminal state (`Completed`, `Failed`, or
`Cancelled`), then closes.

Safe to call after a job has already finished — the server synthesises the terminal event from
stored state and the stream closes immediately. This makes `WatchJob` reconnectable: call it
any time after `JobRequest` returns `job_id`, even after a network gap or client restart.

Consecutive `Progress` events with the same phase are deduplicated server-side. The `Completed` event carries the full `JobKindResponse` result, so no follow-up `WaitJobResult` call is needed when using `WatchJob`.

```
WatchJobRequest → stream JobEvent
```

```rust
struct WatchJobRequest {
    job_id: uuid,
}

enum JobEvent {
    Queued(JobEventQueued),                   // job accepted and waiting for a worker
    Started(JobEventStarted),                 // job assigned to a worker and executing
    Progress(JobEventProgress),               // phase transition within a running job
    WaitingForInput(JobEventWaitingForInput), // job paused; client must call PushJobInput
    Completed(JobEventCompleted),
    Cancelled(JobEventCancelled),
    Failed(JobEventFailed),
}

struct JobEventQueued {
    job_id:    uuid,
    timestamp: DateTime<Utc>,
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

struct JobEventWaitingForInput {
    job_id:    uuid,
    timestamp: DateTime<Utc>,
}

struct JobEventCompleted {
    job_id:    uuid,
    result:    JobKindResponse,
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

// Phases apply to ProveRequest jobs. Other job kinds (Setup, Wrap, Execute)
// emit Started and Completed events but no Progress(JobPhase) events.
enum JobPhase {
    Contributions, // witness generation and partial contributions across workers
    Prove,         // proof generation
    Aggregate,     // proof aggregation
}
```

### `PushJobInput`

Streams additional input chunks to a job in `WaitingForInput` state. A job enters this state
when submitted with `InputKind::Inline` and `is_last: false` on the initial chunk.

**Full multi-chunk input flow:**

1. Submit `JobRequest` with `InputKind::Inline(InputChunk { data: .., is_last: false })`
2. Server emits `JobEventWaitingForInput` — the job is paused awaiting more data
3. Client opens a `PushJobInput` stream and sends the remaining chunks
4. Set `is_last: true` on the final chunk — server closes the input and resumes execution

**Error cases:**
- Calling `PushJobInput` on a job not in `WaitingForInput` returns an error
- If the client stream closes before `is_last: true`, the job transitions to `Failed`

```
stream PushJobInputRequest → ()
```

```rust
struct PushJobInputRequest {
    job_id: uuid,
    chunk:  InputChunk,
}
```
