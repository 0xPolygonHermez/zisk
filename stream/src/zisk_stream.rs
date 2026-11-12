//! ZiskStream is responsible for reading precompile hints from a stream source and sent to a hints processor.

use crate::error::{Result, StreamError};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::{StreamRead, StreamSource};

/// A trait for processing precompile hints from a stream source.
pub trait StreamProcessor: Send + Sync + 'static {
    /// Process a batch of hint data.
    ///
    /// # Returns
    /// `true` if CTRL_END was encountered (signals end of stream), `false` otherwise.
    fn process_hints(&self, data: &[u64], first_batch: bool) -> Result<bool>;

    /// Reset the processor state, if applicable.
    fn reset(&self) {}
}

/// Trait for submitting processed hints to a sink.
///
/// # Arguments
/// * `processed` - A slice of processed hints as u64 values.
///
/// # Returns
/// * `Ok(())` - If hints were successfully submitted
/// * `Err` - If submission fails
pub trait StreamSink: Send + Sync + 'static {
    /// Submit processed hints to the sink.
    fn submit(&self, processed: &[u64]) -> Result<()>;

    /// Reset the sink state, if applicable.
    fn reset(&self) {}
}

enum ThreadCommand {
    Process,
    Shutdown,
}

/// How long retiring a source thread waits for a batch it already passed to the processor.
const RETIRE_TIMEOUT: Duration = Duration::from_secs(5);

/// Admits a source thread's batches to the processor while the thread is the current one.
///
/// Every [`ZiskStream::set_stream_src`] and [`ZiskStream::reset`] starts a new epoch, and a
/// thread spawned under an older one never feeds the processor again. Such a thread is not
/// joined: it may be blocked reading a source that only its producer can end. It is only
/// waited for while a batch it already passed to the processor is still there.
#[derive(Default)]
struct Gate {
    state: Mutex<GateState>,
    idle: Condvar,
}

#[derive(Default)]
struct GateState {
    epoch: u64,
    /// Batches inside `process_hints`.
    in_flight: usize,
}

impl Gate {
    fn lock(&self) -> MutexGuard<'_, GateState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn epoch(&self) -> u64 {
        self.lock().epoch
    }

    /// Starts a new epoch, retiring the current thread. Returns whether it has a batch in the
    /// processor.
    fn advance(&self) -> bool {
        let mut state = self.lock();
        state.epoch += 1;
        state.in_flight > 0
    }

    /// Admits a batch from a thread of `epoch` until the returned guard drops; `None` once
    /// that epoch is retired.
    fn enter(&self, epoch: u64) -> Option<Admitted<'_>> {
        let mut state = self.lock();
        if state.epoch != epoch {
            return None;
        }
        state.in_flight += 1;
        Some(Admitted { gate: self, epoch })
    }

    /// Waits until no batch is in the processor; `false` if one still is after `timeout`.
    fn wait_idle(&self, timeout: Duration) -> bool {
        let state = self.lock();
        let (state, _) = self
            .idle
            .wait_timeout_while(state, timeout, |state| state.in_flight > 0)
            .unwrap_or_else(PoisonError::into_inner);
        state.in_flight == 0
    }
}

/// A batch in the processor. Dropping it, panic unwinding included, lets a retirer go on.
struct Admitted<'a> {
    gate: &'a Gate,
    epoch: u64,
}

impl Drop for Admitted<'_> {
    fn drop(&mut self) {
        let mut state = self.gate.lock();
        state.in_flight -= 1;
        // Only a retirer waits, and it starts a new epoch first.
        if state.epoch != self.epoch {
            self.gate.idle.notify_all();
        }
    }
}

/// ZiskStream struct manages the processing of precompile hints and writing them to shared memory.
pub struct ZiskStream<P: StreamProcessor> {
    /// The hints processor used to process hints before writing.
    hints_processor: Arc<P>,

    /// Channel sender to communicate with the background thread.
    tx: Option<Sender<ThreadCommand>>,

    /// Join handle for the background thread.
    thread_handle: Option<JoinHandle<()>>,

    /// Keeps a retired background thread away from the processor.
    gate: Arc<Gate>,

