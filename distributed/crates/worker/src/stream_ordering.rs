use anyhow::Result;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::mpsc;
use std::sync::Arc;
use tracing::{error, info};
use zisk_common::io::StreamProcessor;
use zisk_common::reinterpret_vec;
use zisk_distributed_common::{JobId, StreamDataDto, StreamMessageKind};

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

impl Drop for StreamOrderingActor {
    fn drop(&mut self) {
        // Drop the sender first so the thread's recv() returns Err and exits
        self.sender.take();

        // Drop the ordering thread, it will terminate promptly once the channel is closed.
        self.thread_handle.take();
    }
}
