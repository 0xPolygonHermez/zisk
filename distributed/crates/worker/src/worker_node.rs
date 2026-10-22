use crate::{
    stop,
    worker::{CancelledWork, ComputationResult, LoopEvent, LoopEventSender},
    ProverConfig, Worker,
};
use anyhow::{anyhow, Context, Result};
use proofman::{AggProofs, ContributionsInfo, WitnessInfo};
use std::collections::{BTreeSet, VecDeque};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use std::{path::PathBuf, time::Duration};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use tonic::Request;
use tracing::{error, info, warn};
use zisk_cluster_api::contribution_params::InputSource;
use zisk_cluster_api::execute_task_response::ResultData;
use zisk_cluster_api::*;
use zisk_cluster_common::{
    AggProofData, AggregationParams, DataCtx, HintsSourceDto, InputSourceDto, JobPhase, ProofKind,
    StreamDataDto, WorkerState,
};
use zisk_cluster_common::{DataId, JobId};
use zisk_common::{ProgramVK, Proof, RandomSeed, StatsCostPerType, ZiskExecutorTime, ZiskPaths};
use zisk_prover_backend::{Asm, Emu, GuestProgram, ZiskBackend, ZiskProver};

use crate::config::WorkerServiceConfig;
use crate::telemetry::{self, ExitReason};

/// The MPI tag of a peer rank's reply that it has stopped a cancelled job's work.
/// Apart from proofman's own: 999996 to 999999, and air group ids for proofs.
pub(crate) const CANCEL_DONE_TAG: i32 = 999_900;

/// What rank 0's recovery does to the cluster, apart from the prover's own state.
pub(crate) trait RecoveryActions {
    fn n_processes(&self) -> i32;
    fn notify_cluster_cancellation(&self);
    /// Tell every peer rank to stop `job_id`'s work and reply once it has.
    fn broadcast_cancel(&self, job_id: &str) -> Result<()>;
    fn wait_until_proofman_ready(&self);
    /// The rank of a peer that has replied it stopped, if a reply has arrived.
    fn try_receive_cancel_done(&self) -> Option<i32>;
}

impl<T: ZiskBackend + 'static> RecoveryActions for ZiskProver<T> {
    fn n_processes(&self) -> i32 {
        ZiskProver::n_processes(self)
    }
    fn notify_cluster_cancellation(&self) {
        ZiskProver::notify_cluster_cancellation(self)
    }
    fn broadcast_cancel(&self, job_id: &str) -> Result<()> {
        let mut message = crate::worker::cancel_broadcast(job_id)?;
        self.mpi_broadcast(&mut message)
    }
    fn wait_until_proofman_ready(&self) {
        ZiskProver::wait_until_proofman_ready(self)
    }
    fn try_receive_cancel_done(&self) -> Option<i32> {
        self.mpi_try_receive(CANCEL_DONE_TAG).map(|(rank, _)| rank)
    }
}

/// Rank 0's first step in a recovery: tell every peer rank to stop `job_id`'s
/// work, before waiting on anything, so that they stop while this rank does.
///
/// Each peer then cancels its prover, waits for its task, ends the job, and replies
/// once its prover is idle; see `Worker::stop_cancelled_job`. A peer does not
/// recover on its own: a failure on one rank reaches rank 0 through the job's
/// collectives, and rank 0's recovery is what stops them all.
pub(crate) fn begin_cluster_recovery<R: RecoveryActions + ?Sized>(
    prover: &R,
    job_id: &str,
) -> Result<()> {
    prover.notify_cluster_cancellation();
    if prover.n_processes() > 1 {
        prover.broadcast_cancel(job_id)?;
    }
    Ok(())
}

/// The rest of rank 0's recovery: wait until this rank's prover is idle and every
/// peer rank has replied that it has stopped, giving up at `deadline`.
///
/// The replies stand in for a barrier: a barrier could meet one a peer was still
/// waiting in for the job, leaving the ranks out of step for every job after.
pub(crate) fn finish_cluster_recovery<R: RecoveryActions + ?Sized>(
    prover: &R,
    deadline: Instant,
) -> Result<()> {
    prover.wait_until_proofman_ready();
    let mut waiting: BTreeSet<i32> = (1..prover.n_processes()).collect();
    while !waiting.is_empty() {
        match prover.try_receive_cancel_done() {
            Some(rank) => {
                waiting.remove(&rank);
            }
            None if Instant::now() >= deadline => {
                let waiting: Vec<_> = waiting.into_iter().collect();
                return Err(anyhow!("ranks {waiting:?} did not stop the cancelled job's work"));
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    Ok(())
}

/// Run [`finish_cluster_recovery`] off the async runtime, giving up after
/// `timeout`, and say why it failed if it did. A handshake that gives up may leave
/// its thread waiting for the prover.
async fn run_recovery_within<R: RecoveryActions + Send + Sync + 'static>(
    prover: Arc<R>,
    timeout: Duration,
) -> std::result::Result<(), String> {
    let deadline = Instant::now() + timeout;
    let handshake =
        tokio::task::spawn_blocking(move || finish_cluster_recovery(&*prover, deadline));
    match tokio::time::timeout(timeout, handshake).await {
        Ok(Ok(Ok(()))) => Ok(()),
        Ok(Ok(Err(e))) => Err(format!("cluster handshake failed: {e:#}")),
        Ok(Err(e)) => Err(format!("recovery task panicked: {e}")),
        Err(_) => Err(format!("cluster handshake timed out after {timeout:?}")),
    }
}

/// The exit status of a worker that ends itself because it cannot recover
/// in-process: `EX_TEMPFAIL`, since a restart is what recovers it.
pub(crate) const WEDGED_EXIT_CODE: i32 = 75;

/// End the worker, every MPI rank of it, because it cannot recover in-process.
///
/// Once ranks may be in different MPI collectives, nothing in the process resumes
/// them, and a rank that exits normally waits in proofman's teardown barrier: only
/// aborting the MPI job ends them all. The service manager then restarts the
/// worker, as after any failed exit, and the coordinator, seeing it disconnect, no
/// longer waits for its recovery.
fn abort_wedged_worker<T: ZiskBackend + 'static>(
    prover: &ZiskProver<T>,
    reason: ExitReason<'_>,
    why: &str,
) -> ! {
    if stop::is_stopping() {
        // A stop kills the services under the job in flight, which may fail it on the way
        // out; the stop ends every rank by itself, and an abort would only fail its exit.
        warn!("{why}; the worker is stopping");
        stop::wait_for_exit();
    }
    telemetry::record_exit(prover.world_rank(), reason);
    error!("{why}; aborting the worker so that it restarts");
    // Also on stderr, unbuffered: a log file's writer thread may never write the line.
    eprintln!("zisk-worker: {why}; aborting the worker so that it restarts");
    prover.abort_cluster(WEDGED_EXIT_CODE)
}

/// Make a panic anywhere in the process call `abort`, once the panic is reported.
///
/// For a worker of several MPI ranks: a rank that panics leaves the others waiting
/// in whichever collective they had reached. A witness thread's panic, for one,
/// never reaches the collective where proofman's ranks agree on a failure, so they
/// wait for good; see [`abort_wedged_worker`]. On one rank there is no collective
/// to wait in, and a task's panic is caught and reported as its failure.
fn abort_on_panic(abort: impl Fn(&str, &str) + Send + Sync + 'static) {
    let report = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        report(info);
        let location =
            info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        abort("a panic left the other MPI ranks waiting for this one", &location);
        // `abort` does not return; should it, the panic must still not unwind.
        std::process::abort();
    }));
}

/// On a worker of several MPI ranks, a panic anywhere ends every rank; see
/// [`abort_on_panic`].
fn abort_every_rank_on_panic<T: ZiskBackend + 'static>(worker: &Worker<T>) {
    let prover = worker.prover_arc();
    if prover.n_processes() > 1 {
        abort_on_panic(move |why, location| {
            abort_wedged_worker(&prover, ExitReason::Panic { location }, why)
        });
    }
}

/// A requested stop cancels the job in flight, as the coordinator's cancel does; see
/// [`crate::stop`].
fn cancel_job_on_stop<T: ZiskBackend + 'static>(worker: &Worker<T>) {
    let prover = worker.prover_arc();
    stop::cancel_on_stop(move || {
        if let Err(e) = prover.cancel() {
            warn!("Cancelling the job in flight on a stop failed: {e:#}");
        }
    });
}

/// A worker process, specialized by MPI role: rank 0 talks to the coordinator
/// over gRPC, secondary ranks only service MPI broadcasts.
// One per process: the rank-0 variant's extra configuration costs nothing.
#[allow(clippy::large_enum_variant)]
pub enum WorkerNode<T: ZiskBackend + 'static> {
    /// Rank-0 node with a gRPC connection to the coordinator.
    WorkerGrpc(WorkerNodeGrpc<T>),
    /// Secondary-rank node driven purely by MPI broadcasts.
    WorkerMpi(WorkerNodeMpi<T>),
}

impl<T: ZiskBackend + 'static> WorkerNode<T> {
    /// This node's global MPI rank.
    pub fn world_rank(&self) -> i32 {
        match self {
            WorkerNode::WorkerGrpc(worker) => worker.world_rank(),
            WorkerNode::WorkerMpi(worker) => worker.world_rank(),
        }
    }
}

impl<T: ZiskBackend + 'static> WorkerNode<T> {
    /// Build an emulator-backed node, choosing the gRPC or MPI variant by rank.
    pub async fn new_emu(
        worker_config: WorkerServiceConfig,
        prover_config: ProverConfig,
    ) -> Result<WorkerNode<Emu>> {
        let worker = Worker::<Emu>::new_emu(prover_config)?;
        abort_every_rank_on_panic(&worker);
        cancel_job_on_stop(&worker);

        if worker.local_rank() == 0 {
            Ok(WorkerNode::WorkerGrpc(WorkerNodeGrpc::<Emu>::new(worker_config, worker).await?))
        } else {
            Ok(WorkerNode::WorkerMpi(WorkerNodeMpi::<Emu>::new(worker).await?))
        }
    }

    /// Build an ASM-backed node, choosing the gRPC or MPI variant by rank.
    pub async fn new_asm(
        worker_config: WorkerServiceConfig,
        prover_config: ProverConfig,
    ) -> Result<WorkerNode<Asm>> {
        let worker = Worker::<Asm>::new_asm(prover_config)?;
        abort_every_rank_on_panic(&worker);
        cancel_job_on_stop(&worker);

        if worker.local_rank() == 0 {
            Ok(WorkerNode::WorkerGrpc(WorkerNodeGrpc::<Asm>::new(worker_config, worker).await?))
        } else {
            Ok(WorkerNode::WorkerMpi(WorkerNodeMpi::<Asm>::new(worker).await?))
        }
    }

    /// Run the node to completion (dispatches to the gRPC or MPI variant).
    pub async fn run(&mut self) -> Result<()> {
        match self {
            WorkerNode::WorkerGrpc(worker) => worker.run().await,
            WorkerNode::WorkerMpi(worker) => worker.run().await,
        }
    }
}

/// Secondary-rank worker node: has no coordinator connection and only services
/// MPI broadcasts from rank 0.
pub struct WorkerNodeMpi<T: ZiskBackend + 'static> {
    worker: Worker<T>,
}

impl<T: ZiskBackend + 'static> WorkerNodeMpi<T> {
    /// Wrap a worker as a secondary-rank MPI node.
    pub async fn new(worker: Worker<T>) -> Result<Self> {
        Ok(Self { worker })
    }

    /// This node's global MPI rank.
    pub fn world_rank(&self) -> i32 {
        self.worker.world_rank()
    }

    async fn run(&mut self) -> Result<()> {
        assert!(self.worker.local_rank() != 0, "WorkerMpi should not be run by rank 0");

        loop {
            // Non-rank 0 workers are executing inside a cluster and only receives MPI requests
            self.worker.handle_mpi_broadcast_request().await?;
        }
    }
}

/// Rank-0 worker node: maintains the gRPC connection to the coordinator and
/// drives the full job lifecycle (registration, heartbeats, task dispatch).
pub struct WorkerNodeGrpc<T: ZiskBackend + 'static> {
    worker_config: WorkerServiceConfig,
    worker: Worker<T>,
    /// Setups and tasks held back, in arrival order, so that a setup never runs
    /// during a job and no task runs during a setup: a setup starts a program's
    /// services, which rewrite the guest memory every program in the process shares,
    /// and broadcasts to the peer ranks a job is using. A held-back task's stream data
    /// is held back with it. See [`Self::run_deferred`].
    deferred: VecDeque<DeferredWork>,
    /// The setup running off the loop, until its `SetupProgramComplete` is handled.
    /// Kept apart from the job's computation, which it must neither replace nor be
    /// cancelled with.
    setup_in_flight: Option<JoinHandle<()>>,
    /// The recovery draining a cancelled job, which a setup waits for.
    recovery: Option<JoinHandle<()>>,
}

/// Work [`WorkerNodeGrpc`] holds back until the worker can take it.
enum DeferredWork {
    Setup(SetupProgram),
    /// Boxed: a task request is several times the size of the other variants.
    Task(Box<ExecuteTaskRequest>),
    /// Stream data for a task held back, which the coordinator relays from the moment it
    /// sends the task: it follows the task in.
    Stream(StreamData),
    Input(InputStreamData),
}