    /// How long retiring a source thread waits for a batch it already passed to the
    /// processor: [`RETIRE_TIMEOUT`].
    retire_timeout: Duration,

    /// A reset gave up waiting for the old source's last batch: the processor is reset again
    /// once that batch has left, before a new source feeds it.
    reset_pending: bool,

    initialized: AtomicBool,
}

impl<P: StreamProcessor> ZiskStream<P> {
    /// Create a new ZiskStream with the given processor.
    ///
    /// # Arguments
    /// * `hints_processor` - The processor used to process hints.
    ///
    /// # Returns
    /// A new `ZiskStream` instance without a running thread.
    pub fn new(hints_processor: P) -> Self {
        Self::from_arc(Arc::new(hints_processor))
    }

    /// Create a new ZiskStream from an already-Arc-wrapped processor.
    ///
    /// Useful when the processor is shared with other owners (e.g. `InputsShmemWriter`).
    pub fn from_arc(hints_processor: Arc<P>) -> Self {
        Self {
            hints_processor,
            tx: None,
            thread_handle: None,
            gate: Arc::default(),
            retire_timeout: RETIRE_TIMEOUT,
            reset_pending: false,
            initialized: AtomicBool::new(false),
        }
    }

    /// Retire the current background thread, if any, so it never feeds the processor again.
    ///
    /// The thread is detached, not joined: it may be blocked reading a source that stays
    /// open, and joining it would wait for that source's producer. The stream has no source
    /// until [`Self::set_stream_src`] gives it one. Returns whether the thread still has a
    /// batch in the processor.
    fn retire_thread(&mut self) -> bool {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(ThreadCommand::Shutdown);
        }
        self.thread_handle.take();
        self.initialized.store(false, Ordering::SeqCst);
        self.gate.advance()
    }

    /// Set a new StreamSource for the pipeline and spawn a background thread to process data.
    ///
    /// This retires any existing background thread and starts a new one with the new stream.
    /// The old thread is not waited for, unless it still has a batch in the processor. If an
    /// earlier [`Self::reset`] gave up waiting for such a batch, the processor is reset again
    /// first.
    ///
    /// # Arguments
    /// * `stream` - The new StreamSource for reading data.
    ///
    /// # Errors
    ///
    /// Returns [`StreamError::Transport`] if the old thread's batch is still in the processor
    /// after a few seconds: two sources would feed it at once. The stream is then left
    /// without a source.
    pub fn set_stream_src(&mut self, mut stream: StreamSource) -> Result<()> {
        if self.retire_thread() && !self.gate.wait_idle(self.retire_timeout) {
            return Err(StreamError::Transport(format!(
                "the previous stream source still feeds the processor after {:?}",
                self.retire_timeout
            )));
        }
        if std::mem::take(&mut self.reset_pending) {
            self.hints_processor.reset();
        }

        // Create a new channel for communication with the thread
        let (tx, rx) = std::sync::mpsc::channel();
        self.tx = Some(tx);

        // Clone Arc references for the thread
        let hints_processor = Arc::clone(&self.hints_processor);
        let gate = Arc::clone(&self.gate);
        let epoch = gate.epoch();

        // Spawn the background thread — open() is deferred here so that
        // QUIC readers can create their dedicated runtime on a plain thread
        // (not inside spawn_blocking, which interferes with block_on).
        let thread_handle = thread::spawn(move || {
            if !stream.is_active() {
                if let Err(e) = stream.open() {
                    tracing::error!("Failed to open stream in background thread: {e}");
                    return;
                }
            }
            Self::background_thread(stream, hints_processor, gate, epoch, rx);
        });

        self.thread_handle = Some(thread_handle);

        self.initialized.store(true, Ordering::SeqCst);

        Ok(())
    }

    /// Background thread function that processes hints when requested.
    fn background_thread(
        mut stream: StreamSource,
        hints_processor: Arc<P>,
        gate: Arc<Gate>,
        epoch: u64,
        rx: Receiver<ThreadCommand>,
    ) {
        while let Ok(ThreadCommand::Process) = rx.recv() {
            match Self::process_stream(&mut stream, &*hints_processor, &gate, epoch) {
                Ok(true) => {}
                Ok(false) => break,
                // What a retired thread's source or batch does no longer matters: the reset
                // that retired it may have torn down the state the batch was writing to.
                Err(_) if gate.epoch() != epoch => break,
                Err(e) => panic!("Error processing hints in background thread: {:?}", e),
            }
        }
        // Loop exits when retired, when Shutdown is received or when the channel is closed
    }

    /// Process all hints from the stream.
    ///
    /// Processes hints in batches until CTRL_END is encountered or the stream ends. Returns
    /// `false` if the thread was retired (its `epoch` is over): it must stop.
    fn process_stream(
        stream: &mut StreamSource,
        hints_processor: &P,
        gate: &Gate,
        epoch: u64,
    ) -> Result<bool> {
        let mut first_batch = true;

        while let Some(hints) = stream.next()? {
            let hints = reinterpret_vec(hints)?;
            let Some(admitted) = gate.enter(epoch) else {
                return Ok(false);
            };
            let has_ctrl_end = hints_processor.process_hints(&hints, first_batch);
            drop(admitted);
            let has_ctrl_end = has_ctrl_end?;

            first_batch = false;

            // Break if CTRL_END was encountered
            if has_ctrl_end {
                break;
            }
        }

        Ok(true)
    }

    /// Reset the ZiskStream state, retiring the background thread and resetting the processor.
    ///
    /// Data the old source delivers afterwards is dropped. A batch the old thread already
    /// passed to the processor is waited for (a few seconds at most) before the processor is
    /// reset, so the reset never runs alongside it. If the batch is still inside after that,
    /// the processor is reset anyway, and again before the next source starts.
    pub fn reset(&mut self) {
        if self.retire_thread() && !self.gate.wait_idle(self.retire_timeout) {
            tracing::warn!(
                "ZiskStream::reset: a batch of the old stream source is still in the processor \
                 after {:?}; it is reset again before the next source starts",
                self.retire_timeout
            );
            self.reset_pending = true;
        }
        self.hints_processor.reset();
    }

    /// Trigger the background thread to process hints asynchronously.
    ///
    /// This method:
    /// 1. Sends a command to the background thread to process hints
    /// 2. Returns immediately without waiting for processing to complete
    ///
    /// # Returns
    /// * `Ok(())` - If the command was successfully sent
    /// * `Err` - If there's no active thread or the channel is closed
    pub fn start_stream(&mut self) -> Result<()> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(StreamError::Transport(
                "Stream is not initialized. Call set_stream_src first.".to_string(),
            ));
        }

        if let Some(tx) = &self.tx {
            tx.send(ThreadCommand::Process).map_err(|e| {
                StreamError::Transport(format!(
                    "Failed to send process command to background thread: {}",
                    e
                ))
            })?;
            Ok(())
        } else {
            Err(StreamError::Transport(
                "No background thread running. Call set_stream_src first.".to_string(),
            ))
        }
    }

    /// Check if the ZiskStream has been initialized with a stream source.
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    /// Get a reference to the hints processor.
    pub fn get_processor(&self) -> Arc<P> {
        Arc::clone(&self.hints_processor)
    }
}

