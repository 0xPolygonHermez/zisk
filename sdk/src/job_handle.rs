use std::future::Future;
use std::future::IntoFuture;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use bytes::Bytes;
use serde::Serialize;
use zisk_coordinator_api::dto::{
    DomainExecutionStats, DomainJobEvent, DomainJobFailure, DomainJobKindResponse, DomainJobPhase,
    TerminalStatus,
};
use zisk_coordinator_client::{InputSender, Job, WatchHandle};

use crate::input_stream::ZiskStream;
use crate::prove::JobEvent;
use crate::setup::SetupResult;

const CANCELLED: &str = "Cancelled";

const PROGRESS_CONTRIBUTIONS: u8 = 25;
const PROGRESS_PROVE: u8 = 75;
const PROGRESS_AGGREGATE: u8 = 90;

pub(crate) type Subscriber = (JobEvent, Arc<dyn Fn(JobEvent) + Send + Sync>);
pub(crate) type SubscriberList = Arc<Mutex<Vec<Subscriber>>>;
pub(crate) type PreProcessHook = Box<dyn FnOnce(&TerminalStatus) -> Result<()> + Send>;

pub(crate) fn fire_event(subscribers: &SubscriberList, event: JobEvent) {
    let matching: Vec<Arc<dyn Fn(JobEvent) + Send + Sync>> = match subscribers.lock() {
        Ok(subs) => subs
            .iter()
            .filter(|(filter, _)| *filter == JobEvent::All || *filter == event)
            .map(|(_, cb)| Arc::clone(cb))
            .collect(),
        Err(_) => return,
    };
    for cb in matching {
        cb(event.clone());
    }
}

