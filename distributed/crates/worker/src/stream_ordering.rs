use anyhow::Result;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::mpsc;
use std::sync::Arc;
use tracing::{error, info};
use zisk_cluster_common::{JobId, StreamDataDto, StreamMessageKind};
use zisk_common::io::StreamProcessor;
use zisk_common::reinterpret_vec;

/// Per-job actor that reorders out-of-order stream chunks and feeds them
/// to `HintsProcessor::process_hints` in strict sequence order.
pub struct StreamOrderingActor {
    sender: Option<mpsc::Sender<StreamDataDto>>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl StreamOrderingActor {
    /// Spawns the ordering thread and returns the actor handle.
    pub fn new<P: StreamProcessor>(processor: Arc<P>, job_id: JobId) -> Self {
        let (tx, rx) = mpsc::channel::<StreamDataDto>();

        let handle = std::thread::spawn(move || Self::run(rx, processor, job_id));

        Self { sender: Some(tx), thread_handle: Some(handle) }
    }

    /// Enqueues a stream message for ordered delivery to `process_hints`.
    ///
    /// This call is non-blocking and safe to invoke from an async context.
    pub fn send(&self, msg: StreamDataDto) -> Result<()> {
        self.sender
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Stream ordering actor already shut down"))?
            .send(msg)
            .map_err(|_| anyhow::anyhow!("Stream ordering actor channel closed unexpectedly"))
    }

    // Error propagation: when run_inner returns Err, rx is dropped, closing the channel.
    // The next actor.send() in the gRPC loop then returns Err.
    fn run<P: StreamProcessor>(
        rx: mpsc::Receiver<StreamDataDto>,
        processor: Arc<P>,
        job_id: JobId,
    ) {
        if let Err(e) = Self::run_inner(rx, &*processor, &job_id) {
            error!("Stream ordering actor failed for job {}: {}", job_id, e);
        }
    }

    fn run_inner<P: StreamProcessor>(
        rx: mpsc::Receiver<StreamDataDto>,
        processor: &P,
        job_id: &JobId,
    ) -> Result<()> {
        // Min-heap ordered by sequence number (Reverse makes BinaryHeap a min-heap)
        let mut heap: BinaryHeap<Reverse<(u32, Vec<u8>)>> = BinaryHeap::new();
        let mut next_seq: u32 = 1;
        let mut is_first = true;

        loop {
            match rx.recv() {
                Ok(msg) => match msg.stream_type {
                    StreamMessageKind::End => {
                        if !heap.is_empty() {
                            return Err(anyhow::anyhow!(
                                "Stream End received for job {} but {} buffered chunk(s) remain \
                                 (next expected seq: {}). Sequence gap detected.",
                                job_id,
                                heap.len(),
                                next_seq
                            ));
                        }
                        info!("Stream ordering actor: received End for job {}", job_id);
                        return Ok(());
                    }
                    StreamMessageKind::Data => {
                        let payload_dto = msg.stream_payload.ok_or_else(|| {
                            anyhow::anyhow!("Data message missing payload for job {}", job_id)
                        })?;

                        heap.push(Reverse((payload_dto.sequence_number, payload_dto.payload)));

                        // Drain all consecutive in-order sequences from the heap,
                        // accumulating their bytes into a single buffer so that
                        // process_hints is called exactly once per recv() iteration.
                        if !matches!(heap.peek(), Some(Reverse((s, _))) if *s == next_seq) {
                            continue;
                        }
                        let mut combined: Vec<u8> = Vec::new();
                        while matches!(heap.peek(), Some(Reverse((s, _))) if *s == next_seq) {
                            let Reverse((_, data)) = heap.pop().unwrap();
                            combined.extend_from_slice(&data);
                            next_seq += 1;
                        }

                        let hints = reinterpret_vec(combined)?;
                        let first = std::mem::replace(&mut is_first, false);
                        processor.process_hints(&hints, first)?;
                    }
                    StreamMessageKind::Start => {
                        return Err(anyhow::anyhow!(
                            "Unexpected Start message received mid-stream for job {}",
                            job_id
                        ));
                    }
                },
                Err(_) => {
                    // Channel closed — sender was dropped (job cancelled or complete)
                    info!("Stream ordering actor: channel closed for job {}", job_id);
                    return Ok(());
                }
            }
        }
    }
}

/// Returned by [`StreamOrderingActor::shutdown_and_join`] on timeout. The
/// thread is detached and may still hold `HintsShmem` locks; the caller must
/// exit the process.
#[must_use]
#[derive(Debug)]
pub struct ShutdownTimeout {
    pub timeout: std::time::Duration,
}

impl std::fmt::Display for ShutdownTimeout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StreamOrderingActor: shutdown timed out after {:?}", self.timeout)
    }
}