impl<P: StreamProcessor> Drop for ZiskStream<P> {
    fn drop(&mut self) {
        // Drop tx — the background thread will see the channel closed and
        // exit after its current process_stream() call returns naturally.
        // We must NOT join here: for gRPC hints the thread blocks on
        // ChannelStreamReader::recv(), which only unblocks when the client
        // calls PushJobHintsInput.  That RPC can't start until submit_job
        // returns, but submit_job can't return if we're blocking in join().
        // Nor is it retired: the coordinator's hints relay drops its stream as soon as it
        // starts it, and relies on the thread to run the source to its end.
        self.tx.take();
        self.thread_handle.take(); // detach
    }
}

/// Marker for types that are sound to reinterpret raw bytes as, in either direction: an
/// arbitrary byte buffer can be read as them, and their own bytes can be read raw. Mirror
/// of `zisk_common::AnyBitPattern` (this crate cannot depend on `zisk-common`).
///
/// # Safety
///
/// Implementors must both accept every bit pattern as a valid value (the destination
/// requirement) and contain no padding or otherwise uninitialized bytes (the source
/// requirement). Integer types qualify; `bool`, `char`, `NonZero*`, niche enums,
/// references, and structs with padding do not.
unsafe trait AnyBitPattern {}

// SAFETY: `u8` has no invalid bit patterns.
unsafe impl AnyBitPattern for u8 {}
// SAFETY: `u64` has no invalid bit patterns.
unsafe impl AnyBitPattern for u64 {}

