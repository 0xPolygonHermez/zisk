//! In-memory mock backend for development and testing.
//!
//! `MockBackend` needs no coordinator. Every submitted job auto-progresses
//! through its lifecycle in a background Tokio task. State is protected by a
//! single `Mutex`; all mutable access is brief (no I/O inside the lock).
//!
//! ## Job lifecycle (mock timing)
//!
//! ```text
//! t=0ms   Queued          → JobEvent::Queued
//! t=20ms  Running         → JobEvent::Started
//! t=40ms  (Prove only)    → JobEvent::Progress(Contributions)
//! t=80ms  (Prove only)    → JobEvent::Progress(Prove)
//! t=150ms Completed       → JobEvent::Completed
//! ```
//!
//! Jobs submitted with `InputKind::Inline { is_last: false }` pause at
//! `WaitingForInput` and resume once the input channel receives a chunk with
//! `is_last: true`.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use futures::stream;
use tokio::sync::{broadcast, mpsc, Mutex, Notify};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::{
    BackendService, DomainExecutionStats, DomainInputChunk, DomainInputKind, DomainJobEvent,
    DomainJobEventCancelled, DomainJobEventCompleted, DomainJobEventFailed, DomainJobEventProgress,
    DomainJobEventQueued, DomainJobEventStarted, DomainJobEventWaitingForInput, DomainJobKind,
    DomainJobKindResponse, DomainJobPhase, DomainJobStatus, DomainProof, DomainProofKind,
    InputChunkStream, JobEventStream, WaitResult,
};
use crate::errors::{GatewayError, GatewayResult};

// ── Internal state ────────────────────────────────────────────────────────────

struct JobRecord {
    status: DomainJobStatus,
    result: Option<DomainJobKindResponse>,
    /// Notified whenever `status` changes — used by `wait_job_result`.
    notify: Arc<Notify>,
}

struct MockState {
    /// Content-addressed program registry (hash_id). Idempotent — same ELF = same hash.
    programs: HashSet<String>,
    jobs: HashMap<Uuid, JobRecord>,
    /// Per-job broadcast channel for `watch_job` subscribers.
    event_txs: HashMap<Uuid, broadcast::Sender<DomainJobEvent>>,
    /// Per-job input channels for `push_job_input`.
    input_txs: HashMap<Uuid, mpsc::Sender<DomainInputChunk>>,
}

impl MockState {
    fn new() -> Self {
        Self {
            programs: HashSet::new(),
            jobs: HashMap::new(),
            event_txs: HashMap::new(),
            input_txs: HashMap::new(),
        }
    }
}

/// How long a terminal job's record is retained before being evicted.
const JOB_TTL: Duration = Duration::from_secs(5 * 60);

// ── MockBackend ───────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct MockBackend {
    state: Arc<Mutex<MockState>>,
    cancel: CancellationToken,
}

impl MockBackend {
    pub fn new(cancel: CancellationToken) -> Self {
        Self { state: Arc::new(Mutex::new(MockState::new())), cancel }
    }

    // ── internal helpers ─────────────────────────────────────────────────────

    /// Transition a job's status, persist the result if provided, and
    /// broadcast the event. Also fires `notify` so `wait_job_result` unblocks.
    async fn transition(
        state: &Arc<Mutex<MockState>>,
        cancel: &CancellationToken,
        job_id: Uuid,
        status: DomainJobStatus,
        result: Option<DomainJobKindResponse>,
        event: DomainJobEvent,
    ) {
        let notify = {
            let mut s = state.lock().await;
            if let Some(rec) = s.jobs.get_mut(&job_id) {
                let is_terminal = status.is_terminal();
                rec.status = status;
                if let Some(r) = result {
                    rec.result = Some(r);
                }
                let notify = rec.notify.clone();
                if let Some(tx) = s.event_txs.get(&job_id) {
                    let _ = tx.send(event);
                }
                if is_terminal {
                    // Drop channels — job is done, no more events or input.
                    s.event_txs.remove(&job_id);
                    s.input_txs.remove(&job_id);
                    Self::schedule_ttl_eviction(Arc::clone(state), cancel, job_id);
                }
                notify
            } else {
                return;
            }
        };
        notify.notify_waiters();
    }

