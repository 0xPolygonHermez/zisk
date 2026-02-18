//! Ordered Result Buffer
//!
//! A thread-safe buffer that allows out-of-order insertion but guarantees
//! in-order consumption. Used for parallel processing where results must
//! be output in the original submission order.

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

/// Status of the buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferStatus {
    /// Normal operation
    Active,
    /// Shutdown requested - no more results expected
    Shutdown,
    /// Error occurred - stop processing
    Error,
}

/// Internal state protected by mutex
struct State<T> {
    /// Ring buffer of slots: None = pending, Some = ready
    buffer: VecDeque<Option<T>>,
    /// Sequence ID of the next result to drain (buffer[0] corresponds to this)
    next_drain: usize,
    /// Current status
    status: BufferStatus,
}

/// A thread-safe buffer for reordering results from parallel workers.
///
/// Workers can fill slots out of order using sequence IDs, but consumers
/// always receive results in the original order.
///
/// # Example
///
/// ```ignore
/// let buffer = OrderedResultBuffer::new();
///
/// // Producer: reserve slots and fill (possibly out of order)
/// let seq1 = buffer.reserve();  // 0
/// let seq2 = buffer.reserve();  // 1
/// buffer.fill(seq2, "second");  // Fill out of order
/// buffer.fill(seq1, "first");
///
/// // Consumer: always gets results in order
/// assert_eq!(buffer.take_next(), Some("first"));
/// assert_eq!(buffer.take_next(), Some("second"));
/// ```
pub struct OrderedResultBuffer<T> {
    inner: Mutex<State<T>>,
    signal: Condvar,
}

impl<T> OrderedResultBuffer<T> {
    /// Create a new empty buffer.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(State {
                buffer: VecDeque::new(),
                next_drain: 0,
                status: BufferStatus::Active,
            }),
            signal: Condvar::new(),
        }
    }

    /// Reserve a slot in the buffer and return its sequence ID.
    ///
    /// The caller must later call `fill()` with this sequence ID.
    pub fn reserve(&self) -> usize {
        let mut state = self.inner.lock().unwrap();
        let seq = state.next_drain + state.buffer.len();
        state.buffer.push_back(None);
        seq
    }

    /// Reserve a slot and immediately fill it with a value.
    ///
    /// Equivalent to `reserve()` followed by `fill()`, but atomic.
    /// Useful for passthrough items that don't need async processing.
    pub fn reserve_and_fill(&self, value: T) -> usize {
        let mut state = self.inner.lock().unwrap();
        let seq = state.next_drain + state.buffer.len();
        state.buffer.push_back(Some(value));
        // Notify while holding lock to prevent race
        self.signal.notify_all();
        seq
    }

    /// Fill a previously reserved slot with a value.
    ///
    /// # Panics
    ///
    /// Panics if `seq_id` was not reserved or was already filled.
    pub fn fill(&self, seq_id: usize, value: T) {
        let mut state = self.inner.lock().unwrap();

        // Calculate offset in buffer
        let offset = seq_id.checked_sub(state.next_drain);
        let offset = match offset {
            Some(o) if o < state.buffer.len() => o,
            _ => {
                // Slot was already drained or invalid - ignore
                // This can happen after reset() if stale workers complete
                return;
            }
        };

        // Fill the slot
        debug_assert!(state.buffer[offset].is_none(), "Slot {} already filled", seq_id);
        state.buffer[offset] = Some(value);

        // Notify while holding lock to prevent race condition
        self.signal.notify_all();
    }

    /// Take the next in-order result, blocking until one is ready.
    ///
    /// Returns `None` if the buffer is shutdown or in error state
    /// and no more results are available.
    pub fn take_next(&self) -> Option<T> {
        let mut state = self.inner.lock().unwrap();

        loop {
            // Check if front slot has a ready result
            if let Some(Some(_)) = state.buffer.front() {
                // Pop and return the value
                let value = state.buffer.pop_front().unwrap().unwrap();
                state.next_drain += 1;
                // Notify wait_until_drained that buffer changed
                self.signal.notify_all();
                return Some(value);
            }

            // No ready result - check if we should stop waiting
            match state.status {
                BufferStatus::Active => {
                    // Wait for notification
                    state = self.signal.wait(state).unwrap();
                }
                BufferStatus::Shutdown | BufferStatus::Error => {
                    // Check one more time after status change
                    if let Some(Some(_)) = state.buffer.front() {
                        let value = state.buffer.pop_front().unwrap().unwrap();
                        state.next_drain += 1;
                        self.signal.notify_all();
                        return Some(value);
                    }
                    return None;
                }
            }
        }
    }

    /// Wait until all reserved slots have been drained.
    ///
    /// Returns the final status (Active means all drained, Error/Shutdown
    /// means stopped early).
    pub fn wait_until_drained(&self) -> BufferStatus {
        let mut state = self.inner.lock().unwrap();

        loop {
            if state.buffer.is_empty() {
                return BufferStatus::Active;
            }

            if state.status == BufferStatus::Error {
                return BufferStatus::Error;
            }

            state = self.signal.wait(state).unwrap();
        }
    }

    /// Signal an error condition. Wakes all waiting threads.
    pub fn set_error(&self) {
        let mut state = self.inner.lock().unwrap();
        state.status = BufferStatus::Error;
        self.signal.notify_all();
    }

    /// Signal shutdown. Wakes all waiting threads.
    pub fn shutdown(&self) {
        let mut state = self.inner.lock().unwrap();
        state.status = BufferStatus::Shutdown;
        self.signal.notify_all();
    }

    /// Get current status.
    pub fn status(&self) -> BufferStatus {
        self.inner.lock().unwrap().status
    }

    /// Check if buffer is empty (all results drained).
    pub fn is_empty(&self) -> bool {
        self.inner.lock().unwrap().buffer.is_empty()
    }

    /// Get count of pending/ready slots.
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().buffer.len()
    }

    /// Reset the buffer for reuse.
    ///
    /// Clears all pending slots and resets sequence counter.
    pub fn reset(&self) {
        let mut state = self.inner.lock().unwrap();
        state.buffer.clear();
        state.next_drain = 0;
        state.status = BufferStatus::Active;
    }
}

