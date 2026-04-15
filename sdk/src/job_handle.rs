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

const CANCELLED: &str = "Cancelled";
const WAIT_TIMEOUT_DEFAULT_SECS: u32 = 3600;
const WAIT_TIMEOUT_MIN_SECS: u32 = 1;
const WAIT_TIMEOUT_MAX_SECS: u32 = 3600;

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

pub(crate) enum JobHandleInner<T> {
    Embedded(JoinHandle<Result<T>>),
    Remote {
        gateway: ZiskGatewayApiClient<Channel>,
        job_id: JobId,
        extract: Box<dyn FnOnce(WaitJobResultResponse) -> Result<T> + Send>,
    },
}

/// Handle to an in-flight job (embedded or remote).
///
/// Obtained by calling `.run()` on a request builder.
/// Await the handle to get the result: `let proof = handle.await?`.
#[must_use = "JobHandle does nothing unless awaited"]
pub struct JobHandle<T> {
    pub(crate) inner: JobHandleInner<T>,
    pub(crate) subscribers: SubscriberList,
    pub(crate) timeout: Option<Duration>,
}

impl<T> JobHandle<T> {
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

    /// Cancel the in-flight job.
    ///
    /// - Embedded: aborts the blocking task (the thread runs to completion but the
    ///   result is discarded; awaiting the handle will return an error).
    /// - Remote: calls the gateway `CancelJob` RPC and waits until the job reaches
    ///   a terminal state. Returns `Ok(true)` if cancelled, `Ok(false)` if it had
    ///   already reached a terminal state before the request arrived.
    pub async fn cancel(&mut self) -> Result<bool> {
        match &self.inner {
            JobHandleInner::Embedded(_handle) => {
                unimplemented!("cancelling embedded jobs is not supported yet")
            }
            JobHandleInner::Remote { gateway, job_id, .. } => {
                let mut gw = gateway.clone();
                let resp = gw
                    .cancel_job(CancelJobRequest { job_id: job_id.clone().into() })
                    .await
                    .map_err(|e| anyhow::anyhow!("CancelJob RPC failed: {e}"))?;
                Ok(resp.into_inner().cancelled)
            }
        }
    }
}

impl<T: Send + 'static> IntoFuture for JobHandle<T> {
    type Output = Result<T>;
    type IntoFuture = Pin<Box<dyn Future<Output = Result<T>> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            match self.inner {
                JobHandleInner::Embedded(handle) => {
                    if let Some(dur) = self.timeout {
                        tokio::time::timeout(dur, handle)
                            .await
                            .map_err(|_| anyhow::anyhow!("job timed out after {dur:?}"))?
                            .map_err(|e| anyhow::anyhow!("task panicked: {e}"))?
                    } else {
                        handle.await.map_err(|e| anyhow::anyhow!("task panicked: {e}"))?
                    }
                }
                JobHandleInner::Remote { mut gateway, job_id, extract } => {
                    let timeout_secs = self
                        .timeout
                        .map(|d| {
                            d.as_secs()
                                .clamp(WAIT_TIMEOUT_MIN_SECS as u64, WAIT_TIMEOUT_MAX_SECS as u64)
                                as u32
                        })
                        .unwrap_or(WAIT_TIMEOUT_DEFAULT_SECS);

                    let subs_watch = Arc::clone(&self.subscribers);
                    let mut watch_gw = gateway.clone();
                    let jid = job_id.clone();
                    let watch_task = tokio::spawn(async move {
                        if let Ok(resp) =
                            watch_gw.watch_job(WatchJobRequest { job_id: jid.into() }).await
                        {
                            let mut stream = resp.into_inner();
                            while let Some(Ok(event)) = stream.next().await {
                                if map_and_fire_event(&subs_watch, event) {
                                    break;
                                }
                            }
                        }
                    });

                    let resp = gateway
                        .wait_job_result(WaitJobResultRequest {
                            job_id: job_id.into(),
                            timeout_seconds: Some(timeout_secs),
                        })
                        .await
                        .map_err(|e| anyhow::anyhow!("WaitJobResult failed: {e}"))?;

                    watch_task.abort();

                    let inner = resp.into_inner();

                    if let Some(status) = &inner.job_status {
                        match &status.status {
                            Some(JobStatusVariant::Completed(_)) => {
                                fire_event(&self.subscribers, JobEvent::Completed);
                            }
                            Some(JobStatusVariant::Failed(f)) => {
                                fire_event(
                                    &self.subscribers,
                                    JobEvent::Failed(format_failure(f.failure.as_ref())),
                                );
                            }
                            Some(JobStatusVariant::Cancelled(_)) => {
                                fire_event(
                                    &self.subscribers,
                                    JobEvent::Failed(CANCELLED.to_string()),
                                );
                            }
                            _ => {}
                        }
                    }

                    extract(inner)
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
pub(crate) fn check_completed(resp: &WaitJobResultResponse) -> Result<SetupResult> {
    match resp.job_status.as_ref().and_then(|s| s.status.as_ref()) {
        Some(JobStatusVariant::Completed(_)) => Ok(SetupResult),
        Some(JobStatusVariant::Failed(f)) => anyhow::bail!(format_failure(f.failure.as_ref())),
        Some(JobStatusVariant::Cancelled(_)) => anyhow::bail!("job was cancelled"),
        other => anyhow::bail!("unexpected terminal status: {:?}", other),
    }
}

/// Extract `Proof` from a prove job result.
pub(crate) fn extract_prove(resp: WaitJobResultResponse) -> Result<crate::proof::Proof> {
    check_completed(&resp)?;

    let result =
        resp.result.ok_or_else(|| anyhow::anyhow!("missing result in WaitJobResultResponse"))?;
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

/// Extract `ExecuteResult` from an execute job result.
pub(crate) fn extract_execute(
    resp: WaitJobResultResponse,
) -> Result<crate::execute::ExecuteResult> {
    check_completed(&resp)?;

    let result =
        resp.result.ok_or_else(|| anyhow::anyhow!("missing result in WaitJobResultResponse"))?;
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

/// Extract `ZiskProofWithPublicValues` from a wrap job result.
pub(crate) fn extract_wrap(
    resp: WaitJobResultResponse,
) -> Result<zisk_common::ZiskProofWithPublicValues> {
    check_completed(&resp)?;

    let result =
        resp.result.ok_or_else(|| anyhow::anyhow!("missing result in WaitJobResultResponse"))?;
    match result.kind {
        Some(KindResponse::Wrap(wrap_resp)) => {
            let proof_msg =
                wrap_resp.proof.ok_or_else(|| anyhow::anyhow!("missing proof in WrapResponse"))?;
            bincode::deserialize(&proof_msg.data)
                .map_err(|e| anyhow::anyhow!("failed to deserialize wrapped proof: {e}"))
        }
        other => anyhow::bail!("unexpected job kind response for wrap: {:?}", other),
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