/// Reinterprets a `Vec<T>` as a `Vec<U>` over the same bytes.
///
/// Mirror of `zisk_common::reinterpret_vec`, duplicated here because `zisk-common`
/// depends on `zisk-stream` and so this crate cannot import it back — keep the two in
/// sync.
///
/// When the source allocation matches `Vec<U>`'s layout (identical alignment and a byte
/// length/capacity that are whole numbers of `U`) the buffer is reused in place;
/// otherwise (e.g. `u8` → `u64`, where the alignment differs) the bytes are copied into
/// a fresh `Vec<U>` allocated — and therefore freed — under `U`'s own layout, so the
/// result is sound to drop on any global allocator. A trailing partial `U` is
/// zero-padded.
///
/// The `T: AnyBitPattern` bound guarantees the source bytes are fully initialized (no
/// padding to read as uninitialized memory) and the `U: AnyBitPattern` bound guarantees
/// the reinterpreted bytes form valid `U` values, so this function is safe.
///
/// # Errors
///
/// Returns [`StreamError::Invalid`] if `U` is a zero-sized type.
fn reinterpret_vec<T: Copy + AnyBitPattern, U: AnyBitPattern>(v: Vec<T>) -> Result<Vec<U>> {
    let size_t = std::mem::size_of::<T>();
    let size_u = std::mem::size_of::<U>();

    // A zero-sized `U` would make the `% size_u` / `/ size_u` arithmetic below divide by
    // zero; reject it explicitly rather than panic.
    if size_u == 0 {
        return Err(StreamError::Invalid(format!(
            "cannot reinterpret Vec<{}> as Vec<{}>: destination type is zero-sized",
            std::any::type_name::<T>(),
            std::any::type_name::<U>()
        )));
    }

    let byte_len = v.len() * size_t;
    let byte_cap = v.capacity() * size_t;
    let len = byte_len.div_ceil(size_u); // whole `U` count, rounding a partial tail up

    // Zero-copy is sound only when the source allocation matches `Vec<U>`'s layout
    // exactly: identical alignment (so `ptr` is aligned for `U` and the free-time
    // `Layout` matches the allocation), a byte capacity that is a whole number of `U` (so
    // `cap` is not truncated), and a byte length already a whole number of `U` (a reused
    // buffer cannot grow to pad a partial tail).
    if std::mem::align_of::<T>() == std::mem::align_of::<U>()
        && byte_cap % size_u == 0
        && byte_len % size_u == 0
    {
        let cap = byte_cap / size_u;
        let ptr = v.as_ptr() as *mut U;
        std::mem::forget(v);
        // SAFETY: `ptr` comes from `v`'s allocation, forgotten just above so it is freed
        // exactly once — by the returned `Vec`. Equal alignment makes `ptr` aligned for
        // `U` and the layout identical under either type; `byte_cap % size_u == 0` makes
        // `cap * size_u == byte_cap`, so the returned `Vec` frees precisely the original
        // allocation, and `byte_len % size_u == 0` makes `len` cover it with no partial
        // element. `U: AnyBitPattern` makes every byte a valid `U`.
        return Ok(unsafe { Vec::from_raw_parts(ptr, len, cap) });
    }

    // Layouts differ (e.g. `u8` → `u64`): reusing the buffer would free it under a
    // different `Layout` than it was allocated with — undefined behaviour, and not merely
    // theoretical under jemalloc/mimalloc/Miri or a non-glibc libc. Copy into a fresh
    // `Vec<U>` owned end-to-end under `U`'s own layout, zero-filling any partial trailing
    // `U`. The source `v` is left untouched and dropped normally at end of scope.
    let mut out: Vec<U> = Vec::with_capacity(len);
    let out_bytes = len * size_u;
    // SAFETY: `out` reserves `out_bytes >= byte_len` bytes in a distinct allocation, so
    // copying the `byte_len` source bytes is in-bounds and non-overlapping; the remaining
    // `[byte_len, out_bytes)` tail is then zeroed, so all `out_bytes` bytes are
    // initialized and `set_len(len)` is sound. `U: AnyBitPattern` makes every byte
    // pattern (source data and zero padding alike) a valid `U`.
    unsafe {
        let dst = out.as_mut_ptr() as *mut u8;
        std::ptr::copy_nonoverlapping(v.as_ptr() as *const u8, dst, byte_len);
        std::ptr::write_bytes(dst.add(byte_len), 0, out_bytes - byte_len);
        out.set_len(len);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{mpsc, Mutex};
    use std::time::{Duration, Instant};

    /// Records what reaches it, in order.
    #[derive(Default)]
    struct Recorder {
        events: Mutex<Vec<&'static str>>,
        /// When set, the first batch reports it entered and blocks until released.
        hold: Mutex<Option<(mpsc::Sender<()>, mpsc::Receiver<()>)>>,
    }

    impl Recorder {
        fn events(&self) -> Vec<&'static str> {
            self.events.lock().unwrap().clone()
        }
    }

    impl StreamProcessor for Recorder {
        fn process_hints(&self, _data: &[u64], _first_batch: bool) -> Result<bool> {
            self.events.lock().unwrap().push("batch");
            if let Some((entered, release)) = self.hold.lock().unwrap().take() {
                entered.send(()).unwrap();
                release.recv().unwrap();
                self.events.lock().unwrap().push("batch-done");
            }
            Ok(false)
        }

        fn reset(&self) {
            self.events.lock().unwrap().push("reset");
        }
    }

    fn one_word() -> Option<Vec<u8>> {
        Some(vec![0u8; 8])
    }

    /// Late data from a source the job is done with must not reach the processor the next
    /// job uses.
    #[test]
    fn reset_stops_the_old_source_feeding_the_processor() {
        let recorder = Arc::new(Recorder::default());
        let mut stream = ZiskStream::from_arc(Arc::clone(&recorder));
        let (source, producer) = StreamSource::channel();
        stream.set_stream_src(source).unwrap();
        stream.start_stream().unwrap();

        stream.reset();
        producer.send(one_word()).unwrap();

        thread::sleep(Duration::from_millis(300));
        assert_eq!(recorder.events(), ["reset"], "the old source fed the processor after reset");
        assert!(!stream.is_initialized());
    }

    /// A source whose producer keeps it open must not hold up the next one.
    #[test]
    fn set_stream_src_does_not_wait_for_a_blocked_old_source() {
        let recorder = Arc::new(Recorder::default());
        let mut stream = ZiskStream::from_arc(Arc::clone(&recorder));
        let (source, producer) = StreamSource::channel();
        stream.set_stream_src(source).unwrap();
        stream.start_stream().unwrap();

        let (done_tx, done_rx) = mpsc::channel();
        let replacer = thread::spawn(move || {
            stream.set_stream_src(StreamSource::from_vec(vec![0u8; 8])).unwrap();
            stream.start_stream().unwrap();
            done_tx.send(()).unwrap();
            stream
        });
        let blocked = done_rx.recv_timeout(Duration::from_secs(1)).is_err();
        // Close the old source either way, so a blocked replacer can finish.
        drop(producer);
        let _stream = replacer.join().unwrap();
        assert!(!blocked, "replacing the source waited for the old producer to close it");

        // Only the new source's batch reached the processor.
        thread::sleep(Duration::from_millis(100));
        assert_eq!(recorder.events(), ["batch"]);
    }

    /// A batch the old source already passed to the processor is waited for, and only then is
    /// the processor reset: never alongside the batch, and after what it wrote.
    #[test]
    fn reset_waits_for_a_batch_in_the_processor_and_resets_after_it() {
        let recorder = Arc::new(Recorder::default());
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        *recorder.hold.lock().unwrap() = Some((entered_tx, release_rx));

        let mut stream = ZiskStream::from_arc(Arc::clone(&recorder));
        let (source, producer) = StreamSource::channel();
        stream.set_stream_src(source).unwrap();
        stream.start_stream().unwrap();
        producer.send(one_word()).unwrap();
        entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();

        let releaser = thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));
            release_tx.send(()).unwrap();
        });
        let start = Instant::now();
        stream.reset();
        assert!(start.elapsed() >= Duration::from_millis(150), "reset did not wait for the batch");
        releaser.join().unwrap();

        // More data from the retired source is dropped.
        producer.send(one_word()).unwrap();
        thread::sleep(Duration::from_millis(100));
        assert_eq!(recorder.events(), ["batch", "batch-done", "reset"]);
    }

    /// Holds the recorder's first batch in the processor; returns what lets it go.
    fn hold_first_batch(recorder: &Recorder) -> (mpsc::Receiver<()>, mpsc::Sender<()>) {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        *recorder.hold.lock().unwrap() = Some((entered_tx, release_rx));
        (entered_rx, release_tx)
    }

    /// A reset that gives up waiting for the old source's batch resets the processor again
    /// once the batch has left, before the next source feeds it.
    #[test]
    fn a_reset_that_gives_up_on_a_batch_resets_again_before_the_next_source() {
        let recorder = Arc::new(Recorder::default());
        let (entered, release) = hold_first_batch(&recorder);
        let mut stream = ZiskStream::from_arc(Arc::clone(&recorder));
        stream.retire_timeout = Duration::from_millis(100);
        let (source, producer) = StreamSource::channel();
        stream.set_stream_src(source).unwrap();
        stream.start_stream().unwrap();
        producer.send(one_word()).unwrap();
        entered.recv_timeout(Duration::from_secs(5)).unwrap();

        stream.reset();
        release.send(()).unwrap();
        thread::sleep(Duration::from_millis(100));

        stream.set_stream_src(StreamSource::from_vec(vec![0u8; 8])).unwrap();
        stream.start_stream().unwrap();
        thread::sleep(Duration::from_millis(100));
        assert_eq!(recorder.events(), ["batch", "reset", "batch-done", "reset", "batch"]);
    }

    /// A new source refused because the old one's batch is still in the processor leaves the
    /// stream without a source, and says so.
    #[test]
    fn a_refused_source_leaves_the_stream_uninitialized() {
        let recorder = Arc::new(Recorder::default());
        let (entered, release) = hold_first_batch(&recorder);
        let mut stream = ZiskStream::from_arc(Arc::clone(&recorder));
        stream.retire_timeout = Duration::from_millis(100);
        let (source, producer) = StreamSource::channel();
        stream.set_stream_src(source).unwrap();
        stream.start_stream().unwrap();
        producer.send(one_word()).unwrap();
        entered.recv_timeout(Duration::from_secs(5)).unwrap();

        assert!(stream.set_stream_src(StreamSource::from_vec(vec![0u8; 8])).is_err());
        assert!(!stream.is_initialized());
        assert!(stream.start_stream().is_err());
        release.send(()).unwrap();
    }

    /// A batch that panics kills its thread, as before, and must not leave the next source
    /// waiting for a batch that never ends.
    #[test]
    fn a_panicking_batch_does_not_hold_up_the_next_source() {
        struct Panicker;
        impl StreamProcessor for Panicker {
            fn process_hints(&self, _data: &[u64], _first_batch: bool) -> Result<bool> {
                panic!("processor panicked on purpose");
            }
        }

        let mut stream = ZiskStream::new(Panicker);
        stream.set_stream_src(StreamSource::from_vec(vec![0u8; 8])).unwrap();
        stream.start_stream().unwrap();
        thread::sleep(Duration::from_millis(100));

        let start = Instant::now();
        stream.set_stream_src(StreamSource::from_vec(vec![])).unwrap();
        assert!(start.elapsed() < Duration::from_secs(1), "waited for the panicked batch");
    }
}
