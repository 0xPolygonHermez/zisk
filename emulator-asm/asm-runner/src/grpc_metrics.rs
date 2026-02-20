use std::sync::atomic::{AtomicU64, Ordering};

/// Global gRPC health counters.
///
/// Incremented by the worker's stream-handling code and read by `AsmRunnerMT`
/// when a semaphore timeout fires so we can correlate a stalled ASM with gRPC
/// activity (or lack thereof) at the same moment.
pub struct GrpcMetrics {
    /// Number of hint streams that have been started but not yet ended.
    pub active_streams: AtomicU64,
    /// Total `StreamData::Data` messages forwarded to the ordering actor.
    pub callbacks_invoked: AtomicU64,
    /// Number of times the Tokio event-loop was delayed by more than the
    /// starvation threshold (detected by a periodic canary task in the worker).
    pub thread_starvation_count: AtomicU64,
}

impl GrpcMetrics {
    const fn new() -> Self {
        Self {
            active_streams: AtomicU64::new(0),
            callbacks_invoked: AtomicU64::new(0),
            thread_starvation_count: AtomicU64::new(0),
        }
    }

    pub fn diagnose_grpc_health(&self) {
        println!("=== GRPC METRICS ===");
        println!("Active streams: {}", self.active_streams.load(Ordering::Relaxed));
        println!("Callbacks invoked: {}", self.callbacks_invoked.load(Ordering::Relaxed));
        println!(
            "Thread starvation events: {}",
            self.thread_starvation_count.load(Ordering::Relaxed)
        );
    }
}

pub static GRPC_METRICS: GrpcMetrics = GrpcMetrics::new();