pub(crate) fn fire_result_event<T>(subs: &SubscriberList, result: &Result<T>) {
    match result {
        Ok(_) => fire_event(subs, JobEvent::Completed),
        Err(e) => fire_event(subs, JobEvent::Failed(e.to_string())),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct JobId(pub(crate) String);

impl From<String> for JobId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<JobId> for String {
    fn from(id: JobId) -> Self {
        id.0
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Implemented by every type that can be produced from a remote job result.
pub(crate) trait FromWaitResult: Sized + Send + 'static {
    fn from_terminal(status: TerminalStatus, job_id: JobId) -> Result<Self>;
}

pub(crate) enum JobHandleInner<T> {
    Embedded(tokio::task::JoinHandle<Result<T>>),
    Remote { remote_job: Job, _watch_handle: WatchHandle },
}

/// Handle to an in-flight job (embedded or remote).
///
/// Obtained by calling `.run()` on a request builder.
/// Await the handle to get the result: `let proof = handle.await?`.
#[must_use = "JobHandle does nothing unless awaited"]
pub struct JobHandle<T> {
    pub(crate) inner: Option<JobHandleInner<T>>,
    pub(crate) subscribers: SubscriberList,
    pub(crate) timeout: Option<Duration>,
    input_sender: Option<InputSender>,
    pre_process: Option<PreProcessHook>,
    /// Stream to finish automatically when the handle is awaited.
    stream: Option<ZiskStream>,
}

impl<T> JobHandle<T> {
    pub fn new_embedded(
        handle: tokio::task::JoinHandle<Result<T>>,
        subscribers: SubscriberList,
        timeout: Option<Duration>,
    ) -> Self {
        Self {
            inner: Some(JobHandleInner::Embedded(handle)),
            subscribers,
            timeout,
            input_sender: None,
            pre_process: None,
            stream: None,
        }
    }

    pub fn new_remote(
        remote_job: Job,
        subscribers: SubscriberList,
        timeout: Option<Duration>,
        stream: Option<ZiskStream>,
    ) -> Self {
        let subs_watch = Arc::clone(&subscribers);
        let watch_handle =
            remote_job.spawn_watch(move |event| map_domain_event(&subs_watch, &event));
        Self {
            inner: Some(JobHandleInner::Remote { remote_job, _watch_handle: watch_handle }),
            subscribers,
            timeout,
            input_sender: None,
            pre_process: None,
            stream,
        }
    }

    pub(crate) fn set_pre_process(&mut self, f: impl FnOnce(&TerminalStatus) -> Result<()> + Send + 'static) {
        self.pre_process = Some(Box::new(f));
    }

    /// Register a post-submission event callback.
    ///
    /// Use [`JobEvent::All`] to subscribe to all events.
    pub fn on(
        &mut self,
        event: JobEvent,
        cb: impl Fn(JobEvent) + Send + Sync + 'static,
    ) -> &mut Self {
        if let Ok(mut subs) = self.subscribers.lock() {
            subs.push((event, Arc::new(cb)));
        }
        self
    }
}

impl<T> JobHandle<T> {
    /// Cancel the in-flight job.
    ///
    /// - Embedded: aborts the blocking task (the thread runs to completion but the
    ///   result is discarded; awaiting the handle will return an error).
    /// - Remote: calls the coordinator `CancelJob` RPC and waits until the job reaches
    ///   a terminal state. Returns `Ok(true)` if cancelled, `Ok(false)` if it had
    ///   already reached a terminal state before the request arrived.
    pub async fn cancel(&mut self) -> Result<bool> {
        match &self.inner {
            Some(JobHandleInner::Embedded(_handle)) => {
                unimplemented!("cancelling embedded jobs is not supported yet")
            }
            Some(JobHandleInner::Remote { remote_job, .. }) => remote_job.cancel_async().await,
            None => anyhow::bail!("cannot cancel: JobHandle already consumed"),
        }
    }

    /// Push input data to a running remote job.
    ///
    /// On the first call, a persistent gRPC input stream is opened. Subsequent
    /// calls reuse the same stream. Large payloads (> 3 MB) are automatically
    /// split into multiple gRPC messages without copying.
    ///
    /// Returns an error for embedded jobs (not supported) or if the handle has
    /// already been consumed.
    pub async fn push_input<S: Serialize>(&mut self, data: &S) -> Result<()> {
        // Lazy-init the InputSender on first push
        if self.input_sender.is_none() {
            match &self.inner {
                Some(JobHandleInner::Embedded(_)) => {
                    anyhow::bail!("push_input is not supported for embedded jobs")
                }
                Some(JobHandleInner::Remote { remote_job, .. }) => {
                    self.input_sender = Some(remote_job.open_input_stream());
                }
                None => anyhow::bail!("cannot push_input: JobHandle already consumed"),
            }
        }

        // Frame the data in ZiskStdin wire format:
        // [8 bytes: length as u64 LE] [N bytes: bincode payload] [padding to 8-byte align]
        let payload = bincode::serialize(data)?;
        let data_len = payload.len();
        let total_len = 8 + data_len;
        let padding = (8 - (total_len % 8)) % 8;

        let mut buf = Vec::with_capacity(total_len + padding);
        buf.extend_from_slice(&data_len.to_le_bytes());
        buf.extend_from_slice(&payload);
        if padding > 0 {
            buf.extend_from_slice(&vec![0u8; padding]);
        }

        self.input_sender.as_ref().unwrap().send(Bytes::from(buf)).await
    }

    /// Close the input stream, signalling EOF to the coordinator.
    ///
    /// If no input was ever pushed, this is a no-op. After calling `close_input`,
    /// further calls to [`push_input`](Self::push_input) will fail.
    pub async fn close_input(&mut self) -> Result<()> {
        if let Some(sender) = self.input_sender.take() {
            sender.close().await?;
        }
        Ok(())
    }
}

#[allow(private_bounds)]
impl<T: FromWaitResult> JobHandle<T> {
    async fn await_embedded(
        handle: tokio::task::JoinHandle<Result<T>>,
        timeout: Option<Duration>,
    ) -> Result<T> {
        let join = |h: tokio::task::JoinHandle<Result<T>>| async {
            h.await.map_err(|e| anyhow::anyhow!("task panicked: {e}"))?
        };
        match timeout {
            Some(dur) => tokio::time::timeout(dur, join(handle))
                .await
                .map_err(|_| anyhow::anyhow!("job timed out after {dur:?}"))?,
            None => join(handle).await,
        }
    }

    async fn await_remote(
        remote_job: Job,
        timeout: Option<Duration>,
        subscribers: SubscriberList,
        pre_process: Option<PreProcessHook>,
    ) -> Result<T> {
        let job_id = JobId(remote_job.job_id().to_string());
        let terminal = remote_job.wait_async(timeout).await?;

        // Fire terminal event from the authoritative WaitJobResult response.
        match &terminal {
            TerminalStatus::Completed(_) => fire_event(&subscribers, JobEvent::Completed),
            TerminalStatus::Failed(f) => {
                fire_event(&subscribers, JobEvent::Failed(format_failure(f)));
            }
            TerminalStatus::Cancelled => {
                fire_event(&subscribers, JobEvent::Failed(CANCELLED.to_string()));
            }
        }

        if let Some(hook) = pre_process {
            hook(&terminal)?;
        }

        T::from_terminal(terminal, job_id)
    }
}

impl<T: Send + 'static + FromWaitResult> IntoFuture for JobHandle<T> {
    type Output = Result<T>;
    type IntoFuture = Pin<Box<dyn Future<Output = Result<T>> + Send>>;

    fn into_future(mut self) -> Self::IntoFuture {
        let inner = self.inner.take().expect("JobHandle already consumed");
        let timeout = self.timeout;
        let subscribers = Arc::clone(&self.subscribers);
        let pre_process = self.pre_process.take();
        let stream = self.stream.take();
        Box::pin(async move {
            let result = match inner {
                JobHandleInner::Embedded(handle) => Self::await_embedded(handle, timeout).await,
                JobHandleInner::Remote { remote_job, _watch_handle } => {
                    // _watch_handle is kept alive until await_remote completes,
                    // then dropped (aborting the watch task).
                    Self::await_remote(remote_job, timeout, subscribers, pre_process).await
                }
            };
            // Automatically close the stream so any flush() before the next
            // run() blocks safely instead of sending to the completed job.
            if let Some(s) = stream {
                let _ = s.finish_async().await;
            }
            result
        })
    }
}

// ── Domain event → SDK event mapping ──────────────────────────────────────────

/// Map a domain event to an SDK event and fire it to subscribers.
/// Returns `true` for terminal events (to stop the watch stream).
fn map_domain_event(subs: &SubscriberList, event: &DomainJobEvent) -> bool {
    match event {
        DomainJobEvent::Queued(_) | DomainJobEvent::WaitingForInput(_) => false,
        DomainJobEvent::Started(_) => {
            fire_event(subs, JobEvent::Started);
            false
        }
        DomainJobEvent::Progress(p) => {
            let pct = match p.phase {
                DomainJobPhase::Contributions => PROGRESS_CONTRIBUTIONS,
                DomainJobPhase::Prove => PROGRESS_PROVE,
                DomainJobPhase::Aggregate => PROGRESS_AGGREGATE,
            };
            fire_event(subs, JobEvent::Progress(pct));
            false
        }
        // Terminal events are fired authoritatively from the WaitJobResult response;
        // returning true here stops the watch stream without double-firing.
        DomainJobEvent::Completed(_) | DomainJobEvent::Cancelled(_) | DomainJobEvent::Failed(_) => {
            true
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn format_failure(failure: &DomainJobFailure) -> String {
    match failure {
        DomainJobFailure::Timeout { phase, limit } => {
            format!("Timeout (phase: {:?}, limit: {:?})", phase, limit)
        }
        DomainJobFailure::Input { reason } => format!("Input error: {}", reason),
        DomainJobFailure::Execution { reason } => format!("Execution error: {}", reason),
        DomainJobFailure::Internal { trace_id } => {
            format!("Internal error (trace_id: {})", trace_id)
        }
        DomainJobFailure::Cancelled => CANCELLED.to_string(),
    }
}

fn domain_stats_to_cost(stats: &DomainExecutionStats) -> zisk_common::StatsCostPerType {
    zisk_common::StatsCostPerType {
        main_cost: stats.main_cost,
        opcode_cost: stats.opcode_cost,
        memory_cost: stats.memory_cost,
        precompile_cost: stats.precompile_cost,
        tables_cost: stats.tables_cost,
        other_cost: stats.other_cost,
    }
}

// ── FromWaitResult impls ──────────────────────────────────────────────────────

impl FromWaitResult for SetupResult {
    fn from_terminal(status: TerminalStatus, job_id: JobId) -> Result<Self> {
        match status {
            TerminalStatus::Completed(DomainJobKindResponse::Setup { .. }) => {
                Ok(SetupResult { job_id: Some(job_id) })
            }
            TerminalStatus::Completed(other) => {
                anyhow::bail!("unexpected response kind for setup: {:?}", other)
            }
            TerminalStatus::Failed(f) => anyhow::bail!(format_failure(&f)),
            TerminalStatus::Cancelled => anyhow::bail!("job was cancelled"),
        }
    }
}

impl FromWaitResult for crate::prove::ProveResult {
    fn from_terminal(status: TerminalStatus, job_id: JobId) -> Result<Self> {
        match status {
            TerminalStatus::Completed(DomainJobKindResponse::Prove { proof, stats }) => {
                let proof_with_pv: zisk_common::Proof = bincode::deserialize(&proof.data)
                    .map_err(|e| anyhow::anyhow!("failed to deserialize proof: {e}"))?;
                let output = zisk_prover_backend::ProveOutput::from_remote(
                    proof_with_pv,
                    stats.steps,
                    Duration::from_nanos(stats.duration_nanos),
                    domain_stats_to_cost(&stats),
                );
                Ok(crate::prove::ProveResult::new(output, Some(job_id)))
            }
            TerminalStatus::Completed(DomainJobKindResponse::Wrap(proof)) => {
                let proof_with_pv: zisk_common::Proof = bincode::deserialize(&proof.data)
                    .map_err(|e| anyhow::anyhow!("failed to deserialize wrapped proof: {e}"))?;
                let output = zisk_prover_backend::ProveOutput::from_remote(
                    proof_with_pv,
                    0,
                    Duration::ZERO,
                    zisk_common::StatsCostPerType::default(),
                );
                Ok(crate::prove::ProveResult::new(output, Some(job_id)))
            }
            TerminalStatus::Completed(other) => {
                anyhow::bail!("unexpected job kind response for prove/wrap: {:?}", other)
            }
            TerminalStatus::Failed(f) => anyhow::bail!(format_failure(&f)),
            TerminalStatus::Cancelled => anyhow::bail!("job was cancelled"),
        }
    }
}

impl FromWaitResult for crate::execute::ExecuteResult {
    fn from_terminal(status: TerminalStatus, job_id: JobId) -> Result<Self> {
        match status {
            TerminalStatus::Completed(DomainJobKindResponse::Execute { stats, public_outputs }) => {
                let output = zisk_prover_backend::ExecuteOutput::from_remote(
                    stats.steps,
                    Duration::from_nanos(stats.duration_nanos),
                    domain_stats_to_cost(&stats),
                    &public_outputs,
                );
                Ok(crate::execute::ExecuteResult::new(output, Some(job_id)))
            }
            TerminalStatus::Completed(other) => {
                anyhow::bail!("unexpected job kind response for execute: {:?}", other)
            }
            TerminalStatus::Failed(f) => anyhow::bail!(format_failure(&f)),
            TerminalStatus::Cancelled => anyhow::bail!("job was cancelled"),
        }
    }
}