impl DeferredWork {
    /// The job a held-back task or its stream data belongs to.
    fn job_id(&self) -> Option<&str> {
        match self {
            DeferredWork::Setup(_) => None,
            DeferredWork::Task(task) => Some(&task.job_id),
            DeferredWork::Stream(data) => Some(&data.job_id),
            DeferredWork::Input(data) => Some(&data.job_id),
        }
    }
}

/// A drain's reporting fields, and whether it was a drain and the job's final one.
struct AggOutcome {
    executed_steps: u64,
    proof_type: ProofKind,
    instances: u64,
    is_final: bool,
    drain: bool,
}

impl<T: ZiskBackend + 'static> WorkerNodeGrpc<T> {
    /// Wrap a worker as the rank-0 gRPC node with the given config.
    pub async fn new(worker_config: WorkerServiceConfig, worker: Worker<T>) -> Result<Self> {
        if worker_config.telemetry.enabled {
            telemetry::start_sampling(Duration::from_secs(
                worker_config.telemetry.interval_seconds.max(1),
            ));
        }
        Ok(Self {
            worker_config,
            worker,
            deferred: VecDeque::new(),
            setup_in_flight: None,
            recovery: None,
        })
    }

    /// This node's global MPI rank.
    pub fn world_rank(&self) -> i32 {
        self.worker.world_rank()
    }

    /// Connect to the coordinator and run the worker event loop until shutdown.
    pub async fn run(&mut self) -> Result<()> {
        assert!(self.worker.local_rank() == 0, "WorkerNodeGrpc should only be run by rank 0");

        // Process-long channel: tasks scheduled before a stream drop must
        // still be deliverable on the next reconnect.
        let (raw_tx, mut loop_rx) = mpsc::unbounded_channel::<LoopEvent>();
        let loop_tx = LoopEventSender::new(raw_tx);

        loop {
            match self.worker.state() {
                WorkerState::Disconnected => {
                    if let Err(e) = self.connect_and_run(&loop_tx, &mut loop_rx).await {
                        error!("Connection failed: {}", e);
                        tokio::time::sleep(Duration::from_secs(
                            self.worker_config.connection.reconnect_interval_seconds,
                        ))
                        .await;
                    }
                }
                WorkerState::Error => {
                    error!("Worker in error state, attempting to reconnect");
                    self.worker.set_state(WorkerState::Disconnected);
                    tokio::time::sleep(Duration::from_secs(
                        self.worker_config.connection.reconnect_interval_seconds,
                    ))
                    .await;
                }
                state => {
                    // Not an Ok: under `mpirun` with `orte_allowed_exit_without_sync`, a rank 0
                    // ending with status 0 would leave the peer ranks waiting for it for ever.
                    return Err(anyhow!("the worker left its connection loop in state {state:?}"));
                }
            }
        }
    }

    async fn connect_and_run(
        &mut self,
        loop_tx: &LoopEventSender,
        loop_rx: &mut mpsc::UnboundedReceiver<LoopEvent>,
    ) -> Result<()> {
        info!("Connecting to coordinator at {}", self.worker_config.coordinator.url);

        let channel =
            Channel::from_shared(self.worker_config.coordinator.url.clone())?.connect().await?;
        let mut client = zisk_distributed_api_client::ZiskDistributedApiClient::new(channel)
            .max_decoding_message_size(MAX_MESSAGE_SIZE)
            .max_encoding_message_size(MAX_MESSAGE_SIZE);

        // Create bidirectional stream
        let (message_sender, message_receiver) = mpsc::unbounded_channel();
        let request_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(message_receiver);
        let request = Request::new(request_stream);

        let response = client.worker_stream(request).await?;
        let mut response_stream = response.into_inner();

        // Send initial registration
        let connect_message = if let Some(job) = self.worker.current_job() {
            WorkerMessage {
                payload: Some(worker_message::Payload::Reconnect(WorkerReconnectRequest {
                    worker_id: self.worker_config.worker.worker_id.as_string(),
                    compute_capacity: Some(self.worker_config.worker.compute_capacity.into()),
                    last_known_job_id: Some(job.lock().await.job_id.as_string()),
                    aggregation_arity: self.worker.aggregation_arity() as u32,
                })),
            }
        } else {
            WorkerMessage {
                payload: Some(worker_message::Payload::Register(WorkerRegisterRequest {
                    worker_id: self.worker_config.worker.worker_id.as_string(),
                    compute_capacity: Some(self.worker_config.worker.compute_capacity.into()),
                    aggregation_arity: self.worker.aggregation_arity() as u32,
                    previous_exit: self
                        .worker_config
                        .telemetry
                        .enabled
                        .then(telemetry::take_previous_exit)
                        .flatten(),
                })),
            }
        };

        message_sender.send(connect_message)?;
        self.worker.set_state(WorkerState::Connecting);
        // Tasks held back on the previous stream go: the coordinator settles any it had
        // dispatched. Setups stay, since it sends again only the setups it has finished,
        // and acking one twice is harmless.
        self.deferred.retain(|work| matches!(work, DeferredWork::Setup(_)));

        let telemetry = &self.worker_config.telemetry;
        let heartbeat_seconds =
            if telemetry.enabled { telemetry.interval_seconds.clamp(1, 30) } else { 30 };
        let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(heartbeat_seconds));

        // Main non-blocking event loop
        loop {
            // Take the computation handle out so select! can poll it directly:
            // happy path fires the channel arm (handle put back), panic/cancel
            // fires the handle arm. `biased` orders branches loop events >
            // coordinator messages > task health > heartbeat — a wakeup race
            // can still let the JoinHandle resolve before the channel poll
            // sees the result, so the `Ok(())` arm re-drains via `try_recv`.
            let mut computation_handle = self.worker.take_current_computation();
            tokio::select! {
                biased;

                // Highest priority: events from spawn_blocking / recovery driver
                Some(event) = loop_rx.recv() => {
                    match event {
                        LoopEvent::Computation(result) => {
                            if matches!(*result, ComputationResult::SetupProgramComplete { .. }) {
                                // A setup runs apart from the computation, which goes on.
                                if let Some(h) = computation_handle.take() {
                                    self.worker.set_current_computation(h);
                                }
                            } else {
                                // Happy path: task completed and sent its result.
                                // Drop the handle — no need to await it, the task already finished.
                                drop(computation_handle.take());
                            }
                            if let Err(e) = self.handle_computation_result(*result, &message_sender, loop_tx).await {
                                error!("Error handling computation result: {}", e);
                                self.report_computation_error(&message_sender, &e.to_string()).await;
                                break;
                            }
                        }
                        LoopEvent::RecoveryComplete(rc) => {
                            // Its last act: setups held back for it may start.
                            self.recovery = None;
                            // Every site that schedules the recovery driver
                            // clears `current_computation` first, so the handle
                            // is None here and `current_job`/`state` were
                            // already cleaned up. Just forward the message.
                            let rc_for_requeue = rc.clone();
                            let msg = WorkerMessage {
                                payload: Some(worker_message::Payload::RecoveryComplete(rc)),
                            };
                            if let Err(e) = message_sender.send(msg) {
                                warn!("Failed to forward WorkerRecoveryComplete: {e}; re-enqueueing");
                                // Put it back on the process-long channel so the
                                // next stream re-delivers; otherwise the coordinator
                                // stays parked in pending_recovery.
                                if let Err(re) = loop_tx.send_recovery_complete(rc_for_requeue) {
                                    error!("Failed to re-enqueue WorkerRecoveryComplete: {re}");
                                }
                                break;
                            }
                        }
                    }
                }
                // Coordinator messages (task dispatch, cancellation, heartbeat, etc.)
                Some(result) = response_stream.next() => {
                    // Put the handle back before processing (coordinator message handler
                    // may need it, e.g. cancel_current_computation).
                    if let Some(h) = computation_handle.take() {
                        self.worker.set_current_computation(h);
                    }
                    match result {
                        Ok(message) => {
                            if let Err(e) = self.handle_coordinator_message(message, &message_sender, loop_tx).await {
                                error!("Error handling coordinator message: {}", e);
                                self.report_computation_error(&message_sender, &e.to_string()).await;
                                // Only truly fatal errors (e.g. registration rejected) set state
                                // to Error inside the handler — those break the loop and trigger
                                // reconnect. Recoverable errors (task dispatch failures, unknown
                                // task types, etc.) just reset the worker to Ready and keep the
                                // stream alive.
                                if matches!(self.worker.state(), WorkerState::Error) {
                                    break;
                                }
                                self.worker.set_current_job(None);
                                self.worker.set_state(WorkerState::Ready);
                            }
                        }
                        Err(e) => {
                            error!("Error receiving message from coordinator: {}", e);
                            break;
                        }
                    }
                }
                // Monitor the computation task handle directly. Fires on panic,
                // cancellation, or silent exit. `biased` checks loop_rx first,
                // but a cross-thread wakeup race can let the JoinHandle resolve before
                // the channel poll observes a just-sent event — so on `Ok(())` we
                // re-drain the channel before declaring a silent exit.
                join_result = async { computation_handle.as_mut().unwrap().await }, if computation_handle.is_some() => {
                    match join_result {
                        Err(join_error) => {
                            error!("Computation task failed unexpectedly: {}", join_error);
                            self.report_computation_error(&message_sender, &join_error.to_string()).await;
                            self.worker.set_current_job(None);
                            self.worker.set_state(WorkerState::Ready);
                        }
                        Ok(()) => {
                            match loop_rx.try_recv() {
                                Ok(LoopEvent::Computation(result)) => {
                                    if let Err(e) = self.handle_computation_result(*result, &message_sender, loop_tx).await {
                                        error!("Error handling computation result: {}", e);
                                        self.report_computation_error(&message_sender, &e.to_string()).await;
                                        break;
                                    }
                                }
                                Ok(LoopEvent::RecoveryComplete(rc)) => {
                                    self.recovery = None;
                                    let rc_for_requeue = rc.clone();
                                    let msg = WorkerMessage {
                                        payload: Some(worker_message::Payload::RecoveryComplete(rc)),
                                    };
                                    if let Err(e) = message_sender.send(msg) {
                                        warn!("Failed to forward WorkerRecoveryComplete: {e}; re-enqueueing");
                                        if let Err(re) = loop_tx.send_recovery_complete(rc_for_requeue) {
                                            error!("Failed to re-enqueue WorkerRecoveryComplete: {re}");
                                        }
                                        break;
                                    }
                                    self.worker.set_current_job(None);
                                    self.worker.set_state(WorkerState::Ready);
                                }
                                Err(_) => {
                                    warn!("Computation task exited without sending an event");
                                    self.worker.set_current_job(None);
                                    self.worker.set_state(WorkerState::Ready);
                                }
                            }
                        }
                    }
                }
                _ = heartbeat_interval.tick() => {
                    // Put the handle back before processing.
                    if let Some(h) = computation_handle.take() {
                        self.worker.set_current_computation(h);
                    }
                    if let Err(e) = self.send_heartbeat_ack(&message_sender).await {
                        error!("Error sending heartbeat: {}", e);
                        break;
                    }
                }
                else => {
                    info!("Stream closed, will reconnect");
                    break;
                }
            }

            self.run_deferred(&message_sender, loop_tx).await;
        }

        self.worker.set_state(WorkerState::Disconnected);
        Ok(())
    }

    /// Process a finished computation result: report it to the coordinator and
    /// advance or finalize the job.
    pub async fn handle_computation_result(
        &mut self,
        result: ComputationResult,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
        loop_tx: &LoopEventSender,
    ) -> Result<()> {
        match result {
            ComputationResult::Execution { job_id, success, result, task_received_time } => {
                self.send_execution(
                    job_id,
                    success,
                    result,
                    message_sender,
                    loop_tx,
                    task_received_time,
                )
                .await
            }
            ComputationResult::Contribution { job_id, success, result, task_received_time } => {
                self.send_partial_contribution(
                    job_id,
                    success,
                    result,
                    message_sender,
                    loop_tx,
                    task_received_time,
                )
                .await
            }
            ComputationResult::Proofs { job_id, success, result } => {
                self.send_proof(job_id, success, result, message_sender, loop_tx).await
            }
            ComputationResult::AggProof {
                job_id,
                success,
                result,
                executed_steps,
                proof_type,
                instances,
                final_proof,
                drain,
            } => {
                self.send_recurser(
                    job_id,
                    success,
                    result,
                    message_sender,
                    AggOutcome {
                        executed_steps,
                        proof_type,
                        instances,
                        is_final: final_proof,
                        drain,
                    },
                )
                .await
            }
            ComputationResult::RecurserAck { job_id, ack } => {
                // Ack already built by the blocking handler; just forward it.
                if let Err(e) = message_sender.send(ack) {
                    warn!("[Recurser] Failed to send recurser ack for job {job_id}: {e}");
                }
                self.worker.set_current_job(None);
                self.worker.set_state(WorkerState::Ready);
                Ok(())
            }
            ComputationResult::SetupProgramComplete {
                job_id,
                worker_id,
                hash_id,
                with_hints,
                emulator_only,
                result,
            } => {
                // Finish the off-loop setup: register the program (the `&mut self`
                // map inserts that couldn't run on the blocking thread) and forward
                // the ack. Does not touch worker state/current_job.
                self.setup_in_flight = None;
                let ack = match result {
                    Ok((program_vk, program)) => {
                        self.worker.register_setup(
                            &hash_id,
                            with_hints,
                            emulator_only,
                            program,
                            program_vk.clone(),
                        );
                        info!("[Setup] job_id {} Completed setup for hash_id {}", job_id, hash_id);
                        telemetry::mark_ready();
                        Self::build_setup_program_ack(
                            job_id.as_string(),
                            worker_id,
                            hash_id,
                            Ok(&program_vk),
                        )
                    }
                    Err(e) => {
                        error!(
                            "[Setup] job_id {} Failed setup for hash_id {}: {}",
                            job_id, hash_id, e
                        );
                        Self::build_setup_program_ack(
                            job_id.as_string(),
                            worker_id,
                            hash_id,
                            Err(e.to_string()),
                        )
                    }
                };
                if let Err(e) = message_sender.send(ack) {
                    warn!("Failed to send SetupProgramAck: {}", e);
                }
                Ok(())
            }
        }
    }

    /// Sends a [`WorkerError`] message to the coordinator when a computation task
    /// fails unexpectedly (e.g. panic inside `spawn_blocking`). This ensures the
    /// coordinator learns about the failure immediately instead of waiting for a
    /// heartbeat timeout.
    async fn report_computation_error(
        &self,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
        error_message: &str,
    ) {
        let job_id = match self.worker.current_job() {
            Some(job) => job.lock().await.job_id.as_string(),
            None => {
                // No current job — nothing useful to report to coordinator
                warn!(
                    "Computation error without active job (not reported to coordinator): {}",
                    error_message
                );
                return;
            }
        };

        let message = WorkerMessage {
            payload: Some(worker_message::Payload::Error(WorkerError {
                worker_id: self.worker_config.worker.worker_id.as_string(),
                job_id,
                error_message: error_message.to_string(),
            })),
        };

        if let Err(e) = message_sender.send(message) {
            error!("Failed to send WorkerError to coordinator: {}", e);
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn send_partial_contribution(
        &mut self,
        job_id: JobId,
        success: bool,
        result: Result<(
            WitnessInfo,
            ZiskExecutorTime,
            Vec<ContributionsInfo>,
            u64,
            StatsCostPerType,
        )>,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
        loop_tx: &LoopEventSender,
        task_received_time: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<()> {
        if let Some(handle) = self.worker.take_current_computation() {
            handle.await?;
        }

        let (result_data, error_message) = match result {
            Ok(data) => {
                if !success {
                    return Err(anyhow!(
                        "Inconsistent state: operation reported failure but returned Ok result"
                    ));
                }
                (data, String::new())
            }
            Err(e) => {
                if success {
                    return Err(anyhow!(
                        "Inconsistent state: operation reported success but returned Err result"
                    ));
                }
                (
                    (
                        WitnessInfo::default(),
                        ZiskExecutorTime::default(),
                        vec![],
                        0,
                        StatsCostPerType::default(),
                    ),
                    e.to_string(),
                )
            }
        };

        let challenges: Vec<Challenges> = result_data
            .2
            .into_iter()
            .map(|cont| Challenges {
                worker_index: cont.worker_index,
                airgroup_id: cont.airgroup_id as u32,
                challenge: cont.challenge.to_vec(),
            })
            .collect();

        let witness_info = WitnessExecInfo {
            witness_time: result_data.0.witness_time,
            publics: result_data.0.publics,
            proof_values: result_data.0.proof_values,
            summary_info: result_data.0.summary_info,
            total_instances: result_data.3,
        };

        let zisk_execution_time = ZiskExecuteTime {
            total_duration: result_data.1.total_duration as f32,
            execution_duration: result_data.1.execution_duration as f32,
            count_and_plan_duration: result_data.1.count_and_plan_duration as f32,
            count_and_plan_mo_duration: result_data.1.count_and_plan_mo_duration as f32,
            asm_execution_duration: result_data
                .1
                .asm_execution_duration
                .map(|asm_info| AsmExecuteInfo { time: asm_info.time, mhz: asm_info.mhz }),
            task_received_time: task_received_time
                .unwrap_or_else(chrono::Utc::now)
                .timestamp_millis() as f64,
        };

        let task_type = TaskType::PartialContribution as i32;
        let result_data_msg = Some(ResultData::Challenges(ChallengesList {
            challenges,
            witness_info: Some(witness_info),
            zisk_execution_time: Some(zisk_execution_time),
            cost_per_type: Some(result_data.4.into()),
        }));

        let worker_in_recovery = !success && self.owns_recovery_for(&job_id).await;

        let message = WorkerMessage {
            payload: Some(worker_message::Payload::ExecuteTaskResponse(ExecuteTaskResponse {
                worker_id: self.worker_config.worker.worker_id.as_string(),
                job_id: job_id.as_string(),
                task_type,
                success,
                result_data: result_data_msg,
                error_message,
                worker_in_recovery,
            })),
        };

        message_sender.send(message)?;

        if worker_in_recovery {
            // Compute task already awaited at the top of this handler; nothing
            // left to drain, so pass no handle.
            self.spawn_post_failure_recovery(
                loop_tx.clone(),
                CancelledWork::default(),
                job_id.as_string(),
            );
            // Drop `current_job` so a coordinator-originated `JobCancelled`
            // for the same job_id does not race a still-running
            // `spawn_post_failure_recovery` and emit a premature
            // `WorkerRecoveryComplete`.
            self.worker.set_current_job(None);
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn send_execution(
        &mut self,
        job_id: JobId,
        success: bool,
        #[allow(clippy::type_complexity)] result: Result<(
            WitnessInfo,
            ZiskExecutorTime,
            u64,
            u64,
            StatsCostPerType,
            Vec<zisk_common::AirInstanceCount>,
        )>,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
        loop_tx: &LoopEventSender,
        task_received_time: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<()> {
        if let Some(handle) = self.worker.take_current_computation() {
            handle.await?;
        }

        let (result_data, error_message) = match result {
            Ok(data) => {
                if !success {
                    return Err(anyhow!(
                        "Inconsistent state: operation reported failure but returned Ok result"
                    ));
                }
                (data, String::new())
            }
            Err(e) => {
                if success {
                    return Err(anyhow!(
                        "Inconsistent state: operation reported success but returned Err result"
                    ));
                }
                (
                    (
                        WitnessInfo::default(),
                        ZiskExecutorTime::default(),
                        0,
                        0,
                        StatsCostPerType::default(),
                        Vec::new(),
                    ),
                    e.to_string(),
                )
            }
        };

        let (witness_info, zisk_exec_time, instances, executed_steps, cost_per_type, plan) =
            result_data;

        let witness_info_msg = WitnessExecInfo {
            witness_time: witness_info.witness_time,
            publics: witness_info.publics,
            proof_values: witness_info.proof_values,
            summary_info: witness_info.summary_info,
            total_instances: instances,
        };

        let zisk_execution_time = ZiskExecuteTime {
            total_duration: zisk_exec_time.total_duration as f32,
            execution_duration: zisk_exec_time.execution_duration as f32,
            count_and_plan_duration: zisk_exec_time.count_and_plan_duration as f32,
            count_and_plan_mo_duration: zisk_exec_time.count_and_plan_mo_duration as f32,
            asm_execution_duration: zisk_exec_time
                .asm_execution_duration
                .map(|asm_info| AsmExecuteInfo { time: asm_info.time, mhz: asm_info.mhz }),
            task_received_time: task_received_time
                .unwrap_or_else(chrono::Utc::now)
                .timestamp_millis() as f64,
        };

        let task_type = TaskType::Execution as i32;
        let result_data_msg = Some(ResultData::Execution(Execution {
            instances,
            executed_steps,
            zisk_execution_time: Some(zisk_execution_time),
            witness_info: Some(witness_info_msg),
            cost_per_type: Some(cost_per_type.into()),
            plan: plan
                .into_iter()
                .map(|p| AirInstanceCount {
                    airgroup_id: p.airgroup_id as u32,
                    air_id: p.air_id as u32,
                    count: p.count,
                })
                .collect(),
        }));

        let worker_in_recovery = !success && self.owns_recovery_for(&job_id).await;

        let message = WorkerMessage {
            payload: Some(worker_message::Payload::ExecuteTaskResponse(ExecuteTaskResponse {
                worker_id: self.worker_config.worker.worker_id.as_string(),
                job_id: job_id.as_string(),
                task_type,
                success,
                result_data: result_data_msg,
                error_message,
                worker_in_recovery,
            })),
        };

        message_sender.send(message)?;
        // Its results are out: the execution was the job's only phase.
        self.worker.end_job();

        if worker_in_recovery {
            // Compute task already awaited at the top of this handler.
            self.spawn_post_failure_recovery(
                loop_tx.clone(),
                CancelledWork::default(),
                job_id.as_string(),
            );
            // See `send_partial_contribution`.
            self.worker.set_current_job(None);
        }

        Ok(())
    }

    async fn send_proof(
        &mut self,
        job_id: JobId,
        success: bool,
        result: Result<Vec<AggProofs>>,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
        loop_tx: &LoopEventSender,
    ) -> Result<()> {
        if let Some(handle) = self.worker.take_current_computation() {
            handle.await?;
        }

        let (result_data, error_message) = match result {
            Ok(data) => {
                if !success {
                    return Err(anyhow!(
                        "Inconsistent state: Prove reported failure but returned Ok result"
                    ));
                }
                (
                    data.into_iter()
                        .map(|v| ProofStark {
                            airgroup_id: v.airgroup_id,
                            values: v.proof,
                            worker_indexes: v.worker_indexes.iter().map(|&i| i as u32).collect(),
                        })
                        .collect(),
                    String::new(),
                )
            }
            Err(e) => {
                if success {
                    return Err(anyhow!(
                        "Inconsistent state: Prove reported success but returned Err result"
                    ));
                }
                (vec![], e.to_string())
            }
        };

        let worker_in_recovery = !success && self.owns_recovery_for(&job_id).await;

        let message = WorkerMessage {
            payload: Some(worker_message::Payload::ExecuteTaskResponse(ExecuteTaskResponse {
                worker_id: self.worker_config.worker.worker_id.as_string(),
                job_id: job_id.as_string(),
                task_type: TaskType::Prove as i32,
                success,
                result_data: Some(ResultData::Proofs(ProofList { proofs: result_data })),
                error_message,
                worker_in_recovery,
            })),
        };

        message_sender.send(message)?;
        // The proofs are out, and what may follow, aggregation, does not use the
        // prover's shared memory.
        self.worker.end_job();

        if worker_in_recovery {
            // Compute task already awaited at the top of this handler.
            self.spawn_post_failure_recovery(
                loop_tx.clone(),
                CancelledWork::default(),
                job_id.as_string(),
            );
            // See `send_partial_contribution`.
            self.worker.set_current_job(None);
        }

        Ok(())
    }

    /// Return a node's folded subtree (it stays resident here).
    async fn send_partial_agg(
        &mut self,
        job_id: JobId,
        folded: Vec<AggProofs>,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
    ) -> Result<()> {
        let proofs = folded
            .into_iter()
            .map(|p| ProofStark {
                airgroup_id: p.airgroup_id,
                values: p.proof,
                worker_indexes: p.worker_indexes.iter().map(|&i| i as u32).collect(),
            })
            .collect();

        let message = WorkerMessage {
            payload: Some(worker_message::Payload::ExecuteTaskResponse(ExecuteTaskResponse {
                worker_id: self.worker_config.worker.worker_id.as_string(),
                job_id: job_id.as_string(),
                task_type: TaskType::Aggregate as i32,
                success: true,
                result_data: Some(ResultData::PartialAggProofs(ProofList { proofs })),
                error_message: String::new(),
                worker_in_recovery: false,
            })),
        };
        message_sender.send(message)?;

        info!("Folded subtree returned for {job_id}");
        Ok(())
    }

    async fn send_recurser(
        &mut self,
        job_id: JobId,
        success: bool,
        result: Result<Option<Vec<AggProofs>>>,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
        outcome: AggOutcome,
    ) -> Result<()> {
        let AggOutcome { executed_steps, proof_type, instances, is_final, drain } = outcome;
        if let Some(handle) = self.worker.take_current_computation() {
            handle.await?;
        }

        let mut error_message = String::new();
        let mut reset_current_job = false;
        // Shadowed so a failure while building the response can flip it. An empty
        // `proof_data` with `success` still true reads to the coordinator as an
        // intermediate ack, which it rejects outright on the final task.
        let mut success = success;

        let result_data = match result {
            Ok(data) => {
                if !success {
                    return Err(anyhow!("Aggregation returned Ok result but reported failure"));
                }

                // Only the root's drain becomes a job proof.
                if !is_final {
                    match data {
                        Some(folded) => {
                            return self.send_partial_agg(job_id, folded, message_sender).await
                        }
                        // Sent as an absorb ack, the coordinator would wait for the drain forever.
                        None if drain => {
                            return Err(anyhow!("aggregation drain returned no proof"))
                        }
                        None => {}
                    }
                }

                if let Some(final_proof) = data {
                    let final_proof: Vec<Vec<u64>> =
                        final_proof.into_iter().map(|p| p.proof).collect();
                    reset_current_job = !final_proof.is_empty();

                    let proof_data = if !final_proof.is_empty() {
                        let is_plonk = proof_type == ProofKind::Plonk;
                        let flat_proof: Vec<u64> = final_proof.into_iter().flatten().collect();
                        // Compression strips the flag that marks this a fold, taking the
                        // recursion-domain check in `Proof::verify` with it.
                        if !fold_allows_kind(proof_type) {
                            fail_aggregation(
                                &mut success,
                                &mut error_message,
                                format!(
                                    "refusing to return the aggregated proof for {job_id} as \
                                     {proof_type:?}: compression drops the recursion-domain \
                                     marker"
                                ),
                            )
                        } else {
                            // A missing verkey or hash family yields an unusable proof
                            // (new_from_vadcop_proof rejects an unrecognized/empty hash).
                            // Treat it as a hard failure rather than emitting a "success"
                            // response carrying empty proof_data.
                            // `fold_allows_kind` above leaves only uncompressed kinds here.
                            let verkey = self
                                .worker
                                .get_vadcop_vk(false)
                                .context("Failed to get vadcop verification key")?;
                            let hash = self
                                .worker
                                .hash()
                                .context("Failed to get proving-key hash family")?;
                            match Proof::new_from_vadcop_proof(&flat_proof, false, verkey, hash) {
                                Ok(zisk_proof) => {
                                    // On wrap failure, emit nothing: returning the
                                    // unwrapped Vadcop proof would answer a PLONK request
                                    // with a different proof kind.
                                    let final_proof: Option<Proof> = if is_plonk {
                                        match self
                                            .worker
                                            .prover_arc()
                                            .wrap_proof(&zisk_proof, ProofKind::Plonk)
                                            .run()
                                        {
                                            Ok(wrapped) => Some(wrapped.get_proof().clone()),
                                            Err(e) => {
                                                fail_aggregation(
                                                    &mut success,
                                                    &mut error_message,
                                                    format!(
                                                        "Failed to wrap Plonk proof for \
                                                         {job_id}: {e}"
                                                    ),
                                                );
                                                None
                                            }
                                        }
                                    } else {
                                        Some(zisk_proof)
                                    };
                                    match final_proof {
                                        Some(p) => bincode::serde::encode_to_vec(
                                            &p,
                                            bincode::config::standard(),
                                        )
                                        .unwrap_or_default(),
                                        None => vec![],
                                    }
                                }
                                Err(e) => fail_aggregation(
                                    &mut success,
                                    &mut error_message,
                                    format!("Failed to build Proof: {e}"),
                                ),
                            }
                        }
                    } else {
                        vec![]
                    };

                    Some(ResultData::FinalProof(FinalProof {
                        proof_data,
                        executed_steps,
                        instances,
                    }))
                } else {
                    Some(ResultData::FinalProof(FinalProof {
                        proof_data: vec![],
                        executed_steps,
                        instances,
                    }))
                }
            }
            Err(e) => {
                if success {
                    return Err(anyhow!("Aggregation returned Err but reported success"));
                }
                error_message = e.to_string();
                Some(ResultData::FinalProof(FinalProof {
                    proof_data: vec![],
                    executed_steps,
                    instances,
                }))
            }
        };

        let message = WorkerMessage {
            payload: Some(worker_message::Payload::ExecuteTaskResponse(ExecuteTaskResponse {
                worker_id: self.worker_config.worker.worker_id.as_string(),
                job_id: job_id.as_string(),
                task_type: TaskType::Aggregate as i32,
                success,
                result_data,
                error_message,
                worker_in_recovery: false,
            })),
        };

        message_sender.send(message)?;

        // TODO! move this logic to the client
        if reset_current_job {
            info!("Aggregation task completed for {}", job_id);
            self.worker.set_current_job(None);
            self.worker.set_state(WorkerState::Ready);
        }

        Ok(())
    }

    /// `true` when this failure should drive recovery for `job_id`. Returns
    /// `false` if a `JobCancelled` handler has already cleared `current_job`
    /// (recovery is in flight there) or if the worker has already moved on
    /// to another job. Without this guard, rank 0 recovers twice from the same
    /// cancelled job, and its second recovery waits for replies the peer ranks,
    /// told to stop once, never send.
    async fn owns_recovery_for(&self, job_id: &JobId) -> bool {
        match self.worker.current_job() {
            Some(job) => job.lock().await.job_id == *job_id,
            None => false,
        }
    }

    /// Drive the cluster recovery handshake for `job_id` (tell the peer ranks to
    /// stop its work, wait for this rank's computation and prover to drain, then for
    /// every peer's reply), then signal `WorkerRecoveryComplete`. The ASM soft reset
    /// itself runs inside `executor::execute`'s Err arm — this task is only the
    /// post-cancel sync. A handshake that fails, or does not complete within
    /// `RECOVERY_TIMEOUT`, aborts the worker; see [`abort_wedged_worker`].
    fn spawn_post_failure_recovery(
        &mut self,
        loop_tx: LoopEventSender,
        cancelled: CancelledWork,
        job_id: String,
    ) {
        let prover = self.worker.prover_arc();
        let worker_id = self.worker_config.worker.worker_id.as_string();
        self.recovery = Some(tokio::spawn(async move {
            warn!("[Recovery] {worker_id}: running cluster cancellation handshake for {job_id}");

            let begun = {
                let prover = Arc::clone(&prover);
                tokio::task::spawn_blocking(move || begin_cluster_recovery(&*prover, &job_id)).await
            };
            match begun {
                Ok(Ok(())) => {}
                Ok(Err(e)) => abort_wedged_worker(
                    &prover,
                    ExitReason::RecoveryFailed,
                    &format!(
                        "[Recovery] {worker_id}: telling the peer ranks to stop failed: {e:#}"
                    ),
                ),
                Err(e) => abort_wedged_worker(
                    &prover,
                    ExitReason::RecoveryFailed,
                    &format!(
                        "[Recovery] {worker_id}: telling the peer ranks to stop panicked: {e}"
                    ),
                ),
            }

            // Await the in-flight compute task before the barrier: its ASM MO
            // run is a synchronous round-trip, so when it returns the child has
            // reset and stopped touching the shmem the next job reuses. Bounded
            // so a stuck task can't wedge recovery.
            let recovery_context = format!("[Recovery] {worker_id}");
            Self::drain_cancelled_computation(cancelled.compute, &recovery_context).await;
            // Drain the ordering-thread shutdown on the same path, so a live
            // computation's cancel never leaves the stream half un-joined.
            Self::drain_cancelled_computation(cancelled.stream, &recovery_context).await;

            match run_recovery_within(Arc::clone(&prover), Self::RECOVERY_TIMEOUT).await {
                Ok(()) => {
                    info!("[Recovery] {worker_id}: cluster handshake done; signalling Ready");
                    let msg = WorkerRecoveryComplete { worker_id: worker_id.clone() };
                    if let Err(e) = loop_tx.send_recovery_complete(msg) {
                        error!("[Recovery] {worker_id}: enqueue RecoveryComplete failed: {e}");
                    }
                }
                Err(why) => {
                    let reason = if why.contains("timed out") {
                        ExitReason::RecoveryTimeout
                    } else {
                        ExitReason::RecoveryFailed
                    };
                    abort_wedged_worker(&prover, reason, &format!("[Recovery] {worker_id}: {why}"))
                }
            }
        }));
    }

    /// Drain a stream-ordering shutdown task off the event loop (no-op if `None`).
    /// Used on cancel paths with no live compute task (so no full recovery
    /// handshake), where a stream thread may still be shutting down and must be
    /// joined before the next job's reset.
    fn spawn_stream_drain(&self, stream: Option<JoinHandle<()>>) {
        if stream.is_none() {
            return;
        }
        let context =
            format!("[cancel-stream] {}", self.worker_config.worker.worker_id.as_string());
        tokio::spawn(async move {
            Self::drain_cancelled_computation(stream, &context).await;
        });
    }

    /// Drain cancelled job `job_id`'s in-flight work off the event loop. If a
    /// compute task was live, run the full recovery handshake (stops the peer
    /// ranks, drains compute + stream, emits `WorkerRecoveryComplete`) and return
    /// `true`. Otherwise just drain any stream-ordering shutdown and return `false`
    /// — the caller decides whether a completion signal still needs emitting.
    fn drive_cancellation(
        &mut self,
        loop_tx: &LoopEventSender,
        cancelled: CancelledWork,
        job_id: String,
    ) -> bool {
        if cancelled.compute.is_some() {
            self.spawn_post_failure_recovery(loop_tx.clone(), cancelled, job_id);
            true
        } else {
            self.spawn_stream_drain(cancelled.stream);
            false
        }
    }

    /// Builds a `SetupProgramAck`: `Ok(vk)` → success with the encoded VK,
    /// `Err(msg)` → failure with the error message. Shared by the setup fast
    /// path, the off-loop completion, and the reconnection setup path.
    fn build_setup_program_ack(
        job_id: String,
        worker_id: String,
        hash_id: String,
        result: std::result::Result<&ProgramVK, String>,
    ) -> WorkerMessage {
        let (success, error_message, vk, hash_mode) = match result {
            Ok(program_vk) => (
                true,
                String::new(),
                program_vk.vk.iter().flat_map(|w| w.to_le_bytes()).collect(),
                program_vk.hash_mode.as_str().to_string(),
            ),
            Err(error_message) => (false, error_message, Vec::new(), String::new()),
        };
        WorkerMessage {
            payload: Some(worker_message::Payload::SetupProgramAck(SetupProgramAck {
                job_id,
                worker_id,
                hash_id,
                success,
                error_message,
                vk,
                hash_mode,
            })),
        }
    }

    /// Await a compute handle (bounded) so the ASM child finishes resetting
    /// before we reuse its shmem. No-op when nothing was running. `context`
    /// prefixes the log lines so callers can be told apart.
    async fn drain_cancelled_computation(handle: Option<JoinHandle<()>>, context: &str) {
        let Some(handle) = handle else { return };
        match tokio::time::timeout(Self::COMPUTE_DRAIN_TIMEOUT, handle).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => warn!("{context}: in-flight compute task join error: {e}"),
            Err(_) => warn!(
                "{context}: in-flight compute task did not drain within {:?}; proceeding — \
                 the ASM child may still be resetting",
                Self::COMPUTE_DRAIN_TIMEOUT
            ),
        }
    }

    /// Healthy reset is sub-second; this only fires when the prover is stuck.
    const RECOVERY_TIMEOUT: Duration = Duration::from_secs(300);

    /// Bound on awaiting the in-flight compute task; above the ASM child's ≤5 s
    /// reset-flag observation window. Past this we proceed rather than wedge.
    const COMPUTE_DRAIN_TIMEOUT: Duration = Duration::from_secs(10);

    async fn send_heartbeat_ack(
        &self,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
    ) -> Result<()> {
        let message = WorkerMessage {
            payload: Some(worker_message::Payload::HeartbeatAck(HeartbeatAck {
                worker_id: self.worker_config.worker.worker_id.as_string(),
                telemetry: self.worker_config.telemetry.enabled.then(telemetry::latest),
            })),
        };

        message_sender.send(message)?;

        Ok(())
    }

    async fn handle_coordinator_message(
        &mut self,
        message: CoordinatorMessage,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
        loop_tx: &LoopEventSender,
    ) -> Result<()> {
        if message.payload.is_none() {
            return Err(anyhow!("Received empty message from coordinator"));
        }

        match message.payload.unwrap() {
            coordinator_message::Payload::RegisterResponse(response) => {
                if response.accepted {
                    info!("Registration accepted: {}", response.message);
                    telemetry::mark_registered();

                    // If we clobbered a live computation, hand its work to the
                    // recovery driver so it awaits the drain before the barrier
                    // (else a new dispatch's `prover.reset()` races the unwinding
                    // task), then emits `WorkerRecoveryComplete`.
                    let mut cancelled_work = CancelledWork::default();
                    let stale_job_id = match self.worker.current_job() {
                        Some(job) => job.lock().await.job_id.as_string(),
                        None => String::new(),
                    };
                    match response.directive.map(|d| ReconnectionAction::try_from(d.action)) {
                        Some(Ok(ReconnectionAction::CancelStaleJob)) => {
                            info!("Coordinator directed cancellation of stale job");
                            cancelled_work = self.worker.clear_current_job();
                        }
                        Some(Ok(ReconnectionAction::KeepComputing)) => {
                            info!("Coordinator confirmed active job; keep computing");
                        }
                        Some(Ok(ReconnectionAction::Idle)) | None => {
                            if self.worker.current_job().is_some() {
                                warn!("No cancel directive but worker has stale job; clearing");
                                cancelled_work = self.worker.clear_current_job();
                            }
                        }
                        Some(Err(_)) => {
                            warn!(
                                "Unknown reconciliation action; clearing stale state defensively"
                            );
                            cancelled_work = self.worker.clear_current_job();
                        }
                    }
                    // Drain the clobbered job's work off the loop (full recovery
                    // handshake if a compute task was live, else just the stream
                    // shutdown). Reconnect never emits WorkerRecoveryComplete itself.
                    self.drive_cancellation(loop_tx, cancelled_work, stale_job_id);

                    // The coordinator attaches a setup to every reconnection. It goes
                    // the way of any other: off the loop, once no job, computation or
                    // recovery is left, and ahead of any task that arrives meanwhile.
                    if let Some(setup) = response.setup_program {
                        self.handle_setup_request(setup, message_sender, loop_tx);
                    }

                    self.worker.set_state(WorkerState::Ready);
                } else {
                    self.worker.set_state(WorkerState::Error);
                    error!("Registration rejected: {}", response.message);
                    return Err(anyhow!("Registration rejected: {}", response.message));
                }
            }
            coordinator_message::Payload::ExecuteTask(request) => {
                // A task waits for a setup in flight, and for whatever was held back before
                // it, unless it continues the current job, which setups wait for.
                let continues_current_job = match self.worker.current_job() {
                    Some(job) => job.lock().await.job_id.as_string() == request.job_id,
                    None => false,
                };
                if self.setup_in_flight.is_some()
                    || (!self.deferred.is_empty() && !continues_current_job)
                {
                    info!("Deferring task {} until the setup in flight completes", request.job_id);
                    self.deferred.push_back(DeferredWork::Task(Box::new(request)));
                } else {
                    self.dispatch_task(request, message_sender, loop_tx).await;
                }
            }
            coordinator_message::Payload::StreamData(stream_data) => {
                if self.holds_back_job(&stream_data.job_id) {
                    self.deferred.push_back(DeferredWork::Stream(stream_data));
                } else {
                    self.handle_stream_data(stream_data).await?;
                }
            }
            coordinator_message::Payload::InputStreamData(input_data) => {
                if self.holds_back_job(&input_data.job_id) {
                    self.deferred.push_back(DeferredWork::Input(input_data));
                } else {
                    self.handle_input_stream_data(input_data).await?;
                }
            }
            coordinator_message::Payload::JobCancelled(cancelled) => {
                info!("Job {} cancelled: {}", cancelled.job_id, cancelled.reason);

                // Two outcomes once we've matched the cancelled job to ours:
                //  - a live computation → the recovery driver does the ASM
                //    soft-reset handshake and emits `WorkerRecoveryComplete` itself.
                //  - no live computation → nothing to soft-reset, but the
                //    coordinator parked us SettingUp awaiting `WorkerRecoveryComplete`,
                //    so we emit it directly below.
                // A returned compute handle is the authoritative "had a live
                // computation" signal — no separate state query needed.
                let mut matched = false;
                let mut cancelled_work = CancelledWork::default();
                // A task held back for the job never started: it, and its stream data, are
                // the job's to drop.
                if self.holds_back_job(&cancelled.job_id) {
                    matched = true;
                }
                self.deferred.retain(|work| work.job_id() != Some(cancelled.job_id.as_str()));
                if let Some(ref job) = self.worker.current_job() {
                    let cancelled_job_id = JobId::from(cancelled.job_id.clone());

                    if job.lock().await.job_id == cancelled_job_id {
                        cancelled_work = self.worker.clear_current_job();
                        self.worker.set_state(WorkerState::Ready);
                        matched = true;
                    }
                }

                let ack = WorkerMessage {
                    payload: Some(worker_message::Payload::JobCancelledAck(JobCancelledAck {
                        worker_id: self.worker_config.worker.worker_id.as_string(),
                        job_id: cancelled.job_id.clone(),
                    })),
                };
                if let Err(e) = message_sender.send(ack) {
                    warn!("Failed to send JobCancelledAck: {}", e);
                }

                // Drain the clobbered work off-loop; the recovery driver emits
                // WorkerRecoveryComplete when a compute task was live. Otherwise,
                // if we matched the job, emit it directly (coordinator parked us
                // SettingUp awaiting it).
                let recovery_spawned =
                    self.drive_cancellation(loop_tx, cancelled_work, cancelled.job_id);
                if !recovery_spawned && matched {
                    let rc = WorkerRecoveryComplete {
                        worker_id: self.worker_config.worker.worker_id.as_string(),
                    };
                    if let Err(e) = loop_tx.send_recovery_complete(rc) {
                        warn!("Failed to enqueue WorkerRecoveryComplete on cancel: {e}");
                    }
                }
            }
            coordinator_message::Payload::Heartbeat(_) => {
                // Send heartbeat ack
                self.send_heartbeat_ack(message_sender).await?;
            }
            coordinator_message::Payload::SetupProgram(setup) => {
                self.handle_setup_request(setup, message_sender, loop_tx);
            }
            coordinator_message::Payload::Shutdown(shutdown) => {
                info!(
                    "Coordinator shutdown: {} (grace period: {}s)",
                    shutdown.reason, shutdown.grace_period_seconds
                );
                tokio::time::sleep(Duration::from_secs(shutdown.grace_period_seconds as u64)).await;
                return Err(anyhow!("Coordinator requested shutdown: {}", shutdown.reason));
            }
            coordinator_message::Payload::SetupAggregationProgram(setup) => {
                // Run the (potentially minutes-long) setup off the message loop
                // so heartbeats keep flowing; the result is delivered back as a
                // `RecurserAck` LoopEvent and forwarded by `handle_computation_result`.
                let worker_id = self.worker_config.worker.worker_id.as_string();
                let job_id_str = setup.job_id.clone();
                let recurser_id = setup.recurser_id.clone();
                let job_id = JobId::from(job_id_str.clone());
                let tx = loop_tx.clone();

                self.worker.set_state(WorkerState::SettingUp);
                let prover = self.worker.prover_arc();
                let proving_key = self.worker.proving_key().to_path_buf();

                let handle = tokio::task::spawn_blocking(move || {
                    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        Self::handle_setup_aggregation_program(&prover, &proving_key, setup)
                    }));
                    let (success, error_message, vk, hash_mode) = match outcome {
                        Ok(Ok((vk, hash_mode))) => (true, String::new(), vk, hash_mode),
                        Ok(Err(e)) => {
                            error!(
                                "[Recurser] job_id {} Failed recurser setup for recurser_id {}: {}",
                                job_id_str, recurser_id, e
                            );
                            (false, e.to_string(), Vec::new(), String::new())
                        }
                        Err(_) => {
                            error!("[Recurser] job_id {job_id_str} recurser setup panicked");
                            (
                                false,
                                "recurser setup panicked".to_string(),
                                Vec::new(),
                                String::new(),
                            )
                        }
                    };
                    let ack = WorkerMessage {
                        payload: Some(worker_message::Payload::SetupAggregationProgramAck(
                            SetupAggregationProgramAck {
                                job_id: job_id_str.clone(),
                                worker_id,
                                recurser_id,
                                success,
                                error_message,
                                vk,
                                hash_mode,
                            },
                        )),
                    };
                    if tx.send_computation(ComputationResult::RecurserAck { job_id, ack }).is_err()
                    {
                        warn!("[Recurser] Failed to deliver setup ack: event loop channel closed");
                    }
                });
                self.worker.set_current_computation(handle);
            }
            coordinator_message::Payload::RunAggregateProofs(req) => {
                // Off the message loop, same as setup above.
                let worker_id = self.worker_config.worker.worker_id.as_string();
                let job_id_str = req.job_id.clone();
                let recurser_id = req.recurser_id.clone();
                let job_id = JobId::from(job_id_str.clone());
                let prover = self.worker.prover_arc();
                let tx = loop_tx.clone();

                self.worker.set_state(WorkerState::Computing((job_id.clone(), JobPhase::Recurse)));

                let handle = tokio::task::spawn_blocking(move || {
                    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        Self::handle_run_aggregate_proofs(&prover, req)
                    }));
                    let (success, error_message, proof) = match outcome {
                        Ok(Ok(proof_bytes)) => (true, String::new(), proof_bytes),
                        Ok(Err(e)) => {
                            error!(
                                "[Recurser] job_id {} Failed recurser prove for recurser_id {}: {}",
                                job_id_str, recurser_id, e
                            );
                            (false, e.to_string(), Vec::new())
                        }
                        Err(_) => {
                            error!("[Recurser] job_id {job_id_str} recurser prove panicked");
                            (false, "recurser prove panicked".to_string(), Vec::new())
                        }
                    };
                    let ack = WorkerMessage {
                        payload: Some(worker_message::Payload::RunAggregateProofsAck(
                            RunAggregateProofsAck {
                                job_id: job_id_str.clone(),
                                worker_id,
                                success,
                                error_message,
                                proof,
                            },
                        )),
                    };
                    if tx.send_computation(ComputationResult::RecurserAck { job_id, ack }).is_err()
                    {
                        warn!("[Recurser] Failed to deliver prove ack: event loop channel closed");
                    }
                });
                self.worker.set_current_computation(handle);
            }
        }

        Ok(())
    }

    /// Handles a setup the coordinator sent: acks one already done at once, and
    /// otherwise starts it, unless the worker cannot take it yet or something is held
    /// back before it; then it waits its turn ([`Self::run_deferred`]).
    fn handle_setup_request(
        &mut self,
        setup: SetupProgram,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
        loop_tx: &LoopEventSender,
    ) {
        if self.ack_if_set_up(&setup, message_sender) {
            return;
        }
        if self.deferred.is_empty() && self.may_start_setup() {
            self.start_setup(setup, loop_tx);
        } else {
            info!(
                "[Setup] job_id {} Deferring setup for hash_id {} until the worker is idle",
                setup.job_id, setup.hash_id
            );
            self.deferred.push_back(DeferredWork::Setup(setup));
        }
    }

    /// Acks `setup` if its program is already set up in its mode; returns whether it did.
    fn ack_if_set_up(
        &self,
        setup: &SetupProgram,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
    ) -> bool {
        let Some(program_vk) =
            self.worker.program_vk(&setup.hash_id, setup.with_hints, setup.emulator_only)
        else {
            return false;
        };
        let ack = Self::build_setup_program_ack(
            setup.job_id.clone(),
            self.worker_config.worker.worker_id.as_string(),
            setup.hash_id.clone(),
            Ok(&program_vk),
        );
        if let Err(e) = message_sender.send(ack) {
            warn!("Failed to send SetupProgramAck: {}", e);
        }
        true
    }

    /// Whether a setup may start: no job, computation, recovery or other setup is
    /// under way.
    fn may_start_setup(&self) -> bool {
        !self.worker.has_job()
            && !self.worker.has_live_computation()
            && !self.recovery.as_ref().is_some_and(|recovery| !recovery.is_finished())
            && self.setup_in_flight.is_none()
    }

    /// Runs `setup` off the message loop, so heartbeats keep flowing while it
    /// writes the ELF and runs `setup_internal`. The `&mut self` map inserts and the
    /// ack are finished by `handle_computation_result` when its
    /// `SetupProgramComplete` arrives.
    fn start_setup(&mut self, setup: SetupProgram, loop_tx: &LoopEventSender) {
        info!("[Setup] job_id {} Starting setup for hash_id {}", setup.job_id, setup.hash_id);
        let worker_id = self.worker_config.worker.worker_id.as_string();
        let job_id = JobId::from(setup.job_id.clone());
        let hash_id = setup.hash_id.clone();
        let with_hints = setup.with_hints;
        let emulator_only = setup.emulator_only;
        let prover = self.worker.prover_arc();
        let tx = loop_tx.clone();
        self.setup_in_flight = Some(tokio::task::spawn_blocking(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let guest_program =
                    Self::stage_guest_program(&setup.hash_id, setup.program_name, setup.elf_bytes)?;
                let vk = Worker::<T>::setup_compute(
                    prover.as_ref(),
                    &setup.hash_id,
                    setup.with_hints,
                    setup.emulator_only,
                    &guest_program,
                )?;
                Ok::<_, anyhow::Error>((vk, guest_program))
            }))
            .unwrap_or_else(|_| Err(anyhow!("setup panicked")));

            if tx
                .send_computation(ComputationResult::SetupProgramComplete {
                    job_id,
                    worker_id,
                    hash_id,
                    with_hints,
                    emulator_only,
                    result,
                })
                .is_err()
            {
                warn!("[Setup] Failed to deliver setup result: event loop channel closed");
            }
        }));
    }

    /// Starts what was held back, in order, as far as the worker can take it: a
    /// setup once no job, computation, recovery or other setup is under way, and a
    /// task once no setup is.
    async fn run_deferred(
        &mut self,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
        loop_tx: &LoopEventSender,
    ) {
        loop {
            let ready = match self.deferred.front() {
                Some(DeferredWork::Setup(_)) => self.may_start_setup(),
                Some(_) => self.setup_in_flight.is_none(),
                None => false,
            };
            if !ready {
                return;
            }
            match self.deferred.pop_front() {
                Some(DeferredWork::Setup(setup)) => {
                    if !self.ack_if_set_up(&setup, message_sender) {
                        self.start_setup(setup, loop_tx);
                    }
                }
                Some(DeferredWork::Task(request)) => {
                    let job_id = request.job_id.clone();
                    self.dispatch_task(*request, message_sender, loop_tx).await;
                    // Its stream data goes in now, ahead of a setup held back meanwhile,
                    // which waits for this job to end.
                    let (data, rest) = std::mem::take(&mut self.deferred)
                        .into_iter()
                        .partition::<VecDeque<_>, _>(|work| {
                            !matches!(work, DeferredWork::Setup(_) | DeferredWork::Task(_))
                                && work.job_id() == Some(job_id.as_str())
                        });
                    self.deferred = rest;
                    for work in data {
                        self.replay(work, message_sender).await;
                    }
                }
                Some(work) => self.replay(work, message_sender).await,
                None => return,
            }
        }
    }

    /// Hands held-back stream data in, and handles its failure as the loop handles that
    /// of a message.
    async fn replay(
        &mut self,
        work: DeferredWork,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
    ) {
        let result = match work {
            DeferredWork::Stream(data) => self.handle_stream_data(data).await,
            DeferredWork::Input(data) => self.handle_input_stream_data(data).await,
            DeferredWork::Setup(_) | DeferredWork::Task(_) => {
                unreachable!("only stream data is replayed")
            }
        };
        if let Err(e) = result {
            error!("Error handling held-back stream data: {e}");
            self.report_computation_error(message_sender, &e.to_string()).await;
            self.worker.set_current_job(None);
            self.worker.set_state(WorkerState::Ready);
        }
    }

    /// Whether a task of `job_id` is held back, so that its stream data must be too.
    fn holds_back_job(&self, job_id: &str) -> bool {
        self.deferred
            .iter()
            .any(|work| matches!(work, DeferredWork::Task(task) if task.job_id == job_id))
    }

    /// Dispatches a task the coordinator sent.
    async fn dispatch_task(
        &mut self,
        request: ExecuteTaskRequest,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
        loop_tx: &LoopEventSender,
    ) {
        let job_id = request.job_id.clone();
        let task_type_int = request.task_type;
        let dispatch = match TaskType::try_from(task_type_int) {
            Ok(TaskType::Execution) => self.execute_only(request, loop_tx).await,
            Ok(TaskType::PartialContribution) => self.partial_contribution(request, loop_tx).await,
            Ok(TaskType::Prove) => self.prove(request, loop_tx).await,
            Ok(TaskType::Aggregate) => self.aggregate(request, loop_tx).await,
            Ok(TaskType::Wrap) => self.handle_wrap_task(request, message_sender).await,
            Err(_) => Err(anyhow!("Unknown task type: {task_type_int}")),
        };

        // Dispatch can fail before any computation starts (cache miss,
        // unknown task type), so `current_job` may not be set and
        // `report_computation_error` would silently drop the report.
        // Send an explicit failure using the request's `job_id`.
        if let Err(e) = dispatch {
            error!("Failed to dispatch task {job_id}: {e:#}");
            let response = WorkerMessage {
                payload: Some(worker_message::Payload::ExecuteTaskResponse(ExecuteTaskResponse {
                    worker_id: self.worker_config.worker.worker_id.as_string(),
                    job_id,
                    task_type: task_type_int,
                    success: false,
                    result_data: None,
                    error_message: e.to_string(),
                    worker_in_recovery: false,
                })),
            };
            if let Err(send_err) = message_sender.send(response) {
                warn!("Failed to send dispatch-failure response: {send_err}");
            }
            self.worker.set_current_job(None);
            self.worker.set_state(WorkerState::Ready);
        }
    }

    /// Writes the ELF to the content-addressed cache (idempotent) and loads the
    /// `GuestProgram`, moving the bytes in (no clone). The cache-write contract
    /// must stay byte-identical for the content-addressed skip check.
    fn stage_guest_program(
        hash_id: &str,
        program_name: String,
        elf_bytes: Vec<u8>,
    ) -> Result<std::sync::Arc<GuestProgram>> {
        let elf_path = ZiskPaths::global().elf_cache(hash_id);
        if !elf_path.exists() {
            if let Some(parent) = elf_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&elf_path, &elf_bytes)?;
        }
        Ok(std::sync::Arc::new(GuestProgram::from_bytes(program_name, elf_bytes)))
    }

    /// Handles a `SetupAggregationProgram` message from the coordinator.
    ///
    /// Runs the recurser setup in a scoped 64 MB-stack rayon pool
    /// (proofman setup overflows the default ~2 MB worker stack). Idempotent:
    /// returns the existing verkey if setup has already run for this `recurser_id`.
    fn handle_setup_aggregation_program(
        prover: &ZiskProver<T>,
        proving_key: &std::path::Path,
        setup: SetupAggregationProgram,
    ) -> Result<(Vec<u8>, String)> {
        use zisk_recurser::setup::{run_setup_recurser_aggregator, SetupRecurserAggregatorOptions};

        info!(
            "[Recurser] job_id {} Received SetupAggregationProgram for recurser_id {}",
            setup.job_id, setup.recurser_id
        );

        let spec = setup.spec.ok_or_else(|| anyhow!("SetupAggregationProgram.spec must be set"))?;

        // The key this worker's prover actually loaded, which an explicit `--proving-key`
        // can move off the global default. Everything below -- the vadcop_final verkey the
        // recurser id is derived from, and the setup built from it -- has to come from that
        // one key: a recurser built against a different key produces a verkey that cannot
        // verify the proofs this prover goes on to make.
        let setup_key = proving_key
            .to_str()
            .ok_or_else(|| anyhow!("proving key path is not valid UTF-8"))?
            .to_string();
        let output_dir = ZiskPaths::global()
            .home
            .join("recurser")
            .to_str()
            .ok_or_else(|| anyhow!("~/.zisk/recurser path is not valid UTF-8"))?
            .to_string();

        let normalize = spec
            .normalize
            .as_ref()
            .map(|n| zisk_recurser::NormalizeCircuit { body: n.body.clone() });
        // Proto `ProgramVk { limbs }` → 4-limb array. Pad/truncate defensively;
        // a wrong length changes the derived id and is caught by the check below.
        let program_vks: Vec<[String; 4]> = spec
            .program_vks
            .iter()
            .map(|vk| {
                let mut limbs = vk.limbs.clone();
                limbs.resize(4, String::from("0"));
                [limbs[0].clone(), limbs[1].clone(), limbs[2].clone(), limbs[3].clone()]
            })
            .collect();
        // The artifact dir is keyed by the *claimed* id; recompute the id from
        // the spec so a mismatched claim can't be served another definition's
        // completed setup (or silently register under the wrong name).
        let zisk_vk = zisk_recurser::setup::read_vadcop_final_verkey(&setup_key)
            .map_err(|e| anyhow!("failed to read local vadcop_final verkey: {e:#}"))?;
        let expected_inputs = zisk_recurser::RecurserManifestInputs::new(
            zisk_vk,
            program_vks.clone(),
            normalize.as_ref(),
            &spec.aggregate_publics_body,
            spec.n_free as usize,
            spec.n_publics_agg as usize,
        );
        let expected_id = expected_inputs.compute_id();
        if expected_id != setup.recurser_id {
            return Err(anyhow!(
                "recurser_id mismatch: request claims '{}' but the spec derives '{}' on this \
                 worker; this usually means the client and worker proving keys differ \
                 (vadcop_final verkey mismatch) or the spec was altered in transit",
                setup.recurser_id,
                expected_id,
            ));
        }

        let artifacts =
            zisk_recurser::artifacts::RecurserArtifacts::new(&output_dir, &setup.recurser_id);

        if !artifacts.is_active() {
            let opts = SetupRecurserAggregatorOptions {
                proving_key: setup_key,
                output_dir: output_dir.clone(),
                templates: zisk_recurser::CircomTemplates {
                    normalize,
                    aggregate_publics: spec.aggregate_publics_body.clone(),
                    n_free: spec.n_free as usize,
                    n_publics_agg: spec.n_publics_agg as usize,
                    program_vks,
                },
            };

            let pool = rayon::ThreadPoolBuilder::new()
                .stack_size(64 * 1024 * 1024)
                .build()
                .map_err(|e| anyhow!("failed to build 64 MB-stack rayon pool: {e}"))?;
            pool.install(|| run_setup_recurser_aggregator(&opts))
                .map_err(|e| anyhow!("recurser setup failed: {e}"))?;
        } else {
            info!(
                "[Recurser] setup already complete for recurser_id {}; skipping setup",
                setup.recurser_id
            );
        }

        // Register the setup with this worker's proofman so subsequent
        // `RunAggregateProofs` jobs routed here can prove. Covers both the
        // fresh-setup and cache-hit branches above.
        prover
            .register_recurser(&output_dir, &setup.recurser_id)
            .map_err(|e| anyhow!("recurser registration failed: {e}"))?;

        let vk = std::fs::read(artifacts.verkey_bin_path()).map_err(|e| {
            anyhow!(
                "failed to read recurser verkey at {}: {}",
                artifacts.verkey_bin_path().display(),
                e
            )
        })?;

        // The hash family travels with the verkey for verify-time matching. Read it from
        // the prover's loaded key, not `ZiskPaths::global()`: an explicit `--proving-key`
        // would otherwise advertise the global key's family. Also covers the cache-hit
        // branch above, where no setup ran to return it.
        let hash_mode =
            prover.hash().map_err(|e| anyhow!("failed to read prover hash family: {e}"))?;

        info!(
            "[Recurser] job_id {} Completed recurser setup for recurser_id {}",
            setup.job_id, setup.recurser_id
        );
        Ok((vk, hash_mode))
    }

    /// Handles a `RunAggregateProofs` message from the coordinator.
    ///
    /// Folds two `VadcopFinalProof`s into one and returns the bincode-serialized
    /// SDK `Proof`. No 64 MB rayon pool — the prove path is FFI.
    fn handle_run_aggregate_proofs(
        prover: &ZiskProver<T>,
        req: RunAggregateProofs,
    ) -> Result<Vec<u8>> {
        use proofman_verifier::VadcopFinalProof;

        info!(
            "[Recurser] job_id {} Received RunAggregateProofs for recurser_id {}",
            req.job_id, req.recurser_id
        );

        let (proof_a, _): (VadcopFinalProof, _) =
            bincode::serde::decode_from_slice(&req.proof_a, bincode::config::standard())
                .map_err(|e| anyhow!("failed to deserialize proof_a: {e}"))?;
        let (proof_b, _): (VadcopFinalProof, _) =
            bincode::serde::decode_from_slice(&req.proof_b, bincode::config::standard())
                .map_err(|e| anyhow!("failed to deserialize proof_b: {e}"))?;

        let root_c_override = match req.root_c_recurser_agg.len() {
            0 => None,
            4 => Some([
                req.root_c_recurser_agg[0],
                req.root_c_recurser_agg[1],
                req.root_c_recurser_agg[2],
                req.root_c_recurser_agg[3],
            ]),
            n => return Err(anyhow!("root_c_recurser_agg must have 0 or 4 limbs; got {}", n)),
        };

        let output_dir = ZiskPaths::global()
            .home
            .join("recurser")
            .to_str()
            .ok_or_else(|| anyhow!("~/.zisk/recurser path is not valid UTF-8"))?
            .to_string();

        // Ensure this worker's proofman has the recurser registered. Normally a
        // prior `SetupAggregationProgram` job already did this (the coordinator
        // rehydrates setups to every worker), but registering here is idempotent
        // and guards against a prove landing before/without setup on this worker.
        prover.register_recurser(&output_dir, &req.recurser_id).map_err(|e| {
            anyhow!(
                "recurser registration failed (was this recurser set up on this worker? \
                 a setup rehydration may have been missed): {e:#}"
            )
        })?;

        // The wire carries a single flat free-value buffer per side, which now maps
        // 1:1 onto the unified `n_free` width that prove_recurser expects.
        //
        // Reuse the worker's already-initialized proofman rather than building a
        // fresh one per fold: `ProofMan::new` initializes the process-global MPI
        // context, which can only happen once per process.
        let vfp = prover
            .prove_recurser(
                &req.recurser_id,
                &proof_a,
                &proof_b,
                &req.free_inputs_a,
                &req.free_inputs_b,
                root_c_override,
            )
            .map_err(|e| anyhow!("recurser prove failed: {e}"))?;

        // Stamp the output Proof with the recurser's own verkey as zisk_vk.
        let artifacts =
            zisk_recurser::artifacts::RecurserArtifacts::new(&output_dir, &req.recurser_id);
        let vk_bytes = std::fs::read(artifacts.verkey_bin_path())
            .map_err(|e| anyhow!("failed to read recurser verkey: {e}"))?;
        if vk_bytes.len() != 32 {
            return Err(anyhow!("recurser verkey must be 32 bytes, got {}", vk_bytes.len()));
        }
        let mut zisk_vk = Vec::with_capacity(4);
        for i in 0..4 {
            let chunk: [u8; 8] = vk_bytes[i * 8..(i + 1) * 8].try_into().unwrap();
            zisk_vk.push(u64::from_le_bytes(chunk));
        }

        // Same guard as the embedded path: compression strips the flag that marks this a
        // fold, taking the recursion-domain check in `Proof::verify` with it.
        if vfp.compressed {
            return Err(anyhow!(
                "recurser produced a compressed proof; the recursion-domain check in \
                 Proof::verify cannot classify it"
            ));
        }

        // The proof's hash family travels on the VadcopFinalProof (stamped by
        // proofman from the recurser's proving key); carry it onto the Proof.
        let proof = Proof::new_from_vadcop_proof(
            &vfp.proof_with_publics(),
            vfp.compressed,
            zisk_vk,
            vfp.hash.clone(),
        )?;
        let bytes = bincode::serde::encode_to_vec(&proof, bincode::config::standard())
            .map_err(|e| anyhow!("failed to serialize recurser proof: {e}"))?;

        info!(
            "[Recurser] job_id {} Completed recurser prove for recurser_id {}",
            req.job_id, req.recurser_id
        );
        Ok(bytes)
    }

    /// Handle a coordinator contribution request: set up the job and spawn the
    /// contribution computation.
    pub async fn partial_contribution(
        &mut self,
        request: ExecuteTaskRequest,
        loop_tx: &LoopEventSender,
    ) -> Result<()> {
        let task_received_time = chrono::Utc::now();
        info!("Starting Partial Contribution for {}", request.job_id);

        // Defensive: the coordinator won't dispatch here until we emit
        // `WorkerRecoveryComplete` (which already drained any prior task), so
        // this is normally a no-op. If a stale computation is somehow still
        // live, drain it before `prepare_for_new_job`'s `prover.reset()` reuses
        // the ASM shmem the child may still be writing.
        let cancelled = self.worker.cancel_current_computation();
        Self::drain_cancelled_computation(cancelled.compute, "partial_contribution").await;
        // Also wait for the previous stream-ordering thread to stop before
        // `prepare_for_new_job` resets the shared HintsProcessor, so it can't
        // still be inside `process_hints` on the new stream.
        Self::drain_cancelled_computation(cancelled.stream, "partial_contribution stream").await;

        // Extract the PartialContribution params
        let Some(execute_task_request::Params::ContributionParams(params)) = request.params else {
            return Err(anyhow!("Expected ContributionParams for Partial Contribution task"));
        };
        let seed = Self::job_random_seed(&params.random_seed)?;

        let job_id = JobId::from(request.job_id);
        let input_source = Self::resolve_input_source(
            &self.worker_config.worker.inputs_folder,
            params.input_source,
        )
        .await?;

        let hints_source = if let Some(hints_data) = params.hints_data {
            HintsSourceDto::HintsData(hints_data)
        } else if let Some(hints_path) = &params.hints_path {
            if params.hints_stream {
                HintsSourceDto::HintsStream(hints_path.clone())
            } else {
                let hints_uri = Self::validate_subdir(
                    &self.worker_config.worker.inputs_folder,
                    &PathBuf::from(hints_path),
                )
                .await?;

                HintsSourceDto::HintsPath(hints_uri.to_string_lossy().to_string())
            }
        } else {
            HintsSourceDto::HintsNull
        };

        let with_hints = !matches!(hints_source, HintsSourceDto::HintsNull);
        let is_first_process =
            params.worker_allocation.contains(&0) && self.worker.world_rank() == 0;
        self.worker.prepare_for_new_job(&params.hash_id, with_hints, is_first_process)?;

        let data_ctx =
            DataCtx { data_id: DataId::from(params.data_id), input_source, hints_source };

        let job = self.worker.new_job(
            job_id.clone(),
            params.hash_id,
            data_ctx,
            seed,
            params.rank_id,
            params.total_workers,
            params.worker_allocation,
            params.job_compute_units,
            Some(task_received_time),
        );

        // Start computation in background task
        self.worker.set_current_computation(
            self.worker.handle_partial_contribution(job, loop_tx.clone()).await?,
        );

        Ok(())
    }

    /// Handle a coordinator execute-only request: set up the job and spawn the
    /// execution.
    pub async fn execute_only(
        &mut self,
        request: ExecuteTaskRequest,
        loop_tx: &LoopEventSender,
    ) -> Result<()> {
        let task_received_time = chrono::Utc::now();
        info!("Starting Execution-only for {}", request.job_id);

        // Defensive drain before `prepare_for_new_job`'s `prover.reset()` reuses
        // the ASM shmem — normally a no-op (see `partial_contribution`).
        let cancelled = self.worker.cancel_current_computation();
        Self::drain_cancelled_computation(cancelled.compute, "execute_only").await;
        // See `partial_contribution`: also drain the previous stream-ordering
        // thread before the reset in `prepare_for_new_job`.
        Self::drain_cancelled_computation(cancelled.stream, "execute_only stream").await;

        // Extract the ExecutionParams (reuses ContributionParams structure)
        let Some(execute_task_request::Params::ExecutionParams(params)) = request.params else {
            return Err(anyhow!("Expected ExecutionParams for Execution-only task"));
        };
        let seed = Self::job_random_seed(&params.random_seed)?;

        let job_id = JobId::from(request.job_id);
        let input_source = Self::resolve_input_source(
            &self.worker_config.worker.inputs_folder,
            params.input_source,
        )
        .await?;

        let hints_source = if let Some(hints_data) = params.hints_data {
            HintsSourceDto::HintsData(hints_data)
        } else if let Some(hints_path) = &params.hints_path {
            if params.hints_stream {
                HintsSourceDto::HintsStream(hints_path.clone())
            } else {
                let hints_uri = Self::validate_subdir(
                    &self.worker_config.worker.inputs_folder,
                    &PathBuf::from(hints_path),
                )
                .await?;

                HintsSourceDto::HintsPath(hints_uri.to_string_lossy().to_string())
            }
        } else {
            HintsSourceDto::HintsNull
        };

        let with_hints = !matches!(hints_source, HintsSourceDto::HintsNull);
        let is_first_process =
            params.worker_allocation.contains(&0) && self.worker.world_rank() == 0;
        self.worker.prepare_for_new_job(&params.hash_id, with_hints, is_first_process)?;

        let data_ctx =
            DataCtx { data_id: DataId::from(params.data_id), input_source, hints_source };

        let job = self.worker.new_job(
            job_id.clone(),
            params.hash_id,
            data_ctx,
            seed,
            params.rank_id,
            params.total_workers,
            params.worker_allocation,
            params.job_compute_units,
            Some(task_received_time),
        );

        // Start execution-only computation in background task
        self.worker.set_current_computation(
            self.worker.handle_execution_only(job, loop_tx.clone()).await?,
        );

        Ok(())
    }

    /// Resolve `InputSource` from a gRPC request into `InputSourceDto`.
    ///
    /// File paths are validated under `inputs_folder` with a 60 s timeout.
    async fn resolve_input_source(
        inputs_folder: &Path,
        source: Option<InputSource>,
    ) -> Result<InputSourceDto> {
        match source {
            Some(InputSource::InputPath(ref path)) => {
                let validated = Self::validate_subdir(inputs_folder, &PathBuf::from(path)).await?;
                Ok(InputSourceDto::InputPath(validated.to_string_lossy().to_string()))
            }
            Some(InputSource::InputData(data)) => Ok(InputSourceDto::InputData(data)),
            None => Ok(InputSourceDto::InputNull),
        }
    }

    /// The job's host randomness seed from its task params. Every task that executes carries one,
    /// so a missing or malformed seed fails the task before it touches the prover.
    fn job_random_seed(bytes: &[u8]) -> Result<RandomSeed> {
        RandomSeed::try_from(bytes).context("coordinator sent no valid random seed for the job")
    }

    /// Validates that a subpath is within the base directory and waits for it to exist.
    ///
    /// This function joins the base directory with the provided subpath, waits for the
    /// resulting file/directory to appear (up to 60 seconds), and validates that the
    /// resolved path is within the base directory to prevent path traversal attacks.
    ///
    /// # Security Considerations
    /// - Joins base and subpath before validation
    /// - Canonicalizes paths to resolve symlinks and relative components (e.g., `..`)
    /// - Validates that the resolved path is within the base directory
    /// - Note: There's a small TOCTOU window between file existence check and canonicalization
    ///   where a file could theoretically be replaced with a malicious symlink
    ///
    /// # Arguments
    /// * `base_dir` - The base directory that must contain the subpath
    /// * `subpath` - The relative path within base_dir (can include subdirectories)
    ///
    /// # Returns
    /// * `Ok(PathBuf)` - The validated, canonicalized full path
    /// * `Err` - If the path doesn't appear within timeout or is outside base directory
    async fn validate_subdir(base_dir: &Path, subpath: &Path) -> Result<PathBuf> {
        let base_canonical =
            base_dir.canonicalize().map_err(|e| anyhow!("Inputs folder error: {e}"))?;

        // Join base with subpath to get full path
        let full_path = base_dir.join(subpath);

        // Wait for file to appear (timeout: 60 seconds)
        let timeout = Duration::from_secs(60);
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(500); // Poll every 500ms

        while !full_path.exists() {
            if start.elapsed() > timeout {
                return Err(anyhow!(
                    "Input path {:?} (subpath: {:?}) did not appear within {:?}",
                    full_path,
                    subpath,
                    timeout
                ));
            }
            tokio::time::sleep(poll_interval).await;
        }

        info!("Found input path {:?} (elapsed: {:?})", full_path, start.elapsed());

        // Canonicalize immediately after existence check to minimize TOCTOU window
        let path_canonical =
            full_path.canonicalize().map_err(|e| anyhow!("Input path error: {e}"))?;

        // Validate that the canonical path is within the base directory
        if path_canonical.starts_with(&base_canonical) {
            Ok(path_canonical)
        } else {
            Err(anyhow!(
                "Input path {:?} (resolved to {:?}) is outside base directory {:?}",
                subpath,
                path_canonical,
                base_canonical
            ))
        }
    }

    /// Handle a coordinator prove request: spawn the prove computation from the
    /// supplied challenges.
    pub async fn prove(
        &mut self,
        request: ExecuteTaskRequest,
        loop_tx: &LoopEventSender,
    ) -> Result<()> {
        if self.worker.current_job().is_none() {
            return Err(anyhow!("Prove received without current job context"));
        }

        let job = self.worker.current_job().clone().unwrap().clone();
        let job_id = job.lock().await.job_id.clone();
        let job_id_str = job_id.as_string();

        if job_id_str != request.job_id {
            return Err(anyhow!(
                "Job ID mismatch in Prove: expected {}, got {}",
                job_id_str,
                request.job_id
            ));
        }

        // Idempotency guard: a duplicate Prove for a job whose computation is
        // still running must NOT spawn a second concurrent prove_phase (it would
        // rewrite the global challenge and race the first phase over the same
        // GPU streams -> internally inconsistent proofs). Log loudly: if this
        // ever fires, the coordinator double-dispatched a phase task.
        if self.worker.has_live_computation() {
            error!(
                "[DUPLICATE-DISPATCH] Prove for {} received while a computation is still running — ignoring",
                job_id
            );
            return Ok(());
        }

        info!("Starting Prove for {}", job_id);

        // Extract the Prove params
        let Some(execute_task_request::Params::ProveParams(prove_params)) = request.params else {
            return Err(anyhow!("Expected Prove params for Prove task"));
        };

        let cont: Vec<_> = prove_params
            .challenges
            .into_iter()
            .map(|ch| ContributionsInfo {
                worker_index: ch.worker_index,
                airgroup_id: ch.airgroup_id as usize,
                challenge: ch.challenge,
                aggregated: false,
            })
            .collect();

        self.worker
            .set_current_computation(self.worker.handle_prove(job, cont, loop_tx.clone()).await?);

        Ok(())
    }

    /// Handle a coordinator aggregate request: spawn the join/aggregate of the
    /// supplied worker proofs.
    pub async fn aggregate(
        &mut self,
        request: ExecuteTaskRequest,
        loop_tx: &LoopEventSender,
    ) -> Result<()> {
        if self.worker.current_job().is_none() {
            return Err(anyhow!("Aggregate received without current job context"));
        }

        // Idempotency guard (symmetric with `prove`): a duplicate/replayed
        // Aggregate while the previous aggregation computation is still running
        // must NOT spawn a second concurrent proofman task over the same GPU
        // streams (set_current_computation overwrites the handle and a dropped
        // JoinHandle DETACHES the running task). If this fires, the coordinator
        // double-dispatched (a scheduler gating bug, or a reconnect replay).
        if self.worker.has_live_computation() {
            error!(
                "[DUPLICATE-DISPATCH] Aggregate for {} received while a computation is still running — ignoring",
                request.job_id
            );
            return Ok(());
        }

        let job = self.worker.current_job().clone().unwrap().clone();
        let job_id = job.lock().await.job_id.clone();

        if job_id.as_string() != request.job_id {
            return Err(anyhow!(
                "Job ID mismatch in Aggregate: expected {}, got {}",
                job_id.as_string(),
                request.job_id
            ));
        }

        // Extract the Aggregate params
        let Some(execute_task_request::Params::AggParams(agg_params)) = request.params else {
            return Err(anyhow!("Expected AggParams params for Aggregate task"));
        };

        let agg_proofs =
            agg_params.agg_proofs.ok_or_else(|| anyhow!("Missing agg_proofs in AggParams"))?.proofs;
        let agg_proofs: Vec<_> = agg_proofs
            .into_iter()
            .map(|p| AggProofData {
                worker_indexes: p.worker_indexes,
                airgroup_id: p.airgroup_id,
                values: p.values,
            })
            .collect();

        let agg_params = AggregationParams {
            agg_proofs,
            last_proof: agg_params.last_proof,
            final_proof: agg_params.final_proof,
            proof_type: ProofKind::from(agg_params.proof_type),
            keep_resident: agg_params.keep_resident,
            reset_state: agg_params.reset_state,
        };
        self.worker.set_current_computation(self.worker.handle_aggregate_proofs(
            job,
            agg_params,
            loop_tx.clone(),
        ));

        Ok(())
    }

    async fn handle_stream_data(&mut self, stream_data: StreamData) -> Result<()> {
        if self.worker.current_job().is_none() {
            return Err(anyhow!("Stream data received without current job context"));
        }

        let job = self.worker.current_job().unwrap();
        let current_job_id = {
            let job_guard = job.lock().await;
            job_guard.job_id.clone()
        };

        let stream_data_dto: StreamDataDto = stream_data.into();

        if current_job_id != stream_data_dto.job_id {
            return Err(anyhow!(
                "Job ID mismatch in StreamData: expected {}, got {}",
                current_job_id.as_string(),
                stream_data_dto.job_id
            ));
        }

        self.worker.route_stream_data(stream_data_dto).await
    }

    async fn handle_input_stream_data(
        &mut self,
        input_data: zisk_cluster_api::InputStreamData,
    ) -> Result<()> {
        let job = self
            .worker
            .current_job()
            .ok_or_else(|| anyhow!("InputStreamData received without current job context"))?;

        let current_job_id = job.lock().await.job_id.clone();
        let incoming_job_id = JobId::from(input_data.job_id.clone());

        if current_job_id != incoming_job_id {
            return Err(anyhow!(
                "Job ID mismatch in InputStreamData: expected {}, got {}",
                current_job_id.as_string(),
                incoming_job_id
            ));
        }

        if !input_data.payload.is_empty() && input_data.payload.len() % 8 != 0 {
            return Err(anyhow!(
                "InputStreamData payload length {} is not a multiple of 8 bytes",
                input_data.payload.len()
            ));
        }

        // gRPC `InputStreamData` only reaches rank 0. Mirror it to peer ranks
        // via MPI so their ASM children get the same bytes — without this
        // they wait on `chunk_done` until the semaphore times out (~10 s) and
        // every streamed-input job fails on every rank ≠ 0. Skipped when this
        // is the only rank: the conversion and borsh pass below copy the chunk
        // twice for nobody.
        // Wire format matches what `process_hints` broadcasts for hint-driven
        // inputs (see `precompiles/hints/src/hints_processor.rs`): tag byte
        // followed by a borsh-encoded `StreamMessage { data: Vec<u64> }`.
        if self.worker.n_processes() > 1 && !input_data.payload.is_empty() {
            let data_u64: Vec<u64> = input_data
                .payload
                .chunks_exact(8)
                .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
                .collect();
            let mut serialized = borsh::to_vec(&(
                crate::worker::WorkerMpiTag::ContributionsInputsStream,
                zisk_cluster_common::StreamMessage { data: data_u64 },
            ))
            .map_err(|e| anyhow!("Failed to serialize input stream broadcast: {e}"))?;
            self.worker.prover_arc().mpi_broadcast(&mut serialized)?;
        }

        self.worker.append_raw_input(&input_data.payload)
    }

    async fn handle_wrap_task(
        &mut self,
        request: ExecuteTaskRequest,
        message_sender: &mpsc::UnboundedSender<WorkerMessage>,
    ) -> Result<()> {
        let job_id = JobId::from(request.job_id.clone());

        let Some(execute_task_request::Params::WrapParams(wrap_params)) = request.params else {
            return Err(anyhow!("Expected WrapParams for Wrap task"));
        };

        let proof_data = wrap_params.proof_data;
        let proof_dest = wrap_params.proof_dest;

        let prover = self.worker.prover_arc();
        let worker_id_str = self.worker_config.worker.worker_id.as_string();
        let job_id_str = job_id.as_string();

        let (success, result_data, error_message) = tokio::task::spawn_blocking(move || {
            match Worker::<T>::execute_wrap_task(&prover, proof_data, proof_dest) {
                Ok(wrapped_bytes) => (
                    true,
                    Some(ResultData::WrapResult(zisk_cluster_api::WrapResult {
                        proof_data: wrapped_bytes,
                    })),
                    String::new(),
                ),
                Err(e) => {
                    error!("Wrap task failed for {}: {}", job_id_str, e);
                    (false, None, e.to_string())
                }
            }
        })
        .await?;

        let message = WorkerMessage {
            payload: Some(worker_message::Payload::ExecuteTaskResponse(ExecuteTaskResponse {
                worker_id: worker_id_str,
                job_id: job_id.as_string(),
                task_type: TaskType::Wrap as i32,
                success,
                result_data,
                error_message,
                worker_in_recovery: false,
            })),
        };

        message_sender.send(message)?;
        Ok(())
    }
}