impl<T> Default for OrderedResultBuffer<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn test_basic_order() {
        let buf = OrderedResultBuffer::new();

        let seq0 = buf.reserve();
        let seq1 = buf.reserve();
        let seq2 = buf.reserve();

        assert_eq!(seq0, 0);
        assert_eq!(seq1, 1);
        assert_eq!(seq2, 2);

        // Fill out of order
        buf.fill(seq2, "third");
        buf.fill(seq0, "first");
        buf.fill(seq1, "second");

        // Take in order
        assert_eq!(buf.take_next(), Some("first"));
        assert_eq!(buf.take_next(), Some("second"));
        assert_eq!(buf.take_next(), Some("third"));
    }

    #[test]
    fn test_reserve_and_fill() {
        let buf = OrderedResultBuffer::new();

        buf.reserve_and_fill("immediate");
        let seq = buf.reserve();
        buf.fill(seq, "delayed");

        assert_eq!(buf.take_next(), Some("immediate"));
        assert_eq!(buf.take_next(), Some("delayed"));
    }

    #[test]
    fn test_blocking_take() {
        let buf = Arc::new(OrderedResultBuffer::new());
        let seq = buf.reserve();

        let buf_clone = Arc::clone(&buf);
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            buf_clone.fill(seq, 42);
        });

        let start = Instant::now();
        let value = buf.take_next();
        let elapsed = start.elapsed();

        assert_eq!(value, Some(42));
        assert!(elapsed >= Duration::from_millis(40));

        handle.join().unwrap();
    }

    #[test]
    fn test_out_of_order_parallel() {
        let buf = Arc::new(OrderedResultBuffer::new());
        let num_items = 100;

        // Reserve all slots
        let seqs: Vec<_> = (0..num_items).map(|_| buf.reserve()).collect();

        // Fill from multiple threads in reverse order
        let handles: Vec<_> = seqs
            .into_iter()
            .rev()
            .map(|seq| {
                let buf = Arc::clone(&buf);
                thread::spawn(move || {
                    buf.fill(seq, seq * 10);
                })
            })
            .collect();

        // Drain and verify order
        for i in 0..num_items {
            let value = buf.take_next().unwrap();
            assert_eq!(value, i * 10);
        }

        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn test_shutdown_wakes_waiter() {
        let buf = Arc::new(OrderedResultBuffer::<i32>::new());
        buf.reserve(); // Reserve but don't fill

        let buf_clone = Arc::clone(&buf);
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            buf_clone.shutdown();
        });

        let start = Instant::now();
        let result = buf.take_next();
        let elapsed = start.elapsed();

        assert!(result.is_none());
        assert!(elapsed >= Duration::from_millis(40));
        assert_eq!(buf.status(), BufferStatus::Shutdown);

        handle.join().unwrap();
    }

    #[test]
    fn test_error_wakes_waiter() {
        let buf = Arc::new(OrderedResultBuffer::<i32>::new());
        buf.reserve();

        let buf_clone = Arc::clone(&buf);
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            buf_clone.set_error();
        });

        let result = buf.take_next();
        assert!(result.is_none());
        assert_eq!(buf.status(), BufferStatus::Error);

        handle.join().unwrap();
    }

    #[test]
    fn test_wait_until_drained() {
        let buf = Arc::new(OrderedResultBuffer::new());

        // Reserve and fill slots
        for i in 0..5 {
            buf.reserve_and_fill(i);
        }

        // Drain in another thread
        let buf_clone = Arc::clone(&buf);
        let drainer = thread::spawn(move || while let Some(_) = buf_clone.take_next() {});

        // Wait for drain
        let status = buf.wait_until_drained();
        assert_eq!(status, BufferStatus::Active);
        assert!(buf.is_empty());

        buf.shutdown();
        drainer.join().unwrap();
    }

    #[test]
    fn test_reset() {
        let buf = OrderedResultBuffer::new();

        buf.reserve_and_fill(1);
        buf.reserve_and_fill(2);
        buf.set_error();

        assert_eq!(buf.status(), BufferStatus::Error);
        assert_eq!(buf.len(), 2);

        buf.reset();

        assert_eq!(buf.status(), BufferStatus::Active);
        assert!(buf.is_empty());

        // Can use again
        buf.reserve_and_fill(100);
        assert_eq!(buf.take_next(), Some(100));
    }

    #[test]
    fn test_stale_fill_after_reset() {
        let buf = OrderedResultBuffer::new();

        let seq = buf.reserve();
        buf.reset();

        // Stale fill should be ignored (not panic)
        buf.fill(seq, "stale");

        assert!(buf.is_empty());
    }

    #[test]
    fn test_stress_throughput() {
        let buf = Arc::new(OrderedResultBuffer::new());
        let num_items = 100_000;

        let producer_buf = Arc::clone(&buf);
        let producer = thread::spawn(move || {
            for i in 0..num_items {
                producer_buf.reserve_and_fill(i);
            }
            producer_buf.shutdown();
        });

        let start = Instant::now();
        let mut count = 0;
        while let Some(_) = buf.take_next() {
            count += 1;
        }
        let elapsed = start.elapsed();

        producer.join().unwrap();

        assert_eq!(count, num_items);
        let ops_per_sec = num_items as f64 / elapsed.as_secs_f64();
        println!(
            "OrderedResultBuffer stress: {} ops in {:?} ({:.0} ops/sec)",
            num_items, elapsed, ops_per_sec
        );
        assert!(ops_per_sec > 100_000.0, "Too slow: {:.0} ops/sec", ops_per_sec);
    }

    #[test]
    fn test_parallel_stress() {
        let buf = Arc::new(OrderedResultBuffer::new());
        let num_items = 10_000;
        let num_producers = 8;

        // Reserve all first (single threaded)
        let seqs: Vec<_> = (0..num_items).map(|_| buf.reserve()).collect();

        // Distribute to producers
        let chunk_size = num_items / num_producers;
        let producers: Vec<_> = (0..num_producers)
            .map(|p| {
                let buf = Arc::clone(&buf);
                let chunk: Vec<_> = seqs[p * chunk_size..(p + 1) * chunk_size].to_vec();
                thread::spawn(move || {
                    for seq in chunk {
                        buf.fill(seq, seq * 2);
                    }
                })
            })
            .collect();

        // Consume
        let buf_consumer = Arc::clone(&buf);
        let consumer = thread::spawn(move || {
            let mut results = Vec::with_capacity(num_items);
            for _ in 0..num_items {
                if let Some(v) = buf_consumer.take_next() {
                    results.push(v);
                }
            }
            results
        });

        for p in producers {
            p.join().unwrap();
        }

        let results = consumer.join().unwrap();

        // Verify in-order
        for (i, &v) in results.iter().enumerate() {
            assert_eq!(v, i * 2, "Out of order at index {}", i);
        }
    }
}