impl std::error::Error for ShutdownTimeout {}

impl StreamOrderingActor {
    /// Closes the input channel and joins the worker thread within `timeout`.
    /// On timeout, detaches the thread and returns `Err`; callers must exit.
    pub fn shutdown_and_join(
        mut self,
        timeout: std::time::Duration,
    ) -> std::result::Result<(), ShutdownTimeout> {
        self.sender.take();

        let Some(handle) = self.thread_handle.take() else {
            return Ok(());
        };

        let deadline = std::time::Instant::now() + timeout;
        while !handle.is_finished() {
            if std::time::Instant::now() >= deadline {
                tracing::error!(
                    "StreamOrderingActor: shutdown timed out after {:?}; thread still running",
                    timeout
                );
                drop(handle);
                return Err(ShutdownTimeout { timeout });
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let _ = handle.join();
        Ok(())
    }
}

impl Drop for StreamOrderingActor {
    fn drop(&mut self) {
        // Drop the sender first so the thread's recv() returns Err and exits.
        self.sender.take();

        // Reaching here with a live handle means `shutdown_and_join` was never
        // called. The thread may still hold HintsShmem locks; abort if it
        // doesn't exit promptly so the next job can't race it.
        let Some(handle) = self.thread_handle.take() else { return };

        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
        while !handle.is_finished() {
            if std::time::Instant::now() >= deadline {
                tracing::error!(
                    "StreamOrderingActor::drop: worker thread still running 500ms after \
                     channel closed; aborting to prevent next-job race"
                );
                std::process::abort();
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let _ = handle.join();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

    struct BlockingProcessor {
        release: Arc<AtomicBool>,
    }

    impl StreamProcessor for BlockingProcessor {
        fn process_hints(&self, _data: &[u64], _first_batch: bool) -> Result<bool> {
            while !self.release.load(AtomicOrdering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Ok(false)
        }
    }

    #[test]
    fn shutdown_timeout_returns_err() {
        let release = Arc::new(AtomicBool::new(false));
        let processor = Arc::new(BlockingProcessor { release: release.clone() });
        let actor = StreamOrderingActor::new(processor, JobId::from("test-job".to_string()));

        actor
            .send(StreamDataDto {
                job_id: JobId::from("test-job".to_string()),
                stream_type: StreamMessageKind::Data,
                stream_payload: Some(zisk_cluster_common::StreamPayloadDto {
                    sequence_number: 1,
                    payload: vec![0u8; 8],
                }),
            })
            .unwrap();
        // Let the worker thread dequeue and start blocking before we shut down.
        std::thread::sleep(std::time::Duration::from_millis(50));

        let err = actor
            .shutdown_and_join(std::time::Duration::from_millis(100))
            .expect_err("expected timeout error while processor is blocked");
        assert!(err.to_string().contains("shutdown timed out"));

        release.store(true, AtomicOrdering::SeqCst);
    }

    #[test]
    fn shutdown_clean_returns_ok() {
        struct NoopProcessor;
        impl StreamProcessor for NoopProcessor {
            fn process_hints(&self, _data: &[u64], _first_batch: bool) -> Result<bool> {
                Ok(false)
            }
        }
        let actor =
            StreamOrderingActor::new(Arc::new(NoopProcessor), JobId::from("clean-job".to_string()));
        actor
            .shutdown_and_join(std::time::Duration::from_secs(5))
            .expect("clean shutdown must succeed");
    }
}