    /// Cancel a job. Returns:
    /// - `None`        — job not found
    /// - `Some(false)` — already terminal (idempotent cancel)
    /// - `Some(true)`  — successfully cancelled
    async fn do_cancel(
        state: &Arc<Mutex<MockState>>,
        cancel: &CancellationToken,
        job_id: Uuid,
    ) -> Option<bool> {
        let notify = {
            let mut s = state.lock().await;
            let rec = match s.jobs.get_mut(&job_id) {
                Some(r) => r,
                None => return None,
            };
            if rec.status.is_terminal() {
                return Some(false);
            }
            rec.status = DomainJobStatus::Cancelled;
            let notify = rec.notify.clone();
            let event = DomainJobEvent::Cancelled(DomainJobEventCancelled {
                job_id,
                timestamp: Utc::now(),
            });
            if let Some(tx) = s.event_txs.get(&job_id) {
                let _ = tx.send(event);
            }
            s.event_txs.remove(&job_id);
            s.input_txs.remove(&job_id);
            Self::schedule_ttl_eviction(Arc::clone(state), cancel, job_id);
            notify
        };
        notify.notify_waiters();
        Some(true)
    }

    /// Remove `job_id` from state after `JOB_TTL`, unless shutdown fires first.
    /// Keeps the record alive long enough for `wait_job_result` to collect it.
    fn schedule_ttl_eviction(
        state: Arc<Mutex<MockState>>,
        cancel: &CancellationToken,
        job_id: Uuid,
    ) {
        let cancel = cancel.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = cancel.cancelled() => {}
                _ = sleep(JOB_TTL) => {
                    state.lock().await.jobs.remove(&job_id);
                }
            }
        });
    }

    /// Spawn the background task that drives a job through its lifecycle.
    fn spawn_job_task(
        state: Arc<Mutex<MockState>>,
        cancel: CancellationToken,
        job_id: Uuid,
        kind: DomainJobKind,
        needs_input_gate: bool,
    ) {
        tokio::spawn(async move {
            // ── Queued ────────────────────────────────────────────────────
            Self::transition(
                &state,
                &cancel,
                job_id,
                DomainJobStatus::Queued,
                None,
                DomainJobEvent::Queued(DomainJobEventQueued { job_id, timestamp: Utc::now() }),
            )
            .await;

            sleep(Duration::from_millis(20)).await;

            // Check if cancelled during queue wait
            {
                let s = state.lock().await;
                if let Some(r) = s.jobs.get(&job_id) {
                    if r.status.is_terminal() {
                        return;
                    }
                }
            }

            // ── Running ───────────────────────────────────────────────────
            let phase = match &kind {
                DomainJobKind::Prove(_) => Some(DomainJobPhase::Contributions),
                _ => None,
            };
            Self::transition(
                &state,
                &cancel,
                job_id,
                DomainJobStatus::Running(phase),
                None,
                DomainJobEvent::Started(DomainJobEventStarted { job_id, timestamp: Utc::now() }),
            )
            .await;

            // ── Input gate (for streaming input jobs) ─────────────────────
            if needs_input_gate {
                // Transition to WaitingForInput
                Self::transition(
                    &state,
                    &cancel,
                    job_id,
                    DomainJobStatus::WaitingForInput,
                    None,
                    DomainJobEvent::WaitingForInput(DomainJobEventWaitingForInput {
                        job_id,
                        timestamp: Utc::now(),
                    }),
                )
                .await;

                // Wait for all input chunks (input_tx is dropped when done)
                let rx = {
                    let mut s = state.lock().await;
                    s.input_txs.remove(&job_id)
                };
                // The input_tx will be dropped by push_job_input when is_last=true.
                // We just need to wait until the sender side closes.
                // We do this by waiting on a notify that push_job_input fires.
                // Actually: we re-use the job's notify — push_job_input transitions
                // the status back to Running after the last chunk.
                if rx.is_some() {
                    // Wait until status is no longer WaitingForInput
                    loop {
                        let notify = {
                            let s = state.lock().await;
                            match s.jobs.get(&job_id) {
                                Some(r) if r.status == DomainJobStatus::WaitingForInput => {
                                    r.notify.clone()
                                }
                                _ => break,
                            }
                        };
                        notify.notified().await;
                        let s = state.lock().await;
                        match s.jobs.get(&job_id) {
                            Some(r) if r.status.is_terminal() => return,
                            Some(r) if r.status != DomainJobStatus::WaitingForInput => break,
                            _ => continue,
                        }
                    }
                }

                // Back to Running after input received
                Self::transition(
                    &state,
                    &cancel,
                    job_id,
                    DomainJobStatus::Running(None),
                    None,
                    DomainJobEvent::Started(DomainJobEventStarted {
                        job_id,
                        timestamp: Utc::now(),
                    }),
                )
                .await;
            }

            // ── Phase progress (Prove jobs only) ──────────────────────────
            if let DomainJobKind::Prove(_) = &kind {
                sleep(Duration::from_millis(40)).await;

                {
                    let s = state.lock().await;
                    if let Some(r) = s.jobs.get(&job_id) {
                        if r.status.is_terminal() {
                            return;
                        }
                    }
                }

                Self::transition(
                    &state,
                    &cancel,
                    job_id,
                    DomainJobStatus::Running(Some(DomainJobPhase::Contributions)),
                    None,
                    DomainJobEvent::Progress(DomainJobEventProgress {
                        job_id,
                        phase: DomainJobPhase::Contributions,
                        timestamp: Utc::now(),
                    }),
                )
                .await;

                sleep(Duration::from_millis(40)).await;

                {
                    let s = state.lock().await;
                    if let Some(r) = s.jobs.get(&job_id) {
                        if r.status.is_terminal() {
                            return;
                        }
                    }
                }

                Self::transition(
                    &state,
                    &cancel,
                    job_id,
                    DomainJobStatus::Running(Some(DomainJobPhase::Prove)),
                    None,
                    DomainJobEvent::Progress(DomainJobEventProgress {
                        job_id,
                        phase: DomainJobPhase::Prove,
                        timestamp: Utc::now(),
                    }),
                )
                .await;
            }

            sleep(Duration::from_millis(60)).await;

            // ── Check not cancelled ───────────────────────────────────────
            {
                let s = state.lock().await;
                if let Some(r) = s.jobs.get(&job_id) {
                    if r.status.is_terminal() {
                        return;
                    }
                }
            }

            // ── Completed ─────────────────────────────────────────────────
            let result = synthesize_result(&kind);
            let event_result = result.clone();

            Self::transition(
                &state,
                &cancel,
                job_id,
                DomainJobStatus::Completed,
                Some(result),
                DomainJobEvent::Completed(DomainJobEventCompleted {
                    job_id,
                    result: event_result,
                    timestamp: Utc::now(),
                }),
            )
            .await;
        });
    }
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new(CancellationToken::new())
    }
}