/// Whether a fold may be returned in `proof_type`.
///
/// `FinalCompressed` strips the `is_vadcop_final_proof` flag, and `Proof::verify` keys the
/// recursion-domain check on it -- so a compressed fold verifies weaker than the one that
/// was produced.
fn fold_allows_kind(proof_type: ProofKind) -> bool {
    proof_type != ProofKind::VadcopFinalMinimal
}

/// Record an aggregation failure and return the (empty) `proof_data` the response carries.
///
/// The diagnostic, the log line and the `success` flag move together: an empty
/// `proof_data` left paired with `success` reads to the coordinator as an intermediate
/// acknowledgement, which it rejects outright on the final task.
fn fail_aggregation(success: &mut bool, error_message: &mut String, diagnostic: String) -> Vec<u8> {
    error!("{diagnostic}");
    *error_message = diagnostic;
    *success = false;
    Vec::new()
}

#[cfg(test)]
mod aggregate_response_tests {
    use super::*;

    #[test]
    fn a_fold_may_not_be_returned_compressed() {
        assert!(!fold_allows_kind(ProofKind::VadcopFinalMinimal));
        assert!(fold_allows_kind(ProofKind::VadcopFinal));
        assert!(fold_allows_kind(ProofKind::Plonk));
    }

    /// The pairing the coordinator depends on: a failure never looks like an ack.
    #[test]
    fn a_failure_clears_success_and_carries_a_diagnostic() {
        let mut success = true;
        let mut error_message = String::new();

        let proof_data = fail_aggregation(&mut success, &mut error_message, "boom".to_string());

        assert!(!success, "an empty proof with success set reads as an intermediate ack");
        assert_eq!(error_message, "boom");
        assert!(proof_data.is_empty());
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    use std::sync::Mutex;

    /// A cluster of `n_processes` ranks whose peers reply in the order of
    /// `replies`, recording what rank 0's recovery does to it.
    struct RecordingProver {
        n_processes: i32,
        calls: Mutex<Vec<String>>,
        replies: Mutex<VecDeque<i32>>,
    }

    impl RecordingProver {
        fn new(n_processes: i32, replies: &[i32]) -> Self {
            Self {
                n_processes,
                calls: Mutex::new(Vec::new()),
                replies: Mutex::new(replies.iter().copied().collect()),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }

        fn record(&self, call: impl Into<String>) {
            self.calls.lock().unwrap().push(call.into());
        }
    }

    impl RecoveryActions for RecordingProver {
        fn n_processes(&self) -> i32 {
            self.n_processes
        }

        fn notify_cluster_cancellation(&self) {
            self.record("notify");
        }

        fn broadcast_cancel(&self, job_id: &str) -> Result<()> {
            self.record(format!("cancel {job_id}"));
            Ok(())
        }

        fn wait_until_proofman_ready(&self) {
            self.record("wait_until_proofman_ready");
        }

        fn try_receive_cancel_done(&self) -> Option<i32> {
            self.replies.lock().unwrap().pop_front()
        }
    }

    fn in_a_second() -> Instant {
        Instant::now() + Duration::from_secs(1)
    }

    /// The peers are told to stop before rank 0 waits on anything, so that they
    /// stop while it does.
    #[test]
    fn a_recovery_tells_the_peer_ranks_to_stop_first() {
        let prover = RecordingProver::new(3, &[]);
        begin_cluster_recovery(&prover, "job-1").unwrap();
        assert_eq!(prover.calls(), ["notify", "cancel job-1"]);
    }

    #[test]
    fn a_recovery_on_one_rank_waits_for_no_peer() {
        let prover = RecordingProver::new(1, &[]);
        begin_cluster_recovery(&prover, "job-1").unwrap();
        finish_cluster_recovery(&prover, in_a_second()).unwrap();
        assert_eq!(prover.calls(), ["notify", "wait_until_proofman_ready"]);
    }

    #[test]
    fn a_recovery_ends_once_every_peer_rank_has_replied() {
        // In any order; a reply already counted counts once.
        let prover = RecordingProver::new(4, &[3, 1, 3, 2]);
        finish_cluster_recovery(&prover, in_a_second()).unwrap();
        assert_eq!(prover.calls(), ["wait_until_proofman_ready"]);
    }

    #[test]
    fn a_recovery_names_the_peer_ranks_that_never_replied() {
        let prover = RecordingProver::new(4, &[2]);
        let deadline = Instant::now() + Duration::from_millis(50);
        let error = finish_cluster_recovery(&prover, deadline).unwrap_err();
        assert!(error.to_string().contains("[1, 3]"), "{error:#}");
    }

    /// A prover that stays busy until the test lets it go, as one does while its
    /// rank's work waits in a collective nothing will complete.
    struct StuckProver {
        release: Mutex<std::sync::mpsc::Receiver<()>>,
    }

    impl RecoveryActions for StuckProver {
        fn n_processes(&self) -> i32 {
            1
        }

        fn notify_cluster_cancellation(&self) {}

        fn broadcast_cancel(&self, _job_id: &str) -> Result<()> {
            Ok(())
        }

        fn wait_until_proofman_ready(&self) {
            let _ = self.release.lock().unwrap().recv();
        }

        fn try_receive_cancel_done(&self) -> Option<i32> {
            None
        }
    }

    struct PanickingProver;

    impl RecoveryActions for PanickingProver {
        fn n_processes(&self) -> i32 {
            1
        }

        fn notify_cluster_cancellation(&self) {}

        fn broadcast_cancel(&self, _job_id: &str) -> Result<()> {
            Ok(())
        }

        fn wait_until_proofman_ready(&self) {
            panic!("a prover that panics");
        }

        fn try_receive_cancel_done(&self) -> Option<i32> {
            None
        }
    }

    #[tokio::test]
    async fn a_handshake_that_completes_succeeds() {
        let prover = Arc::new(RecordingProver::new(2, &[1]));
        run_recovery_within(Arc::clone(&prover), Duration::from_secs(5)).await.unwrap();
        assert_eq!(prover.calls(), ["wait_until_proofman_ready"]);
    }

    #[tokio::test]
    async fn a_handshake_that_does_not_complete_in_time_fails() {
        let (release, stuck) = std::sync::mpsc::channel();
        let prover = Arc::new(StuckProver { release: Mutex::new(stuck) });

        let outcome = run_recovery_within(prover, Duration::from_millis(50)).await;
        // Let the prover go, so the runtime's shutdown does not wait for it.
        release.send(()).unwrap();

        assert!(matches!(&outcome, Err(why) if why.contains("timed out")), "{outcome:?}");
    }

    #[tokio::test]
    async fn a_handshake_that_panics_fails() {
        let outcome = run_recovery_within(Arc::new(PanickingProver), Duration::from_secs(5)).await;
        assert!(matches!(&outcome, Err(why) if why.contains("panicked")), "{outcome:?}");
    }

    /// Set in the child process of `a_panic_anywhere_aborts_through_the_hook`.
    const ABORT_ON_PANIC_CHILD: &str = "ZISK_WORKER_TEST_ABORT_ON_PANIC_CHILD";

    /// The child half of `a_panic_anywhere_aborts_through_the_hook`, in a process of
    /// its own since a panic hook is process-wide. Does nothing unless that test
    /// runs it.
    #[test]
    fn abort_on_panic_child() {
        if std::env::var_os(ABORT_ON_PANIC_CHILD).is_none() {
            return;
        }
        abort_on_panic(|why, location| {
            println!("aborting: {why} (at {location})");
            std::process::exit(WEDGED_EXIT_CODE)
        });
        // As a witness thread's panic would be: on a thread nobody joins in time.
        let _ = std::thread::spawn(|| panic!("a witness thread's panic")).join();
        unreachable!("the hook ends the process before the panic unwinds");
    }

    #[test]
    fn a_panic_anywhere_aborts_through_the_hook() {
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "worker_node::recovery_tests::abort_on_panic_child", "--nocapture"])
            .env(ABORT_ON_PANIC_CHILD, "1")
            .output()
            .unwrap();

        assert_eq!(child.status.code(), Some(WEDGED_EXIT_CODE), "{child:?}");
        let stdout = String::from_utf8_lossy(&child.stdout);
        assert!(stdout.contains("aborting: a panic left the other MPI ranks"), "{stdout}");
        assert!(stdout.contains("worker_node.rs:"), "the panic's location is passed on: {stdout}");
        let stderr = String::from_utf8_lossy(&child.stderr);
        assert!(stderr.contains("a witness thread's panic"), "the panic is reported: {stderr}");
    }

