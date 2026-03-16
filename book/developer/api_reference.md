# ZisK user API — Reference

---

## Summary

| Method | Category | Description |
|--------|----------|-------------|
| [`GetNodeInfo`](#getnodeinfo) | Node | Query node version and proof capabilities |
| [`ListGuestPrograms`](#listguestprograms) | Program | List all programs registered in the cluster |
| [`GetGuestProgram`](#getguestprogram) | Program | Get full details of a single program |
| [`AddGuestProgram`](#addguestprogram) | Program | Register a new program |
| [`UpdateGuestProgram`](#updateguestprogram) | Program | Update mutable fields of an existing program |
| [`DeleteGuestProgram`](#deleteguestprogram) | Program | Remove a program from the cluster |
| [`Prove`](#prove) | Proof | Submit a job (execute, stats, verify constraints, or prove) |
| [`ListJobs`](#listjobs) | Runtime | List jobs with optional filters |
| [`GetJob`](#getjob) | Runtime | Get full details and current status of a job |
| [`CancelJob`](#canceljob) | Runtime | Cancel a queued or running job |

---

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

---

## Program Management

A **GuestProgram** is a ZisK program registered in the cluster. It holds the ELF binaries and
metadata needed to execute, prove, and verify. The `hash_id` is derived from `zisk_elf` at
registration time and serves as the content-addressed identifier.

### `ListGuestPrograms`

List all programs registered in the cluster, with optional filters. Returns lightweight summaries
without binary fields.

```
ListGuestProgramsRequest → Page<GuestProgramSummary>
```

```rust
struct ListGuestProgramsRequest {
    name:   Option<String>,  // filter by name (substring match)
    author: Option<String>,  // filter by author
    limit:  Option<u32>,     // max items per page; server default applies if omitted
    cursor: Option<String>,  // continuation token from a previous Page response
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

---

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

---

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

---

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

---

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

---

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
    job_id: String,
    timestamp: DateTime<Utc>,
}

struct JobEventProgress {
    job_id: String,
    phase: JobPhase,
    timestamp: DateTime<Utc>,
}

struct JobEventCompleted {
    job_id: String,
    result: JobResult,
    timestamp: DateTime<Utc>,
}

struct JobEventFailed {
    job_id: String,
    error: String,
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
    proof_id:      String,         // unique proof identifier (UUID)
    program_id:    String,         // GuestProgram ID used to generate this proof
    verification_key: Vec<u8>,     // raw verification key
    proof_kind:    ProofKind,
    data:          Vec<u8>,        // serialized proof
    public_inputs: Vec<u8>,
    started_at:    DateTime<Utc>,
    completed_at:  DateTime<Utc>,
}
```

---

## Runtime Management

### `ListJobs`

List jobs with optional filters on status and time range.

```
ListJobsRequest → Page<JobSummary>
```

```rust
struct ListJobsRequest {
    status: Option<JobStatus>,
    since:  Option<DateTime<Utc>>,
    until:  Option<DateTime<Utc>>,
    limit:  Option<u32>,    // max items per page; server default applies if omitted
    cursor: Option<String>, // continuation token from a previous Page response
}

struct JobSummary {
    id:         String,
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

---

### `GetJob`

Get full details and current status of a job.

```
GetJobRequest → JobInfo
```

```rust
struct GetJobRequest {
    id:            String,
    blocking_time: Option<Duration>, // if set, hold connection until status changes or timeout elapses
}

struct JobInfo {
    id:           String,
    program_id:   String,
    kind:         JobKind,
    status:       JobStatus,
    result:       Option<JobResult>,       // present when status is Completed
    error:        Option<String>,          // present when status is Failed
    created_at:   DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}
```

---

### `CancelJob`

Cancel a queued or running job.

```
CancelJobRequest → CancelJobResponse
```

```rust
struct CancelJobRequest {
    id: String,
}

struct CancelJobResponse {
    id:         String,
    job_status: JobStatus,
}
```

--- 

## Common Types

Paginated list wrapper — used by all List methods.

```rust
struct Page<T> {
    items:       Vec<T>,
    next_cursor: Option<String>, // absent when no further pages exist
    total:       u64,            // total count
}
```

---

## Admin Management

| Method | Category | Description |
|--------|----------|-------------|
| [`Clean`](#clean) | Node | Reset all ZisK state on this node |
| [`ListSetups`](#listsetups) | Setup | List all setups available on this node |
| [`GetSetup`](#getsetup) | Setup | Get details of a single setup |
| [`AddSetup`](#addsetup) | Setup | Download and install a new setup |
| [`UpdateSetup`](#updatesetup) | Setup | Update mutable fields of an existing setup |
| [`DeleteSetup`](#deletesetup) | Setup | Remove a setup from this node |
| [`ListClusters`](#listclusters) | Cluster | List all clusters with status and member counts |
| [`CreateCluster`](#createcluster) | Cluster | Create a new cluster and issue its cluster key |
| [`GetCluster`](#getcluster) | Cluster | Get full details of a cluster |
| [`DeleteCluster`](#deletecluster) | Cluster | Delete a cluster and stop all its processes |
| [`ListClusterWorkers`](#listclusterworkers) | Cluster | List workers assigned to a specific cluster |
| [`AssignWorker`](#assignworker) | Cluster | Assign an existing worker to a cluster |
| [`UnassignWorker`](#unassignworker) | Cluster | Remove a worker from a cluster |
| [`MoveWorker`](#moveworker) | Cluster | Move a worker from one cluster to another |
| [`ListCoordinators`](#listcoordinators) | Process | List all coordinator processes across the deployment |
| [`CreateCoordinator`](#createcoordinator) | Process | Create and start a new coordinator process |
| [`GetCoordinator`](#getcoordinator) | Process | Get status and details of a coordinator |
| [`DeleteCoordinator`](#deletecoordinator) | Process | Stop and remove a coordinator process |
| [`ListWorkers`](#listworkers) | Process | List all worker processes across the deployment |
| [`CreateWorker`](#createworker) | Process | Create and start a new worker process on a node |
| [`GetWorker`](#getworker) | Process | Get status and details of a worker |
| [`DeleteWorker`](#deleteworker) | Process | Stop and remove a worker process |
| [`AssignGPUs`](#assigngpus) | Process | Update the GPU subset assigned to a worker |

---

### `Clean`

Reset all ZisK state on this node: installed setups, registered programs, and any cached files.

```
CleanRequest → ()
```

```rust
struct CleanRequest {}
```

---

## Setup Management

### `ListSetups`

List all setups currently installed on this node.

```
ListSetupsRequest → Page<SetupSummary>
```

```rust
struct ListSetupsRequest {
    limit:  Option<u32>,    // max items per page; server default applies if omitted
    cursor: Option<String>, // continuation token from a previous Page response
}

struct SetupSummary {
    id:          String,
    version:     String,
    description: Option<String>,
    proof_kinds: Vec<ProofKind>,
    is_default:  bool,
    created_at:  DateTime<Utc>,
}
```

---

### `GetSetup`

Get details of a single setup by its ID.

```
GetSetupRequest → SetupSummary
```

```rust
struct GetSetupRequest {
    id: String,
}
```

---

### `AddSetup`

Download and install a new setup on this node.

```
AddSetupRequest → AddSetupResponse
```

```rust
struct AddSetupRequest {
    version:     String,
    description: Option<String>,
    uri:         String,  // where to download the setup archive
    proof_kinds: Vec<ProofKind>,
}

struct AddSetupResponse {
    id: String,
}
```

---

### `UpdateSetup`

Update mutable fields of an existing setup. Setting `is_default: true` atomically clears the
previous default and marks this setup as the new one.

```
UpdateSetupRequest → UpdateSetupResponse
```

```rust
struct UpdateSetupRequest {
    id:          String,
    description: Option<String>,
    is_default:  Option<bool>,
}

struct UpdateSetupResponse {
    id:          String,
    description: String,
    is_default:  bool,
}
```

---

### `DeleteSetup`

Remove an installed setup from this node.

```
DeleteSetupRequest → ()
```

```rust
struct DeleteSetupRequest {
    id: String,
}
```

--- 

### `ListClusters`

List all clusters with their status and member counts.

```
ListClustersRequest → Vec<ClusterSummary>
```

```rust
struct ListClustersRequest {}

struct ClusterSummary {
    id:          String,
    coordinator: String,      // coordinator instance name
    workers:     Vec<String>,
    status:      ProcessStatus,
}

enum ProcessStatus {
    Starting,
    Running,
    Stopped,
    Failed,
}
```

---

### `CreateCluster`

Create a new cluster and issue its cluster key. The cluster key must be stored securely by the
caller — it cannot be retrieved again.

```
CreateClusterRequest → CreateClusterResponse
```

```rust
struct CreateClusterRequest {
    id:          String,  // unique cluster name
    coordinator: CreateCoordinatorRequest,
}

struct CreateClusterResponse {
    id:          String,
    cluster_key: String,  // issued cluster key — store securely
}
```

---

### `GetCluster`

Get full details of a cluster, including its coordinator and all assigned workers.

```
GetClusterRequest → ClusterInfo
```

```rust
struct GetClusterRequest {
    id: String,
}

struct ClusterInfo {
    id:          String,
    coordinator: CoordinatorInfo,
    workers:     Vec<WorkerInfo>,
}
```

---

### `DeleteCluster`

Delete a cluster and stop all its processes.

```
DeleteClusterRequest → ()
```

```rust
struct DeleteClusterRequest {
    id: String,
}
```

---

### `ListClusterWorkers`

List all workers assigned to a specific cluster.

```
ListClusterWorkersRequest → Vec<WorkerSummary>
```

```rust
struct ListClusterWorkersRequest {
    cluster_id: String,
}
```

---

### `AssignWorker`

Assign an existing worker process to a cluster.

```
AssignWorkerRequest → ()
```

```rust
struct AssignWorkerRequest {
    cluster_id: String,
    instance:   String,  // worker instance name
}
```

---

### `UnassignWorker`

Remove a worker from a cluster without stopping it.

```
UnassignWorkerRequest → ()
```

```rust
struct UnassignWorkerRequest {
    cluster_id: String,
    instance:   String,
}
```

---

### `MoveWorker`

Atomically move a worker from one cluster to another.

```
MoveWorkerRequest → ()
```

```rust
struct MoveWorkerRequest {
    instance:        String,
    from_cluster_id: String,
    to_cluster_id:   String,
}
```

---

### `ListCoordinators`

List all coordinator processes across the deployment, optionally filtered by cluster.

```
ListCoordinatorsRequest → Vec<CoordinatorSummary>
```

```rust
struct ListCoordinatorsRequest {
    cluster_id: Option<String>,
}

struct CoordinatorSummary {
    instance:   String,
    machine:    String,
    port:       u16,
    cluster_id: String,
    status:     ProcessStatus,
}
```

---

### `CreateCoordinator`

Create and start a new coordinator process on a machine.

```
CreateCoordinatorRequest → CoordinatorInfo
```

```rust
struct CreateCoordinatorRequest {
    instance:   String,  // unique instance name
    machine:    String,  // must exist in machines registry
    port:       u16,
    cluster_id: String,
}

struct CoordinatorInfo {
    instance:   String,
    machine:    String,
    port:       u16,
    cluster_id: String,
    status:     ProcessStatus,
    pid:        Option<u32>,
}
```

---

### `GetCoordinator`

Get status and details of a coordinator process.

```
GetCoordinatorRequest → CoordinatorInfo
```

```rust
struct GetCoordinatorRequest {
    instance: String,
}
```

---

### `DeleteCoordinator`

Stop and remove a coordinator process.

```
DeleteCoordinatorRequest → ()
```

```rust
struct DeleteCoordinatorRequest {
    instance: String,
}
```

---

### `ListWorkers`

List all worker processes across the deployment, optionally filtered by machine or cluster.

```
ListWorkersRequest → Vec<WorkerSummary>
```

```rust
struct ListWorkersRequest {
    machine:    Option<String>,
    cluster_id: Option<String>,
}

struct WorkerSummary {
    instance:   String,
    machine:    String,
    port:       u16,
    cluster_id: Option<String>,
    gpus:       Vec<u32>,
    status:     ProcessStatus,
}
```

---

### `CreateWorker`

Create and start a new worker process on a machine, pinned to a set of GPUs.

```
CreateWorkerRequest → WorkerInfo
```

```rust
struct CreateWorkerRequest {
    instance: String,  // unique instance name
    machine:  String,
    port:     u16,
    gpus:     Vec<u32>,
}

struct WorkerInfo {
    instance:   String,
    machine:    String,
    port:       u16,
    cluster_id: Option<String>,
    gpus:       Vec<u32>,
    status:     ProcessStatus,
    pid:        Option<u32>,
}
```

---

### `GetWorker`

Get status and details of a worker process.

```
GetWorkerRequest → WorkerInfo
```

```rust
struct GetWorkerRequest {
    instance: String,
}
```

---

### `DeleteWorker`

Stop and remove a worker process.

```
DeleteWorkerRequest → ()
```

```rust
struct DeleteWorkerRequest {
    instance: String,
}
```

---

### `AssignGPUs`

Update the GPU subset assigned to a worker. Takes effect on the next job; does not interrupt
work in progress.

```
AssignGPUsRequest → WorkerInfo
```

```rust
struct AssignGPUsRequest {
    instance: String,
    gpus:     Vec<u32>,
}
```

---