#[async_trait]
impl BackendService for MockBackend {
    async fn register_guest_program(&self, elf: Vec<u8>) -> GatewayResult<String> {
        let hash_id = blake3_hex(&elf);
        let mut s = self.state.lock().await;
        let inserted = s.programs.insert(hash_id.clone());
        if inserted {
            tracing::debug!(%hash_id, elf_bytes = elf.len(), "registered guest program");
        } else {
            tracing::debug!(%hash_id, "guest program already registered (idempotent)");
        }
        Ok(hash_id)
    }

    async fn submit_job(&self, kind: DomainJobKind) -> GatewayResult<Uuid> {
        // Validate program exists for kinds that reference a hash_id
        {
            let s = self.state.lock().await;
            if let Some(hash_id) = kind.hash_id() {
                if !s.programs.contains(hash_id) {
                    return Err(GatewayError::ProgramNotFound(hash_id.to_owned()));
                }
            }
        }

        let needs_input_gate = kind.needs_input_gate();
        let job_id = Uuid::new_v4();

        // Create broadcast channel (capacity 64 — enough for all lifecycle events)
        let (event_tx, _) = broadcast::channel::<DomainJobEvent>(64);
        let notify = Arc::new(Notify::new());

        let record =
            JobRecord { status: DomainJobStatus::Queued, result: None, notify: notify.clone() };

        let (input_tx, input_rx) = if needs_input_gate {
            let (tx, rx) = mpsc::channel::<DomainInputChunk>(32);
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };

        {
            let mut s = self.state.lock().await;
            s.jobs.insert(job_id, record);
            s.event_txs.insert(job_id, event_tx);
            if let Some(tx) = input_tx {
                s.input_txs.insert(job_id, tx);
            }
        }

        // The notify-based approach (see spawn_job_task) doesn't need the rx side.
        drop(input_rx);

        Self::spawn_job_task(
            Arc::clone(&self.state),
            self.cancel.clone(),
            job_id,
            kind,
            needs_input_gate,
        );

        tracing::debug!(%job_id, "submitted job");
        Ok(job_id)
    }

