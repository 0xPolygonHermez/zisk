use std::future::Future;
use std::future::IntoFuture;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use futures::StreamExt;
use tokio::task::JoinHandle;
use tonic::transport::Channel;
use zisk_gateway_grpc_api::{
    proto::{
        job_event::Event as GatewayEvent, job_failure::Kind as FailureKind,
        job_kind_response::Kind as KindResponse, job_status::Status as JobStatusVariant,
        CancelJobRequest, JobEvent as GatewayJobEvent, JobFailure, WaitJobResultRequest,
        WaitJobResultResponse, WatchJobRequest,
    },
    ZiskGatewayApiClient,
};

use crate::prove::JobEvent;
use crate::remote::JobId;
use crate::setup::SetupResult;
use crate::Proof;

const CANCELLED: &str = "Cancelled";
/// Per-call hold duration sent to the gateway's WaitJobResult long-poll.
/// The gateway returns early as soon as the job reaches a terminal state,
/// so this only controls how often we re-poll when the job is still running.
const WAIT_POLL_SECS: u32 = 30;

pub(crate) type Subscriber = (JobEvent, Arc<dyn Fn(JobEvent) + Send + Sync>);
pub(crate) type SubscriberList = Arc<Mutex<Vec<Subscriber>>>;

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

/// Implemented by every type that can be produced from a remote job result.
///
/// The impl lives in `job_handle` (not in the individual result types) because the
/// conversion requires gateway proto imports that should not leak into domain types.
pub(crate) trait FromJobResult: Sized + Send + 'static {
    fn from_job_result(resp: WaitJobResultResponse) -> Result<Self>;
}

pub(crate) enum JobHandleInner<T> {
    Embedded(JoinHandle<Result<T>>),
    Remote { gateway: ZiskGatewayApiClient<Channel>, job_id: JobId, watch_task: JoinHandle<()> },
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
}

impl<T> JobHandle<T> {
    pub fn new_embedded(
        handle: JoinHandle<Result<T>>,
        subscribers: SubscriberList,
        timeout: Option<Duration>,
    ) -> Self {
        Self { inner: Some(JobHandleInner::Embedded(handle)), subscribers, timeout }
    }