    /// Process-long loop channel must redeliver after the original
    /// `message_sender` is dropped (transient disconnect).
    #[tokio::test]
    async fn recovery_complete_channel_survives_message_sender_drop() {
        let (raw_tx, mut loop_rx) = mpsc::unbounded_channel::<LoopEvent>();
        let loop_tx = LoopEventSender::new(raw_tx);
        let (msg_tx, msg_rx) = mpsc::unbounded_channel::<WorkerMessage>();

        loop_tx.send_recovery_complete(WorkerRecoveryComplete { worker_id: "w0".into() }).unwrap();

        drop(msg_tx);
        drop(msg_rx);

        let (new_msg_tx, mut new_msg_rx) = mpsc::unbounded_channel::<WorkerMessage>();
        let event =
            loop_rx.recv().await.expect("WorkerRecoveryComplete must persist across reconnect");
        let LoopEvent::RecoveryComplete(rc) = event else {
            panic!("expected RecoveryComplete event");
        };
        new_msg_tx
            .send(WorkerMessage { payload: Some(worker_message::Payload::RecoveryComplete(rc)) })
            .unwrap();

        let forwarded = new_msg_rx.recv().await.unwrap();
        assert!(matches!(forwarded.payload, Some(worker_message::Payload::RecoveryComplete(_))));
    }
}