    async fn wait_job_result(&self, job_id: Uuid, timeout: Duration) -> GatewayResult<WaitResult> {
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            let (status, result, notify) = {
                let s = self.state.lock().await;
                let rec = s.jobs.get(&job_id).ok_or(GatewayError::JobNotFound(job_id))?;
                (rec.status.clone(), rec.result.clone(), rec.notify.clone())
            };

            if status.is_terminal() {
                return Ok(WaitResult { job_id, job_status: status, result });
            }

            // Park until the next status change or deadline
            let notified = notify.notified();
            tokio::select! {
                _ = notified => continue,
                _ = tokio::time::sleep_until(deadline) => {
                    // Timeout: return current (non-terminal) status
                    let s = self.state.lock().await;
                    let rec = s.jobs.get(&job_id)
                        .ok_or(GatewayError::JobNotFound(job_id))?;
                    return Ok(WaitResult {
                        job_id,
                        job_status: rec.status.clone(),
                        result:     rec.result.clone(),
                    });
                }
            }
        }
    }

    async fn watch_job(&self, job_id: Uuid) -> GatewayResult<JobEventStream> {
        let (status, result, rx) = {
            let s = self.state.lock().await;
            let rec = s.jobs.get(&job_id).ok_or(GatewayError::JobNotFound(job_id))?;

            let rx = s.event_txs.get(&job_id).map(|tx| tx.subscribe());

            (rec.status.clone(), rec.result.clone(), rx)
        };

        // If already terminal, synthesize the full event history and return a
        // closed stream.
        if status.is_terminal() {
            let events = synthesize_history_events(job_id, &status, result);
            return Ok(Box::pin(stream::iter(events)));
        }

        let rx = match rx {
            Some(r) => r,
            None => return Err(GatewayError::JobNotFound(job_id)),
        };

        // Synthesize "past" events the subscriber may have missed (Queued,
        // Started, any Progress events) based on the current non-terminal status.
        // This makes watch_job reliable regardless of when the subscriber connects.
        let past = synthesize_past_events(job_id, &status);

        let watch_stream = async_stream::stream! {
            // Replay events that already happened before we subscribed.
            for event in past {
                yield event;
            }

            let mut rx = rx;
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        let is_terminal = matches!(
                            event,
                            DomainJobEvent::Completed(_)
                            | DomainJobEvent::Failed(_)
                            | DomainJobEvent::Cancelled(_)
                        );
                        yield Ok(event);
                        if is_terminal { break; }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(%job_id, skipped = n, "watch_job: subscriber lagged, events skipped");
                        // continue — we'll get subsequent events
                    }
                }
            }
        };

        Ok(Box::pin(watch_stream))
    }

    async fn push_job_input(
        &self,
        job_id: Uuid,
        mut chunks: InputChunkStream,
    ) -> GatewayResult<()> {
        // Verify the job exists and is in WaitingForInput
        {
            let s = self.state.lock().await;
            let rec = s.jobs.get(&job_id).ok_or(GatewayError::JobNotFound(job_id))?;
            if rec.status != DomainJobStatus::WaitingForInput {
                return Err(GatewayError::InvalidJobState {
                    reason: format!(
                        "job {} is not in WaitingForInput state (current: {:?})",
                        job_id, rec.status
                    ),
                });
            }
        }

        // Drain the chunk stream. When the last chunk (is_last=true) arrives,
        // transition the job back to Running.
        use futures::StreamExt;
        while let Some(chunk_result) = chunks.next().await {
            let chunk = chunk_result.map_err(|e| GatewayError::InvalidJobState {
                reason: format!("input stream error: {e}"),
            })?;
            let is_last = chunk.is_last;

            if is_last {
                // Resume the job: transition from WaitingForInput → Running
                let notify = {
                    let mut s = self.state.lock().await;
                    if let Some(rec) = s.jobs.get_mut(&job_id) {
                        rec.status = DomainJobStatus::Running(None);
                        let n = rec.notify.clone();
                        let event = DomainJobEvent::Started(DomainJobEventStarted {
                            job_id,
                            timestamp: Utc::now(),
                        });
                        if let Some(tx) = s.event_txs.get(&job_id) {
                            let _ = tx.send(event);
                        }
                        n
                    } else {
                        return Err(GatewayError::JobNotFound(job_id));
                    }
                };
                notify.notify_waiters();
                break;
            }
        }

        Ok(())
    }

    async fn cancel_job(&self, job_id: Uuid) -> GatewayResult<bool> {
        Self::do_cancel(&self.state, &self.cancel, job_id)
            .await
            .ok_or(GatewayError::JobNotFound(job_id))
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn blake3_hex(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

fn synthesize_result(kind: &DomainJobKind) -> DomainJobKindResponse {
    match kind {
        DomainJobKind::Setup(_) => DomainJobKindResponse::Setup,
        DomainJobKind::Execute(_) => DomainJobKindResponse::Execute {
            stats: DomainExecutionStats::default(),
            public_outputs: vec![],
        },
        DomainJobKind::Prove(req) => DomainJobKindResponse::Prove {
            proof: DomainProof {
                proof_id: Uuid::new_v4(),
                hash_id: req.hash_id.clone(),
                verification_key: vec![0u8; 32],
                proof_kind: DomainProofKind::Stark,
                data: vec![1u8; 64],
                public_inputs: vec![2u8; 32],
                started_at: Utc::now(),
                completed_at: Utc::now(),
            },
            stats: DomainExecutionStats::default(),
        },
        DomainJobKind::Wrap(req) => {
            let mut proof = req.proof.clone();
            proof.proof_id = Uuid::new_v4();
            proof.proof_kind = req.proof_dest.clone();
            DomainJobKindResponse::Wrap(proof)
        }
    }
}

fn synthesize_terminal_event(
    job_id: Uuid,
    status: &DomainJobStatus,
    result: Option<DomainJobKindResponse>,
) -> DomainJobEvent {
    let now = Utc::now();
    match status {
        DomainJobStatus::Completed => DomainJobEvent::Completed(DomainJobEventCompleted {
            job_id,
            result: result.unwrap_or(DomainJobKindResponse::Execute {
                stats: DomainExecutionStats::default(),
                public_outputs: vec![],
            }),
            timestamp: now,
        }),
        DomainJobStatus::Cancelled => {
            DomainJobEvent::Cancelled(DomainJobEventCancelled { job_id, timestamp: now })
        }
        DomainJobStatus::Failed(f) => DomainJobEvent::Failed(DomainJobEventFailed {
            job_id,
            failure: f.clone(),
            timestamp: now,
        }),
        _ => unreachable!("synthesize_terminal_event called on non-terminal status"),
    }
}

/// Build the full ordered event history for a job that has reached a terminal
/// state. Returns `Ok(event)` entries so they can be used directly in a stream.
fn synthesize_history_events(
    job_id: Uuid,
    status: &DomainJobStatus,
    result: Option<DomainJobKindResponse>,
) -> Vec<GatewayResult<DomainJobEvent>> {
    let mut events = synthesize_past_events(job_id, &DomainJobStatus::Running(None));
    events.push(Ok(synthesize_terminal_event(job_id, status, result)));
    events
}

/// Build "past" events for a job that is still in progress, so that a late
/// subscriber sees the events it missed before it connected.
fn synthesize_past_events(
    job_id: Uuid,
    status: &DomainJobStatus,
) -> Vec<GatewayResult<DomainJobEvent>> {
    let now = Utc::now();
    match status {
        DomainJobStatus::Queued => {
            vec![Ok(DomainJobEvent::Queued(DomainJobEventQueued { job_id, timestamp: now }))]
        }
        DomainJobStatus::Running(_) | DomainJobStatus::WaitingForInput => vec![
            Ok(DomainJobEvent::Queued(DomainJobEventQueued { job_id, timestamp: now })),
            Ok(DomainJobEvent::Started(DomainJobEventStarted { job_id, timestamp: now })),
        ],
        // Terminal states are handled separately via synthesize_history_events.
        _ => vec![],
    }
}

// ── Extension helpers on DomainJobKind ───────────────────────────────────────

trait JobKindExt {
    fn hash_id(&self) -> Option<&str>;
    fn needs_input_gate(&self) -> bool;
}

impl JobKindExt for DomainJobKind {
    fn hash_id(&self) -> Option<&str> {
        match self {
            DomainJobKind::Setup(r) => Some(&r.hash_id),
            DomainJobKind::Prove(r) => Some(&r.hash_id),
            DomainJobKind::Execute(r) => Some(&r.hash_id),
            DomainJobKind::Wrap(_) => None,
        }
    }

    fn needs_input_gate(&self) -> bool {
        let inline_not_last = |input: &DomainInputKind| {
            matches!(
                input,
                DomainInputKind::Inline(c) if !c.is_last
            )
        };
        match self {
            DomainJobKind::Prove(r) => inline_not_last(&r.input),
            DomainJobKind::Execute(r) => inline_not_last(&r.input),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        DomainExecuteRequest, DomainJobFailure, DomainProveRequest, DomainSetupRequest,
        DomainWrapRequest,
    };
    use super::*;
    use std::time::Duration;

    fn dummy_hash(backend: &MockBackend) -> String {
        // Register a dummy program synchronously via blocking
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { backend.register_guest_program(vec![0u8; 16]).await.unwrap() })
        })
    }

    #[tokio::test]
    async fn register_idempotent() {
        let b = MockBackend::default();
        let elf = vec![1u8, 2, 3];
        let h1 = b.register_guest_program(elf.clone()).await.unwrap();
        let h2 = b.register_guest_program(elf).await.unwrap();
        assert_eq!(h1, h2);
    }

    #[tokio::test]
    async fn setup_job_completes() {
        let b = MockBackend::default();
        let hash_id = b.register_guest_program(vec![0u8; 8]).await.unwrap();
        let job_id =
            b.submit_job(DomainJobKind::Setup(DomainSetupRequest { hash_id })).await.unwrap();

        let result = b.wait_job_result(job_id, Duration::from_secs(5)).await.unwrap();
        assert_eq!(result.job_status, DomainJobStatus::Completed);
    }

    #[tokio::test]
    async fn cancel_running_job() {
        let b = MockBackend::default();
        let hash_id = b.register_guest_program(vec![0u8; 8]).await.unwrap();
        let job_id = b
            .submit_job(DomainJobKind::Prove(DomainProveRequest {
                hash_id,
                input: DomainInputKind::Inline(DomainInputChunk { data: vec![], is_last: true }),
                proof_timeout: None,
            }))
            .await
            .unwrap();

        // Cancel before it completes
        let cancelled = b.cancel_job(job_id).await.unwrap();
        assert!(cancelled);

        // Cancelling again is idempotent
        let again = b.cancel_job(job_id).await.unwrap();
        assert!(!again);
    }

    #[tokio::test]
    async fn cancel_completed_job_returns_false() {
        let b = MockBackend::default();
        let hash_id = b.register_guest_program(vec![0u8; 8]).await.unwrap();
        let job_id = b
            .submit_job(DomainJobKind::Execute(DomainExecuteRequest {
                hash_id,
                input: DomainInputKind::Inline(DomainInputChunk { data: vec![], is_last: true }),
                execute_timeout: None,
            }))
            .await
            .unwrap();

        // Wait for completion
        b.wait_job_result(job_id, Duration::from_secs(5)).await.unwrap();

        let cancelled = b.cancel_job(job_id).await.unwrap();
        assert!(!cancelled);
    }

    #[tokio::test]
    async fn program_not_found_error() {
        let b = MockBackend::default();
        let err = b
            .submit_job(DomainJobKind::Setup(DomainSetupRequest { hash_id: "nonexistent".into() }))
            .await
            .unwrap_err();
        assert!(matches!(err, GatewayError::ProgramNotFound(_)));
    }

    #[tokio::test]
    async fn job_not_found_error() {
        let b = MockBackend::default();
        let fake_id = Uuid::new_v4();
        let err = b.wait_job_result(fake_id, Duration::from_millis(100)).await.unwrap_err();
        assert!(matches!(err, GatewayError::JobNotFound(_)));
    }

    #[tokio::test]
    async fn wrap_job_produces_proof() {
        use chrono::Utc;
        let b = MockBackend::default();
        let src_proof = DomainProof {
            proof_id: Uuid::new_v4(),
            hash_id: "h".into(),
            verification_key: vec![],
            proof_kind: DomainProofKind::Stark,
            data: vec![],
            public_inputs: vec![],
            started_at: Utc::now(),
            completed_at: Utc::now(),
        };
        let job_id = b
            .submit_job(DomainJobKind::Wrap(DomainWrapRequest {
                proof: src_proof,
                proof_dest: DomainProofKind::Plonk,
                wrap_timeout: None,
            }))
            .await
            .unwrap();

        let result = b.wait_job_result(job_id, Duration::from_secs(5)).await.unwrap();
        assert_eq!(result.job_status, DomainJobStatus::Completed);
        assert!(matches!(result.result, Some(DomainJobKindResponse::Wrap(_))));
    }
}