    pub fn new_remote(
        gateway: ZiskGatewayApiClient<Channel>,
        job_id: JobId,
        subscribers: SubscriberList,
        timeout: Option<Duration>,
    ) -> Self {
        let mut watch_gw = gateway.clone();
        let watch_jid = job_id.clone();
        let subs_watch = Arc::clone(&subscribers);
        let watch_task = tokio::spawn(async move {
            if let Ok(resp) = watch_gw.watch_job(WatchJobRequest { job_id: watch_jid.into() }).await
            {
                let mut stream = resp.into_inner();
                while let Some(Ok(event)) = stream.next().await {
                    if map_and_fire_event(&subs_watch, event) {
                        break;
                    }
                }
            }
        });
        Self {
            inner: Some(JobHandleInner::Remote { gateway, job_id, watch_task }),
            subscribers,
            timeout,
        }
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

impl<T> Drop for JobHandle<T> {
    fn drop(&mut self) {
        if let Some(JobHandleInner::Remote { watch_task, .. }) = &self.inner {
            watch_task.abort();
        }
    }
}

impl<T> JobHandle<T> {
    /// Cancel the in-flight job.
    ///
    /// - Embedded: aborts the blocking task (the thread runs to completion but the
    ///   result is discarded; awaiting the handle will return an error).
    /// - Remote: calls the gateway `CancelJob` RPC and waits until the job reaches
    ///   a terminal state. Returns `Ok(true)` if cancelled, `Ok(false)` if it had
    ///   already reached a terminal state before the request arrived.
    pub async fn cancel(&mut self) -> Result<bool> {
        match &self.inner {
            Some(JobHandleInner::Embedded(_handle)) => {
                unimplemented!("cancelling embedded jobs is not supported yet")
            }
            Some(JobHandleInner::Remote { gateway, job_id, .. }) => {
                let mut gw = gateway.clone();
                let resp = gw
                    .cancel_job(CancelJobRequest { job_id: job_id.clone().into() })
                    .await
                    .map_err(|e| anyhow::anyhow!("CancelJob RPC failed: {e}"))?;
                Ok(resp.into_inner().cancelled)
            }
            None => anyhow::bail!("cannot cancel: JobHandle already consumed"),
        }
    }
}

#[allow(private_bounds)]
impl<T: FromJobResult> JobHandle<T> {
    async fn await_embedded(handle: JoinHandle<Result<T>>, timeout: Option<Duration>) -> Result<T> {
        if let Some(dur) = timeout {
            tokio::time::timeout(dur, handle)
                .await
                .map_err(|_| anyhow::anyhow!("job timed out after {dur:?}"))?
                .map_err(|e| anyhow::anyhow!("task panicked: {e}"))?
        } else {
            handle.await.map_err(|e| anyhow::anyhow!("task panicked: {e}"))?
        }
    }

    /// WaitJobResult is a long-poll: the gateway holds the request for up to WAIT_POLL_SECS
    /// and returns early when the job finishes. If it returns a non-terminal status we loop
    /// until the job completes or the deadline is exceeded.
    async fn await_remote(
        mut gateway: ZiskGatewayApiClient<Channel>,
        job_id: JobId,
        watch_task: JoinHandle<()>,
        timeout: Option<Duration>,
        subscribers: SubscriberList,
    ) -> Result<T> {
        let deadline = timeout.map(|d| tokio::time::Instant::now() + d);

        let inner = loop {
            if let Some(dl) = deadline {
                if tokio::time::Instant::now() >= dl {
                    watch_task.abort();
                    anyhow::bail!("job timed out after {:?}", timeout.unwrap());
                }
            }

            let resp = match gateway
                .wait_job_result(WaitJobResultRequest {
                    job_id: job_id.clone().into(),
                    timeout_seconds: Some(WAIT_POLL_SECS),
                })
                .await
            {
                Ok(r) => r.into_inner(),
                Err(e) => {
                    watch_task.abort();
                    return Err(anyhow::anyhow!("WaitJobResult failed: {e}"));
                }
            };

            match resp.job_status.as_ref().and_then(|s| s.status.as_ref()) {
                Some(JobStatusVariant::Completed(_))
                | Some(JobStatusVariant::Failed(_))
                | Some(JobStatusVariant::Cancelled(_)) => break resp,
                _ => continue,
            }
        };

        watch_task.abort();

        if let Some(status) = &inner.job_status {
            match &status.status {
                Some(JobStatusVariant::Completed(_)) => {
                    fire_event(&subscribers, JobEvent::Completed);
                }
                Some(JobStatusVariant::Failed(f)) => {
                    fire_event(&subscribers, JobEvent::Failed(format_failure(f.failure.as_ref())));
                }
                Some(JobStatusVariant::Cancelled(_)) => {
                    fire_event(&subscribers, JobEvent::Failed(CANCELLED.to_string()));
                }
                _ => {}
            }
        }

        T::from_job_result(inner)
    }
}

impl<T: Send + 'static + FromJobResult> IntoFuture for JobHandle<T> {
    type Output = Result<T>;
    type IntoFuture = Pin<Box<dyn Future<Output = Result<T>> + Send>>;

    fn into_future(mut self) -> Self::IntoFuture {
        let inner = self.inner.take().expect("JobHandle already consumed");
        let timeout = self.timeout;
        let subscribers = Arc::clone(&self.subscribers);
        Box::pin(async move {
            match inner {
                JobHandleInner::Embedded(handle) => Self::await_embedded(handle, timeout).await,
                JobHandleInner::Remote { gateway, job_id, watch_task } => {
                    Self::await_remote(gateway, job_id, watch_task, timeout, subscribers).await
                }
            }
        })
    }
}

fn map_and_fire_event(subs: &SubscriberList, event: GatewayJobEvent) -> bool {
    match event.event {
        Some(GatewayEvent::Queued(_)) | Some(GatewayEvent::WaitingForInput(_)) | None => false,
        Some(GatewayEvent::Started(_)) => {
            fire_event(subs, JobEvent::Started);
            false
        }
        Some(GatewayEvent::Progress(p)) => {
            let pct = match p.phase() {
                zisk_gateway_grpc_api::proto::JobPhase::Contributions => 25,
                zisk_gateway_grpc_api::proto::JobPhase::Prove => 75,
                zisk_gateway_grpc_api::proto::JobPhase::Aggregate => 90,
                _ => 0,
            };
            fire_event(subs, JobEvent::Progress(pct));
            false
        }
        // Terminal events are fired authoritatively from the WaitJobResult response below;
        // returning true here stops the watch stream without double-firing the event.
        Some(GatewayEvent::Completed(_))
        | Some(GatewayEvent::Cancelled(_))
        | Some(GatewayEvent::Failed(_)) => true,
    }
}

fn format_failure(failure: Option<&JobFailure>) -> String {
    let Some(f) = failure else {
        return "Unknown failure".to_string();
    };
    match &f.kind {
        Some(FailureKind::Timeout(t)) => {
            format!("Timeout (phase: {:?}, limit: {:?})", t.phase, t.limit)
        }
        Some(FailureKind::Input(i)) => format!("Input error: {}", i.reason),
        Some(FailureKind::Execution(e)) => format!("Execution error: {}", e.reason),
        Some(FailureKind::Internal(i)) => format!("Internal error (trace_id: {})", i.trace_id),
        Some(FailureKind::Cancelled(_)) => CANCELLED.to_string(),
        None => "Unknown failure".to_string(),
    }
}

/// Assert that a `WaitJobResultResponse` is in a completed terminal state.
pub(crate) fn check_completed(resp: &WaitJobResultResponse) -> Result<()> {
    match resp.job_status.as_ref().and_then(|s| s.status.as_ref()) {
        Some(JobStatusVariant::Completed(_)) => Ok(()),
        Some(JobStatusVariant::Failed(f)) => anyhow::bail!(format_failure(f.failure.as_ref())),
        Some(JobStatusVariant::Cancelled(_)) => anyhow::bail!("job was cancelled"),
        other => anyhow::bail!("unexpected terminal status: {:?}", other),
    }
}

// FromJobResult impls for each type that can be returned by a remote job.
// Each impl extracts the relevant data from the WaitJobResultResponse and converts it into the appropriate Rust type.
impl FromJobResult for SetupResult {
    fn from_job_result(resp: WaitJobResultResponse) -> Result<Self> {
        check_completed(&resp)?;
        Ok(SetupResult)
    }
}

impl FromJobResult for Proof {
    fn from_job_result(resp: WaitJobResultResponse) -> Result<Self> {
        check_completed(&resp)?;
        let result = resp
            .result
            .ok_or_else(|| anyhow::anyhow!("missing result in WaitJobResultResponse"))?;
        match result.kind {
            Some(KindResponse::Prove(prove_resp)) => {
                let proof_msg = prove_resp
                    .proof
                    .ok_or_else(|| anyhow::anyhow!("missing proof in ProveResponse"))?;
                let proof_with_pv: zisk_common::ZiskProofWithPublicValues =
                    bincode::deserialize(&proof_msg.data)
                        .map_err(|e| anyhow::anyhow!("failed to deserialize proof: {e}"))?;
                let (steps, duration, cost) = proto_stats_to_rust(prove_resp.stats);
                let result = zisk_prover_backend::ZiskProveResult::from_remote(
                    proof_with_pv,
                    steps,
                    duration,
                    cost,
                );
                Ok(crate::proof::Proof::new(result))
            }
            other => anyhow::bail!("unexpected job kind response for prove: {:?}", other),
        }
    }
}

impl FromJobResult for crate::execute::ExecuteResult {
    fn from_job_result(resp: WaitJobResultResponse) -> Result<Self> {
        check_completed(&resp)?;
        let result = resp
            .result
            .ok_or_else(|| anyhow::anyhow!("missing result in WaitJobResultResponse"))?;
        match result.kind {
            Some(KindResponse::Execute(execute_resp)) => {
                let (steps, duration, cost) = proto_stats_to_rust(execute_resp.stats);
                let inner = zisk_prover_backend::ZiskExecuteResult::from_remote(
                    steps,
                    duration,
                    cost,
                    &execute_resp.public_outputs,
                );
                Ok(crate::execute::ExecuteResult::new(inner))
            }
            other => anyhow::bail!("unexpected job kind response for execute: {:?}", other),
        }
    }
}

impl FromJobResult for zisk_common::ZiskProofWithPublicValues {
    fn from_job_result(resp: WaitJobResultResponse) -> Result<Self> {
        check_completed(&resp)?;
        let result = resp
            .result
            .ok_or_else(|| anyhow::anyhow!("missing result in WaitJobResultResponse"))?;
        match result.kind {
            Some(KindResponse::Wrap(wrap_resp)) => {
                let proof_msg = wrap_resp
                    .proof
                    .ok_or_else(|| anyhow::anyhow!("missing proof in WrapResponse"))?;
                bincode::deserialize(&proof_msg.data)
                    .map_err(|e| anyhow::anyhow!("failed to deserialize wrapped proof: {e}"))
            }
            other => anyhow::bail!("unexpected job kind response for wrap: {:?}", other),
        }
    }
}

fn proto_stats_to_rust(
    stats: Option<zisk_gateway_grpc_api::proto::ExecutionStats>,
) -> (u64, Duration, zisk_common::StatsCostPerType) {
    let stats = stats.unwrap_or_default();
    let cost = stats.cost_per_type.unwrap_or_default();
    let sct = zisk_common::StatsCostPerType {
        main_cost: cost.main,
        opcode_cost: cost.opcode,
        memory_cost: cost.memory,
        precompile_cost: cost.precompile,
        tables_cost: cost.tables,
        other_cost: cost.other,
    };
    (stats.steps, Duration::from_nanos(stats.duration_nanos), sct)
}
