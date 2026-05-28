use anyhow::Result;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::mpsc;
use std::sync::Arc;
use tracing::{error, info};
use zisk_cluster_common::{JobId, StreamDataDto, StreamMessageKind};
use zisk_common::io::StreamProcessor;
use zisk_common::reinterpret_vec;

// ZDIAG: hang-instrumentation - remove after diagnosis
static ZDIAG_ACTOR_NEW_SEQ: AtomicU64 = AtomicU64::new(0);
static ZDIAG_ACTOR_PROCESS_SEQ: AtomicU64 = AtomicU64::new(0);
static ZDIAG_ACTOR_SHUTDOWN_SEQ: AtomicU64 = AtomicU64::new(0);

/// Per-job actor that reorders out-of-order stream chunks and feeds them
/// to `HintsProcessor::process_hints` in strict sequence order.
pub struct StreamOrderingActor {
    sender: Option<mpsc::Sender<StreamDataDto>>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl StreamOrderingActor {
    /// Spawns the ordering thread and returns the actor handle.
    pub fn new<P: StreamProcessor>(processor: Arc<P>, job_id: JobId) -> Self {
        // ZDIAG: a new actor while the OLD actor's thread might still be running process_hints
        let _zd_seq = ZDIAG_ACTOR_NEW_SEQ.fetch_add(1, AtomicOrdering::Relaxed);
        eprintln!(
            "[ZDIAG ACTOR-NEW] seq={} pid={} tid={:?} job_id={}",
            _zd_seq, std::process::id(), std::thread::current().id(), job_id
        );

        let (tx, rx) = mpsc::channel::<StreamDataDto>();

        let job_id_for_thread = job_id.clone();
        let handle = std::thread::spawn(move || {
            eprintln!(
                "[ZDIAG ACTOR-THREAD-START] pid={} tid={:?} job_id={}",
                std::process::id(), std::thread::current().id(), job_id_for_thread
            );
            Self::run(rx, processor, job_id_for_thread.clone());
            eprintln!(
                "[ZDIAG ACTOR-THREAD-EXIT] pid={} tid={:?} job_id={}",
                std::process::id(), std::thread::current().id(), job_id_for_thread
            );
        });

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
                        // ZDIAG: throttled — every 200th + first_batch + slow + err
                        let _zd_pseq = ZDIAG_ACTOR_PROCESS_SEQ.fetch_add(1, AtomicOrdering::Relaxed);
                        let _zd_pstart = std::time::Instant::now();
                        let _zd_log_entry = _zd_pseq % 200 == 0 || first;
                        if _zd_log_entry {
                            eprintln!(
                                "[ZDIAG ACTOR-PROC-ENTER] seq={} pid={} tid={:?} job_id={} hints_u64_len={} first_batch={}",
                                _zd_pseq, std::process::id(), std::thread::current().id(),
                                job_id, hints.len(), first
                            );
                        }
                        let result = processor.process_hints(&hints, first);
                        let _zd_pms = _zd_pstart.elapsed().as_millis();
                        if result.is_err() || _zd_pms > 50 || _zd_pseq % 200 == 0 {
                            eprintln!(
                                "[ZDIAG ACTOR-PROC-EXIT] seq={} pid={} tid={:?} job_id={} elapsed_ms={} ok={}",
                                _zd_pseq, std::process::id(), std::thread::current().id(),
                                job_id, _zd_pms, result.is_ok()
                            );
                        }
                        result?;
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

impl StreamOrderingActor {
    pub fn shutdown_and_join(mut self, timeout: std::time::Duration) {
        let _zd_seq = ZDIAG_ACTOR_SHUTDOWN_SEQ.fetch_add(1, AtomicOrdering::Relaxed);
        let _zd_start = std::time::Instant::now();
        eprintln!(
            "[ZDIAG ACTOR-SHUTDOWN-ENTER] seq={} pid={} tid={:?} timeout_ms={}",
            _zd_seq, std::process::id(), std::thread::current().id(),
            timeout.as_millis()
        );

        self.sender.take();

        let Some(handle) = self.thread_handle.take() else {
            eprintln!(
                "[ZDIAG ACTOR-SHUTDOWN-NOHANDLE] seq={} pid={} tid={:?}",
                _zd_seq, std::process::id(), std::thread::current().id()
            );
            return;
        };

        // Bounded join: poll is_finished until timeout, then detach.
        let deadline = std::time::Instant::now() + timeout;
        while !handle.is_finished() {
            if std::time::Instant::now() >= deadline {
                eprintln!(
                    "[ZDIAG ACTOR-SHUTDOWN-TIMEOUT] seq={} pid={} tid={:?} timeout_ms={} (DETACHING — thread still running, possible race with new actor)",
                    _zd_seq, std::process::id(), std::thread::current().id(),
                    timeout.as_millis()
                );
                tracing::warn!(
                    "StreamOrderingActor: shutdown timed out after {:?}; detaching thread",
                    timeout
                );
                return; // detach
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let _ = handle.join();
        eprintln!(
            "[ZDIAG ACTOR-SHUTDOWN-EXIT] seq={} pid={} tid={:?} elapsed_ms={}",
            _zd_seq, std::process::id(), std::thread::current().id(),
            _zd_start.elapsed().as_millis()
        );
    }
}

impl Drop for StreamOrderingActor {
    fn drop(&mut self) {
        // Drop the sender first so the thread's recv() returns Err and exits.
        self.sender.take();

        // Detach the ordering thread; it will terminate promptly once the channel
        // is closed. Callers that need to *wait* for shutdown should use
        // `shutdown_and_join` before dropping.
        self.thread_handle.take();
    }
}
