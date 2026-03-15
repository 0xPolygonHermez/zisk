# ZisK API — Reference

All methods are exposed on every `zisk node`. Access is governed by two key types:
- **Admin key** — full control over the deployment (cluster topology, hardware, all processes).
- **Cluster key** — scoped to one cluster (job execution, that cluster's process operations).

Proof request methods stream events back to the caller:
`Started` | `Progress` | `Completed { result }` | `Failed { error }`

---

## Summary

| Method | Category | Access | Description |
|--------|----------|--------|-------------|
| [`GetNodeInfo`](#getnodeinfo) | Node | Admin | Query node version and proof capabilities |
| [`Clean`](#clean) | Node | Admin | Reset all ZisK state on this node |
| [`ListSetups`](#listsetups) | Setup | Admin | List all setups available on this node |
| [`AddSetup`](#addsetup) | Setup | Admin | Download and install a new setup |
| [`UpdateSetup`](#updatesetup) | Setup | Admin | Update mutable fields of an existing setup |
| [`DeleteSetup`](#deletesetup) | Setup | Admin | Remove a setup from this node |
| [`ListGuestPrograms`](#listguestprograms) | Program | Cluster | List all programs registered in the cluster |
| [`GetGuestProgram`](#getguestprogram) | Program | Cluster | Get full details of a single program |
| [`AddGuestProgram`](#addguestprogram) | Program | Cluster | Register a new program |
| [`UpdateGuestProgram`](#updateguestprogram) | Program | Cluster | Update mutable fields of an existing program |
| [`DeleteGuestProgram`](#deleteguestprogram) | Program | Cluster | Remove a program from the cluster |
| [`Prove`](#prove) | Proof | Cluster | Submit a job (execute, stats, verify constraints, or prove) |
| [`Verify`](#verify) | Proof | Cluster | Verify a previously generated proof |
| [`ListJobs`](#listjobs) | Runtime | Cluster | List jobs with optional filters |
| [`GetJob`](#getjob) | Runtime | Cluster | Get full details and current status of a job |
| [`CancelJob`](#canceljob) | Runtime | Cluster | Cancel a queued or running job |

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
    proof_kinds: Vec<ProofKind>,
}
```

---

### `Clean`

Reset all ZisK state on this node: installed setups, registered programs, and any cached files.

```
CleanRequest → OpResult
```

```rust
struct CleanRequest {}
```

---

## Setup Management

### `ListSetups`

List all setups currently installed on this node.

```
ListSetupsRequest → Vec<SetupSummary>
```

```rust
struct ListSetupsRequest {}

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
UpdateSetupRequest → OpResult
```

```rust
struct UpdateSetupRequest {
    id:          String,
    description: Option<String>,
    is_default:  Option<bool>,
}
```

---

### `DeleteSetup`

Remove an installed setup from this node.

```
DeleteSetupRequest → OpResult
```

```rust
struct DeleteSetupRequest {
    id: String,
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
ListGuestProgramsRequest → Vec<GuestProgramSummary>
```

```rust
struct ListGuestProgramsRequest {
    name:   Option<String>,  // filter by name (substring match)
    author: Option<String>,  // filter by author
}

// lightweight — no binary fields
struct GuestProgramSummary {
    id:          String,
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

Get full details of a single program, including its ELF binaries.

```
GetGuestProgramRequest → GuestProgramSummary
```

```rust
struct GetGuestProgramRequest {
    id: Option<String>,
    hash_id: Option<String>,
    name: Option<String>,    // substring match; may return multiple results if not unique
} // one of id, hash_id, or name must be supplied
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
    id: String,      // program id (UUID)
    hash_id: String, // derived from zisk_elf
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
    id:          String,          // program UUID
    name:        Option<String>,
    description: Option<String>,
    author:      Option<String>,
    zisk_elf:    Option<Vec<u8>>, // triggers hash_id recomputation
    native_elf:  Option<HashMap<ElfKind, Vec<u8>>>,
    metadata:    Option<String>,
}

struct UpdateGuestProgramResponse {
    id: String,      // program UUID
    hash_id: String, // derived from zisk_elf (updated if zisk_elf was supplied)
}
```

---

### `DeleteGuestProgram`

Remove a program from the cluster.

```
DeleteGuestProgramRequest → OpResult
```

```rust
struct DeleteGuestProgramRequest {
    id: Option<String>,
    hash_id: Option<String>,
} // one of id or hash_id must be supplied
```

---

## Proof Requests

### `Prove`

Submit a job against a registered program. `job_kind` controls what is computed: a plain
execution trace, detailed stats, constraint verification, or a full proof. Streams `JobEvent`
values back to the caller until the job reaches a terminal state.

```
ProveRequest → stream JobEvent
```

```rust
struct ProveRequest {
    program_id:  String,          // GuestProgram ID
    setup_id:    Option<String>,  // Setup ID to use for this job
    input:       InputKind,
    job_kind:    JobKind,
    timeout:     Option<u64>,     // seconds; uses cluster default if omitted
    webhook_url: Option<String>,  // if set, POST JobEvent::Completed / JobEvent::Failed here
}

enum InputKind {
    Raw(Vec<u8>),
    Inputs(String), // file path or http:// URL
    Stream(String), // file:// socket:// quic://
}

enum JobKind {
    Execute,           // run the program and return execution info/stats
    Stats,             // run the program and return detailed execution statistics
    VerifyConstraints, // run the program and verify all constraints are satisfied
    Prove(ProofKind),  // generate a proof
}

enum ProofKind {
    Basic,
    Compressed,
    Plonk,
    Fflonk,
}

enum JobEvent {
    Started   { job_id: String, timestamp: DateTime<Utc> },
    Progress  { job_id: String, phase: JobPhase, timestamp: DateTime<Utc> },
    Completed { job_id: String, result: JobResult, timestamp: DateTime<Utc> },
    Failed    { job_id: String, error: String, timestamp: DateTime<Utc> },
}

enum JobPhase {
    Contributions,
    Prove,
    Aggregate
}

// result payload varies by JobKind
enum JobResult {
    ExecutionInfo(ExecutionInfo),
    Stats(ExecutionStats),
    ConstraintsVerified,
    Proof(Proof),
}

struct Proof {
    proof_id:      String,         // unique proof identifier (UUID)
    program_id:    String,         // GuestProgram ID used to generate this proof
    setup_id:      Option<String>, // Setup ID used for this proof
    job_kind:      JobKind,
    data:          Vec<u8>,        // serialized proof
    public_inputs: Vec<u8>,
    started_at:    DateTime<Utc>,
    completed_at:  DateTime<Utc>,
}
```

---

### `Verify`

Verify a previously generated proof.

```
VerifyRequest → VerifyResult
```

```rust
struct VerifyRequest {
    proof:            Proof,
    proof_kind:       ProofKind,
    verification_key: String, // TODO! How we know currently which vk use to verify a proof?
}

struct VerifyResult {
    valid: bool,
    verified_at: DateTime<Utc>,
}
```

---

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
    id: String,
}

struct JobInfo {
    id:           String,
    program_id:   String,
    kind:         JobKind,
    status:       JobStatus,
    created_at:   DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    error:        Option<String>,
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
    id: String,
    success: bool,
    job_status: JobStatus,
}
```

---

## Cluster Management

| Method | Access | Description |
|--------|--------|-------------|
| [`ListClusters`](#listclusters) | Admin | List all clusters with status and member counts |
| [`CreateCluster`](#createcluster) | Admin | Create a new cluster and issue its cluster key |
| [`GetCluster`](#getcluster) | Admin | Get full details of a cluster |
| [`DeleteCluster`](#deletecluster) | Admin | Delete a cluster and stop all its processes |
| [`ListClusterWorkers`](#listclusterworkers) | Admin | List workers assigned to a specific cluster |
| [`AssignWorker`](#assignworker) | Admin | Assign an existing worker to a cluster |
| [`UnassignWorker`](#unassignworker) | Admin | Remove a worker from a cluster |
| [`MoveWorker`](#moveworker) | Admin | Move a worker from one cluster to another |
| [`ListCoordinators`](#listcoordinators) | Admin | List all coordinator processes across the deployment |
| [`CreateCoordinator`](#createcoordinator) | Admin | Create and start a new coordinator process |
| [`GetCoordinator`](#getcoordinator) | Admin | Get status and details of a coordinator |
| [`DeleteCoordinator`](#deletecoordinator) | Admin | Stop and remove a coordinator process |
| [`ListWorkers`](#listworkers) | Admin | List all worker processes across the deployment |
| [`CreateWorker`](#createworker) | Admin | Create and start a new worker process on a node |
| [`GetWorker`](#getworker) | Admin | Get status and details of a worker |
| [`DeleteWorker`](#deleteworker) | Admin | Stop and remove a worker process |
| [`AssignGPUs`](#assigngpus) | Admin | Update the GPU subset assigned to a worker |

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
DeleteClusterRequest → OpResult
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
AssignWorkerRequest → OpResult
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
UnassignWorkerRequest → OpResult
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
MoveWorkerRequest → OpResult
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
DeleteCoordinatorRequest → OpResult
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
DeleteWorkerRequest → OpResult
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

## Common Types

```rust
struct OpResult {
    success: bool,
    message: Option<String>,
}
```

---

## Monitor (TO DO)

## Store (TO DO)
