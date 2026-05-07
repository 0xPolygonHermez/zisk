use anyhow::{Context, Result};
use bytes::Bytes;
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use uuid::Uuid;
use zisk_coordinator_api::grpc::proto::{
    InputChunk, PushJobHintsInputRequest, PushJobInputRequest,
};
use zisk_coordinator_api::grpc::ZiskCoordinatorApiClient;

/// Maximum gRPC message payload per chunk (~3 MB, well under the 4 MB default).
const MAX_CHUNK_BYTES: usize = 3 * 1024 * 1024;

/// Persistent sender for streaming input data to a running job.
///
/// Data sent via [`send`](Self::send) is forwarded to the coordinator through a
/// gRPC `PushJobInput` client-streaming RPC.  Large payloads are automatically
/// split into ≤ 3 MB chunks (zero-copy via [`Bytes::slice`]).
///
/// The stream is closed (EOF) when the `InputSender` is dropped or
/// [`close`](Self::close) is called explicitly.
pub struct InputSender {
    job_id: Uuid,
    tx: Option<mpsc::Sender<Bytes>>,
    task: Option<JoinHandle<Result<()>>>,
}

impl InputSender {
    /// Open a new stdin stream to the coordinator for `job_id`.
    pub(crate) fn open(
        job_id: Uuid,
        mut client: ZiskCoordinatorApiClient<tonic::transport::Channel>,
    ) -> Self {
        let (tx, rx) = mpsc::channel::<Bytes>(16);

        let task = tokio::spawn(async move {
            let job_id_str = job_id.to_string();
            let stream = tokio_stream::wrappers::ReceiverStream::new(rx).map(move |data| {
                PushJobInputRequest {
                    job_id: job_id_str.clone(),
                    chunk: Some(InputChunk { data: data.to_vec() }),
                }
            });

            client
                .push_job_input(stream)
                .await
                .map_err(|e| anyhow::anyhow!("PushJobInput RPC failed: {e}"))?;

            Ok(())
        });

        Self { job_id, tx: Some(tx), task: Some(task) }
    }

    /// Open a new hints stream to the coordinator for `job_id`.
    pub(crate) fn open_hints(
        job_id: Uuid,
        mut client: ZiskCoordinatorApiClient<tonic::transport::Channel>,
    ) -> Self {
        let (tx, rx) = mpsc::channel::<Bytes>(16);

        let task = tokio::spawn(async move {
            let job_id_str = job_id.to_string();
            let stream = tokio_stream::wrappers::ReceiverStream::new(rx).map(move |data| {
                PushJobHintsInputRequest {
                    job_id: job_id_str.clone(),
                    chunk: Some(InputChunk { data: data.to_vec() }),
                }
            });

            client
                .push_job_hints_input(stream)
                .await
                .map_err(|e| anyhow::anyhow!("PushJobHintsInput RPC failed: {e}"))?;

            Ok(())
        });

        Self { job_id, tx: Some(tx), task: Some(task) }
    }

    pub fn job_id(&self) -> Uuid {
        self.job_id
    }

    /// Send input data to the job.  Large payloads (> 3 MB) are automatically
    /// split into multiple gRPC messages without copying.
    pub async fn send(&self, data: impl Into<Bytes>) -> Result<()> {
        let tx = self.tx.as_ref().context("InputSender already closed")?;
        let data: Bytes = data.into();

        if data.is_empty() {
            return Ok(());
        }

        if data.len() <= MAX_CHUNK_BYTES {
            tx.send(data).await.map_err(|_| anyhow::anyhow!("input stream closed"))?;
        } else {
            // Auto-chunk: zero-copy slicing via Bytes::slice
            let mut offset = 0;
            while offset < data.len() {
                let end = (offset + MAX_CHUNK_BYTES).min(data.len());
                let chunk = data.slice(offset..end);
                tx.send(chunk).await.map_err(|_| anyhow::anyhow!("input stream closed"))?;
                offset = end;
            }
        }

        Ok(())
    }

    /// Close the input stream and wait for the RPC to finish.
    ///
    /// Dropping the sender without calling `close` also closes the stream,
    /// but errors from the RPC completion are silently ignored.
    pub async fn close(mut self) -> Result<()> {
        // Drop sender → receiver closes → stream ends → EOF
        self.tx.take();

        if let Some(task) = self.task.take() {
            task.await.context("input stream task panicked")??;
        }

        Ok(())
    }
}

impl Drop for InputSender {
    fn drop(&mut self) {
        // Dropping tx closes the channel → stream ends → EOF on the server.
        // The background task will complete on its own.
        self.tx.take();
        // Don't await the task in drop — just let it finish asynchronously.
    }
}

/// Adapter that lets [`InputSender`] plug into [`zisk_common::io::ZiskStreamWriter`]'s
/// `Push` transport variant.
///
/// `ZiskStreamWriter` calls the [`BytesPushSender`] trait synchronously; this
/// adapter bridges to `InputSender`'s async API by capturing a
/// [`tokio::runtime::Handle`] at construction and using `block_on`
/// (with `block_in_place` when the caller is already on a runtime thread).
///
/// Construct from a tokio runtime context — `Handle::current()` will panic
/// otherwise.
pub struct InputSenderPushAdapter {
    sender: tokio::sync::Mutex<Option<InputSender>>,
    rt: tokio::runtime::Handle,
}

impl InputSenderPushAdapter {
    /// Wrap an `InputSender`. Captures the current tokio runtime handle for
    /// later sync-from-async dispatch — must be called from inside a tokio
    /// runtime.
    pub fn new(sender: InputSender) -> Self {
        Self {
            sender: tokio::sync::Mutex::new(Some(sender)),
            rt: tokio::runtime::Handle::current(),
        }
    }
}

impl zisk_common::io::BytesPushSender for InputSenderPushAdapter {
    fn send_blocking(&self, data: Vec<u8>) -> anyhow::Result<()> {
        let bytes = Bytes::from(data);
        let send = async {
            let guard = self.sender.lock().await;
            let sender =
                guard.as_ref().ok_or_else(|| anyhow::anyhow!("InputSender already closed"))?;
            sender.send(bytes).await
        };
        match tokio::runtime::Handle::try_current() {
            Ok(_) => tokio::task::block_in_place(|| self.rt.block_on(send)),
            Err(_) => self.rt.block_on(send),
        }
    }

    fn close_blocking(self: Box<Self>) -> anyhow::Result<()> {
        let rt = self.rt.clone();
        let close = async move {
            let mut guard = self.sender.lock().await;
            if let Some(sender) = guard.take() {
                sender.close().await
            } else {
                Ok(())
            }
        };
        match tokio::runtime::Handle::try_current() {
            Ok(_) => tokio::task::block_in_place(|| rt.block_on(close)),
            Err(_) => rt.block_on(close),
        }
    }
}
