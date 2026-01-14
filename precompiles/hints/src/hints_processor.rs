//! Precompile Hints Processor
//!
//! This module provides functionality for processing precompile hints
//! that are received as a stream of `u64` values. Hints are used to provide preprocessed
//! data to precompile operations in the ZisK zkVM.

use anyhow::Result;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::collections::{HashMap, VecDeque};
use std::mem::ManuallyDrop;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use tracing::debug;
use zisk_common::io::{StreamProcessor, StreamSink};
use zisk_common::{BuiltInHint, CtrlHint, HintCode, PrecompileHint};

/// Ordered result buffer with drain state.
///
/// This structure maintains a VecDeque that holds processed results in order,
/// allowing out-of-order completion while ensuring in-order output.
struct ResultQueue {
    /// The result buffer: None = pending, Some(Ok(...)) = ready, Some(Err(...)) = error
    buffer: VecDeque<Option<Result<Vec<u64>>>>,
    /// Sequence ID of the next result to drain from buffer[0]
    next_drain_seq: usize,
}

/// Thread-safe shared state for parallel hint processing.
struct HintProcessorState {
    /// Ordered results ready for draining
    queue: Mutex<ResultQueue>,
    /// Notifies drainer thread when a hint completes
    drain_signal: Condvar,
    /// Next sequence ID to assign to incoming hints
    next_seq: AtomicUsize,
    /// Signals processing should stop
    error_flag: AtomicBool,
    /// Signals drainer thread to shut down
    shutdown: AtomicBool,
    /// Invalidates stale workers after reset
    generation: AtomicUsize,
}

impl HintProcessorState {
    fn new() -> Self {
        Self {
            queue: Mutex::new(ResultQueue { buffer: VecDeque::new(), next_drain_seq: 0 }),
            drain_signal: Condvar::new(),
            next_seq: AtomicUsize::new(0),
            error_flag: AtomicBool::new(false),
            shutdown: AtomicBool::new(false),
            generation: AtomicUsize::new(0),
        }
    }
}

/// Type alias for custom hint handler functions.
pub type CustomHintHandler = Arc<dyn Fn(&[u64]) -> Result<Vec<u64>> + Send + Sync>;

/// Builder for configuring and constructing a [`HintsProcessor`].
pub struct HintsProcessorBuilder<HS: StreamSink + Send + Sync + 'static> {
    hints_sink: HS,
    num_threads: usize,
    enable_stats: bool,
    custom_handlers: HashMap<u32, CustomHintHandler>,
}

impl<HS: StreamSink + Send + Sync + 'static> HintsProcessorBuilder<HS> {
    /// Sets the number of worker threads in the thread pool.
    pub fn num_threads(mut self, num_threads: usize) -> Self {
        self.num_threads = num_threads;
        self
    }

    /// Enables or disables statistics collection.
    pub fn enable_stats(mut self, enable: bool) -> Self {
        self.enable_stats = enable;
        self
    }

    /// Registers a custom hint handler for a specific hint code.
    ///
    /// # Arguments
    ///
    /// * `hint_code` - The u32 hint code identifier (should not conflict with built-in codes)
    /// * `handler` - Function that processes the hint data and returns the result
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let processor = HintsProcessor::builder(my_sink)
    ///     .custom_hint(0x10, |data| {
    ///         // Custom processing logic
    ///         Ok(vec![data[0] * 2])
    ///     })
    ///     .build()?;
    /// ```
    pub fn custom_hint<F>(mut self, hint_code: u32, handler: F) -> Self
    where
        F: Fn(&[u64]) -> Result<Vec<u64>> + Send + Sync + 'static,
    {
        self.custom_handlers.insert(hint_code, Arc::new(handler));
        self
    }

    /// Builds the [`HintsProcessor`] with the configured settings.
    ///
    /// # Returns
    ///
    /// * `Ok(HintsProcessor)` - Successfully constructed processor
    /// * `Err` - If the thread pool fails to initialize
    pub fn build(self) -> Result<HintsProcessor<HS>> {
        let pool = ThreadPoolBuilder::new()
            .num_threads(self.num_threads)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create thread pool: {}", e))?;

        let state = Arc::new(HintProcessorState::new());
        let hints_sink = Arc::new(self.hints_sink);

        // Spawn drainer thread
        let drainer_state = Arc::clone(&state);
        let drainer_sink = Arc::clone(&hints_sink);
        let drainer_thread = std::thread::spawn(move || {
            HintsProcessor::drainer_thread(drainer_state, drainer_sink);
        });

        Ok(HintsProcessor {
            pool,
            num_hint: AtomicUsize::new(0),
            state,
            stats: if self.enable_stats { Some(Mutex::new(HashMap::new())) } else { None },
            hints_sink,
            drainer_thread: ManuallyDrop::new(drainer_thread),
            custom_handlers: Arc::new(self.custom_handlers),
        })
    }
}

/// Processor for precompile hints that supports parallel execution.
///
/// This struct provides methods to parse and process a stream of concatenated
/// hints, using a dedicated Rayon thread pool for parallel processing while
/// preserving the original order of results.
pub struct HintsProcessor<HS: StreamSink + Send + Sync + 'static> {
    /// The thread pool used for parallel hint processing.
    pool: ThreadPool,

    num_hint: AtomicUsize,

    /// Shared state for parallel hint processing
    state: Arc<HintProcessorState>,

    /// Optional statistics collected during hint processing (for debugging).
    stats: Option<Mutex<HashMap<HintCode, usize>>>,

    /// The hints sink used to submit processed hints (kept for ownership).
    #[allow(dead_code)]
    hints_sink: Arc<HS>,

    /// Handle to the drainer thread (wrapped in ManuallyDrop to join in Drop)
    drainer_thread: ManuallyDrop<std::thread::JoinHandle<()>>,

    /// Custom hint handlers registered by the user
    custom_handlers: Arc<HashMap<u32, CustomHintHandler>>,
}

impl<HS: StreamSink + Send + Sync + 'static> HintsProcessor<HS> {
    const DEFAULT_NUM_THREADS: usize = 1;

    /// Creates a builder for configuring a [`HintsProcessor`].
    ///
    /// # Arguments
    ///
    /// * `hints_sink` - The sink used to submit processed hints
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let processor = HintsProcessor::builder(my_sink)
    ///     .num_threads(16)
    ///     .enable_stats(false)
    ///     .build()?;
    /// ```
    pub fn builder(hints_sink: HS) -> HintsProcessorBuilder<HS> {
        HintsProcessorBuilder {
            hints_sink,
            num_threads: Self::DEFAULT_NUM_THREADS,
            enable_stats: false,
            custom_handlers: HashMap::new(),
        }
    }

    /// Processes hints in parallel with non-blocking, ordered output.
    ///
    /// This method dispatches each hint to the thread pool for parallel processing.
    /// Results are collected in a reorder buffer and submitted to the sink in the original
    /// order as soon as consecutive results become available.
    ///
    /// # Key characteristics:
    /// - **Non-blocking**: Returns immediately after enqueuing hints
    /// - **Global sequence**: Sequence IDs maintained across multiple batch calls
    /// - **Ordered submission**: Results submitted to sink in order hints were received
    /// - **Error handling**: Stops processing on first error
    ///
    /// # Concurrency Warning
    ///
    /// This method takes is designed for **sequential usage only**.
    /// Concurrent calls may cause incorrect processing.
    ///
    /// # Arguments
    ///
    /// * `hints` - A slice of `u64` values containing concatenated hints
    /// * `first_batch` - Whether this is the first batch (for CTRL_START validation)
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - CTRL_END was encountered
    /// * `Ok(false)` - Batch processed successfully, no CTRL_END
    /// * `Err` - If a previous error occurred or hints are malformed
    pub fn process_hints(&self, hints: &[u64], first_batch: bool) -> Result<bool> {
        let mut has_ctrl_end = false;

        // Parse hints and dispatch to pool
        let mut idx = 0;
        while idx < hints.len() {
            // Check for error before processing each hint
            if self.state.error_flag.load(Ordering::Acquire) {
                return Err(anyhow::anyhow!("Processing stopped due to previous error"));
            }

            let hint = PrecompileHint::from_u64_slice(hints, idx, true)?;
            self.num_hint.fetch_add(1, Ordering::Relaxed);
            println!("[{}] Hint processed {:?}:", self.num_hint.load(Ordering::Relaxed), hint);

            // Check if custom handler is registered for custom hints
            if let HintCode::Custom(code) = hint.hint_code {
                if !self.custom_handlers.contains_key(&code) {
                    return Err(anyhow::anyhow!(
                        "Unknown custom hint code {:#x}: no handler registered",
                        code
                    ));
                }
            }

            let length = hint.data.len();

            if let Some(stats) = &self.stats {
                stats.lock().unwrap().entry(hint.hint_code).and_modify(|c| *c += 1).or_insert(1);
            }

            // Check if this is a control code
            match hint.hint_code {
                HintCode::Ctrl(CtrlHint::Start) => {
                    // CTRL_START must be the first message of the first batch
                    if !first_batch {
                        return Err(anyhow::anyhow!(
                            "CTRL_START can only be sent as the first message in the stream"
                        ));
                    }
                    if idx != 0 {
                        return Err(anyhow::anyhow!(
                            "CTRL_START must be the first hint in the batch, but found at index {}",
                            idx
                        ));
                    }
                    // Reset global sequence and buffer at stream start
                    self.reset();
                    // Control hint only; skip processing
                    idx += length + 1;
                    continue;
                }
                HintCode::Ctrl(CtrlHint::End) => {
                    // Control hint only; wait for completion then set flag
                    self.wait_for_completion()?;
                    has_ctrl_end = true;
                    idx += length + 1;

                    debug!("CTRL_END received, all hints processed");

                    // CTRL_END should be the last message - verify and break
                    if idx < hints.len() {
                        return Err(anyhow::anyhow!(
                            "CTRL_END must be the last hint, but {} bytes remain",
                            hints.len() - idx
                        ));
                    }
                    break;
                }
                HintCode::Ctrl(CtrlHint::Cancel) => {
                    // Cancel current stream: set error and notify
                    self.state.error_flag.store(true, Ordering::Release);
                    self.state.drain_signal.notify_all();
                    return Err(anyhow::anyhow!("Stream cancelled"));
                }
                HintCode::Ctrl(CtrlHint::Error) => {
                    // External error signal
                    self.state.error_flag.store(true, Ordering::Release);
                    self.state.drain_signal.notify_all();
                    return Err(anyhow::anyhow!("Stream error signalled"));
                }
                _ => {} // Built-in data hint or custom hint; continue processing
            }

            // Capture generation outside mutex - SeqCst provides sufficient ordering
            let generation = self.state.generation.load(Ordering::SeqCst);

            // Atomically reserve slot - use Relaxed for seq since mutex provides ordering
            let seq_id = {
                let mut queue = self.state.queue.lock().unwrap();
                let seq = self.state.next_seq.fetch_add(1, Ordering::Relaxed);

                // Handle HintCode::Noop synchronously - reserve and fill slot in one step
                if hint.hint_code == HintCode::BuiltIn(BuiltInHint::Noop) {
                    queue.buffer.push_back(Some(Ok(hint.data.clone())));
                    // Notify immediately while holding the lock to ensure drainer sees the result
                    // Release lock after this block, avoiding duplicate notification
                    drop(queue);
                    // Use notify_all since wait_for_completion also waits on this condvar
                    self.state.drain_signal.notify_all();
                    // Continue to next hint without spawning worker
                    idx += length + 1;
                    continue;
                } else {
                    queue.buffer.push_back(None);
                }

                seq
            };

            // Spawn processing task for async hints (Noop already handled above)
            let state = Arc::clone(&self.state);
            let custom_handlers = Arc::clone(&self.custom_handlers);
            self.pool.spawn(move || {
                Self::worker_thread(state, hint, generation, seq_id, custom_handlers);
            });

            idx += length + 1;
        }

        if has_ctrl_end {
            if let Some(stats) = &self.stats {
                debug!("Processed hints stats:");
                let stats = stats.lock().unwrap();
                let mut sorted_stats: Vec<_> = stats.iter().collect();
                sorted_stats.sort_by_key(|(&hint_code, _)| hint_code.to_u32());
                for (hint_code, count) in sorted_stats {
                    debug!("Hint type {}: {}", hint_code, count);
                }
            }
        }

        Ok(has_ctrl_end)
    }

    /// Worker thread that processes a single hint and stores the result.
    ///
    /// # Arguments
    ///
    /// * `state` - Shared processor state
    /// * `hint` - The hint to process
    /// * `generation` - Generation number for detecting stale workers
    /// * `seq_id` - Sequence ID for ordering results
    /// * `custom_handlers` - Custom hint handlers
    fn worker_thread(
        state: Arc<HintProcessorState>,
        hint: PrecompileHint,
        generation: usize,
        seq_id: usize,
        custom_handlers: Arc<HashMap<u32, CustomHintHandler>>,
    ) {
        // Check generation first to detect stale workers (before processing)
        let current_gen = state.generation.load(Ordering::SeqCst);
        if generation != current_gen {
            // Worker belongs to old generation; ignore
            return;
        }

        println!("Hint processed {:?}:", hint);

        // Check if we should stop due to error - but still need to fill the slot
        let result = if state.error_flag.load(Ordering::Acquire) {
            Err(anyhow::anyhow!("Processing stopped due to error"))
        } else {
            // Process the hint
            Self::dispatch_hint(hint, custom_handlers)
        };

        // println!(
        //     "Hint result: {:x?} bytes",
        //     match &result {
        //         Ok(data) => format!("{:?}", data),
        //         Err(e) => format!("Err({})", e),
        //     }
        // );

        // Store result - MUST fill slot even if error occurred
        let mut queue = state.queue.lock().unwrap();

        // Check generation again in case reset happened during processing
        let current_gen = state.generation.load(Ordering::SeqCst);
        if generation != current_gen {
            // Worker belongs to old generation; buffer was cleared and repopulated
            // Our seq_id is from the old session and doesn't correspond to current slots
            return;
        }

        // Calculate offset in buffer; handle drained slots
        if seq_id < queue.next_drain_seq {
            // This result belongs to a previous stream/session; ignore
            return;
        }
        let offset = seq_id - queue.next_drain_seq;

        // Check if slot exists - if not, drainer already processed and removed it
        if offset >= queue.buffer.len() {
            // Slot was already drained; safe to drop this result
            return;
        }

        // Fill the slot to allow drainer to proceed (critical for ordering)
        queue.buffer[offset] = Some(result);

        // Release lock before notifying
        drop(queue);

        // Notify drainer thread (use notify_all to wake any waiting threads)
        state.drain_signal.notify_all();
    }

    /// Drainer thread that waits for hints to complete and drains ready results from queue.
    fn drainer_thread(state: Arc<HintProcessorState>, hints_sink: Arc<HS>) {
        loop {
            let mut queue = state.queue.lock().unwrap();

            // Check for shutdown
            if state.shutdown.load(Ordering::Acquire) {
                break;
            }

            // Drain all consecutive ready results from the front
            let mut drained_any = false;
            while let Some(Some(res)) = queue.buffer.front() {
                drained_any = true;
                match res {
                    Ok(data) => {
                        // Clone data before dropping lock
                        let data_to_submit = data.clone();
                        queue.buffer.pop_front();
                        queue.next_drain_seq += 1;

                        // Drop lock before submitting to avoid blocking workers
                        drop(queue);

                        // Submit to sink
                        if let Err(e) = hints_sink.submit(data_to_submit) {
                            eprintln!("Error submitting to sink: {}", e);
                            state.error_flag.store(true, Ordering::Release);
                            state.drain_signal.notify_all();
                            return;
                        }

                        // Re-acquire lock for next iteration
                        queue = state.queue.lock().unwrap();
                    }
                    Err(e) => {
                        // Error found - signal to stop
                        state.error_flag.store(true, Ordering::Release);
                        eprintln!("[seq={}] Error: {}", queue.next_drain_seq, e);
                        queue.buffer.pop_front();
                        queue.next_drain_seq += 1;
                        state.drain_signal.notify_all();
                        return;
                    }
                }
            }

            // If we drained any results, notify wait_for_completion that buffer changed
            if drained_any {
                state.drain_signal.notify_all();
            }

            // Check for shutdown again before waiting
            if state.shutdown.load(Ordering::Acquire) {
                break;
            }

            // Wait for notification that a hint completed
            #[allow(unused_assignments)]
            {
                queue = state.drain_signal.wait(queue).unwrap();
            }
        }
    }

    /// Waits for all pending hints to be processed and drained.
    ///
    /// This method blocks until the reorder buffer is empty, meaning all
    /// dispatched hints have been processed and their results printed.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - All hints processed successfully
    /// * `Err` - If an error occurred during processing
    pub fn wait_for_completion(&self) -> Result<()> {
        let mut queue = self.state.queue.lock().unwrap();

        while !queue.buffer.is_empty() {
            if self.state.error_flag.load(Ordering::Acquire) {
                return Err(anyhow::anyhow!("Processing stopped due to error"));
            }
            // Wait for notification that buffer state changed
            queue = self.state.drain_signal.wait(queue).unwrap();
        }

        if self.state.error_flag.load(Ordering::Acquire) {
            return Err(anyhow::anyhow!("Processing stopped due to error"));
        }

        Ok(())
    }

    /// Resets the processor state, clearing any errors and the reorder buffer.
    ///
    /// This should be called to start a fresh processing session after an error
    /// or when you want to reset the global sequence counter.
    ///
    /// Increments the generation counter to invalidate any in-flight workers
    /// from the previous session, preventing them from corrupting the new state.
    fn reset(&self) {
        // Clear error flag - use Release to synchronize with Acquire loads in workers
        self.state.error_flag.store(false, Ordering::Release);
        // Reset sequence counter - Relaxed is sufficient as it's only used within mutex
        self.state.next_seq.store(0, Ordering::Relaxed);
        // Increment generation with SeqCst to invalidate stale workers
        // This provides a total ordering fence that synchronizes with worker generation checks
        self.state.generation.fetch_add(1, Ordering::SeqCst);
        let mut queue = self.state.queue.lock().unwrap();
        queue.buffer.clear();
        queue.next_drain_seq = 0;
    }

    /// Dispatches a single hint to its appropriate handler based on hint type.
    ///
    /// # Arguments
    ///
    /// * `hint` - The parsed hint to dispatch
    /// * `custom_handlers` - Custom hint handlers
    ///
    /// # Returns
    ///
    /// The result produced by the selected hint handler.
    ///
    /// # Note
    ///
    /// Control codes and Noop hints are handled before this function is called.
    #[inline]
    fn dispatch_hint(
        hint: PrecompileHint,
        custom_handlers: Arc<HashMap<u32, CustomHintHandler>>,
    ) -> Result<Vec<u64>> {
        match hint.hint_code {
            // EcRecover Hint
            HintCode::BuiltIn(BuiltInHint::EcRecover) => Self::process_hint_ecrecover(&hint),

            // Big Integer Arithmetic Hints
            HintCode::BuiltIn(BuiltInHint::RedMod256) => Self::process_hint_redmod256(&hint),
            HintCode::BuiltIn(BuiltInHint::AddMod256) => Self::process_hint_addmod256(&hint),
            HintCode::BuiltIn(BuiltInHint::MulMod256) => Self::process_hint_mulmod256(&hint),
            HintCode::BuiltIn(BuiltInHint::DivRem256) => Self::process_hint_divrem256(&hint),
            HintCode::BuiltIn(BuiltInHint::WPow256) => Self::process_hint_wpow256(&hint),
            HintCode::BuiltIn(BuiltInHint::OMul256) => Self::process_hint_omul256(&hint),
            HintCode::BuiltIn(BuiltInHint::WMul256) => Self::process_hint_wmul256(&hint),

            // Modular Exponentiation Hint
            HintCode::BuiltIn(BuiltInHint::ModExp) => Self::process_hint_modexp(&hint),

            // BN254 hints
            HintCode::BuiltIn(BuiltInHint::IsOnCurveBn254) => {
                Self::process_hint_is_on_curve_bn254(&hint)
            }
            HintCode::BuiltIn(BuiltInHint::ToAffineBn254) => {
                Self::process_hint_to_affine_bn254(&hint)
            }
            HintCode::BuiltIn(BuiltInHint::AddBn254) => Self::process_hint_add_bn254(&hint),
            HintCode::BuiltIn(BuiltInHint::MulBn254) => Self::process_hint_mul_bn254(&hint),
            HintCode::BuiltIn(BuiltInHint::ToAffineTwistBn254) => {
                Self::process_hint_to_affine_twist_bn254(&hint)
            }
            HintCode::BuiltIn(BuiltInHint::IsOnCurveTwistBn254) => {
                Self::process_hint_is_on_curve_twist_bn254(&hint)
            }
            HintCode::BuiltIn(BuiltInHint::IsOnSubgroupTwistBn254) => {
                Self::process_hint_is_on_subgroup_twist_bn254(&hint)
            }
            HintCode::BuiltIn(BuiltInHint::PairingBatchBn254) => {
                Self::process_hint_pairing_batch_bn254(&hint)
            }

            // BLS12-381 hints
            HintCode::BuiltIn(BuiltInHint::MulFp12Bls12_381) => {
                Self::process_hint_mul_fp_bls12_381(&hint)
            }
            HintCode::BuiltIn(BuiltInHint::DecompressBls12_381) => {
                Self::process_hint_decompress_bls12_381(&hint)
            }
            HintCode::BuiltIn(BuiltInHint::IsOnCurveBls12_381) => {
                Self::process_hint_is_on_curve_bls12_381(&hint)
            }
            HintCode::BuiltIn(BuiltInHint::IsOnSubgroupBls12_381) => {
                Self::process_hint_is_on_subgroup_bls12_381(&hint)
            }
            HintCode::BuiltIn(BuiltInHint::AddBls12_381) => Self::process_hint_add_bls12_381(&hint),
            HintCode::BuiltIn(BuiltInHint::ScalarMulBls12_381) => {
                Self::process_hint_scalar_mul_bls12_381(&hint)
            }
            HintCode::BuiltIn(BuiltInHint::DecompressTwistBls12_381) => {
                Self::process_hint_decompress_twist_bls12_381(&hint)
            }
            HintCode::BuiltIn(BuiltInHint::IsOnCurveTwistBls12_381) => {
                Self::process_hint_is_on_curve_twist_bls12_381(&hint)
            }
            HintCode::BuiltIn(BuiltInHint::IsOnSubgroupTwistBls12_381) => {
                Self::process_hint_is_on_subgroup_twist_bls12_381(&hint)
            }
            HintCode::BuiltIn(BuiltInHint::AddTwistBls12_381) => {
                Self::process_hint_add_twist_bls12_381(&hint)
            }
            HintCode::BuiltIn(BuiltInHint::ScalarMulTwistBls12_381) => {
                Self::process_hint_scalar_mul_twist_bls12_381(&hint)
            }
            HintCode::BuiltIn(BuiltInHint::MillerLoopBls12_381) => {
                Self::process_hint_miller_loop_bls12_381(&hint)
            }
            HintCode::BuiltIn(BuiltInHint::FinalExpBls12_381) => {
                Self::process_hint_final_exp_bls12_381(&hint)
            }

            // Custom hints
            HintCode::Custom(code) => {
                if let Some(handler) = custom_handlers.get(&code) {
                    handler(&hint.data)
                } else {
                    Err(anyhow::anyhow!("Unknown custom hint code: {:#x}", code))
                }
            }

            // Control codes and Noop are handled before dispatch
            _ => Err(anyhow::anyhow!("Unexpected hint code: {:#x}", hint.hint_code.to_u32())),
        }
    }

    /// Processes a [`ECRECOVER`] hint.
    #[inline]
    fn process_hint_ecrecover(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::secp256k1_ecdsa_verify_hint(&hint.data)
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Processes a [`REDMOD256`] hint.
    #[inline]
    fn process_hint_redmod256(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::redmod256_hint(&hint.data).map_err(|e| anyhow::anyhow!(e))
    }
    /// Processes a [`ADDMOD256`] hint.
    #[inline]
    fn process_hint_addmod256(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::addmod256_hint(&hint.data).map_err(|e| anyhow::anyhow!(e))
    }
    /// Processes a [`MULMOD256`] hint.
    #[inline]
    fn process_hint_mulmod256(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::mulmod256_hint(&hint.data).map_err(|e| anyhow::anyhow!(e))
    }
    /// Processes a [`DIVREM256`] hint.
    #[inline]
    fn process_hint_divrem256(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::divrem256_hint(&hint.data).map_err(|e| anyhow::anyhow!(e))
    }
    /// Processes a [`WPOW256`] hint.
    #[inline]
    fn process_hint_wpow256(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::wpow256_hint(&hint.data).map_err(|e| anyhow::anyhow!(e))
    }
    /// Processes a [`OMUL256`] hint.
    #[inline]
    fn process_hint_omul256(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::omul256_hint(&hint.data).map_err(|e| anyhow::anyhow!(e))
    }
    /// Processes a [`WMUL256`] hint.
    #[inline]
    fn process_hint_wmul256(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::wmul256_hint(&hint.data).map_err(|e| anyhow::anyhow!(e))
    }

    /// Processes a [`MODEXP`] hint.
    #[inline]
    fn process_hint_modexp(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::modexp_hint(&hint.data).map_err(|e| anyhow::anyhow!(e))
    }

    #[inline]
    fn process_hint_is_on_curve_bn254(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::is_on_curve_bn254_hint(&hint.data).map_err(|e| anyhow::anyhow!(e))
    }

    #[inline]
    fn process_hint_to_affine_bn254(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::to_affine_bn254_hint(&hint.data).map_err(|e| anyhow::anyhow!(e))
    }

    #[inline]
    fn process_hint_add_bn254(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::add_bn254_hint(&hint.data).map_err(|e| anyhow::anyhow!(e))
    }

    #[inline]
    fn process_hint_mul_bn254(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::mul_bn254_hint(&hint.data).map_err(|e| anyhow::anyhow!(e))
    }

    #[inline]
    fn process_hint_to_affine_twist_bn254(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::to_affine_twist_bn254_hint(&hint.data)
            .map_err(|e| anyhow::anyhow!(e))
    }

    #[inline]
    fn process_hint_is_on_curve_twist_bn254(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::is_on_curve_twist_bn254_hint(&hint.data)
            .map_err(|e| anyhow::anyhow!(e))
    }

    #[inline]
    fn process_hint_is_on_subgroup_twist_bn254(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::is_on_subgroup_twist_bn254_hint(&hint.data)
            .map_err(|e| anyhow::anyhow!(e))
    }

    #[inline]
    fn process_hint_pairing_batch_bn254(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::pairing_batch_bn254_hint(&hint.data).map_err(|e| anyhow::anyhow!(e))
    }

    #[inline]
    fn process_hint_mul_fp_bls12_381(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::mul_fp12_bls12_381_hint(&hint.data).map_err(|e| anyhow::anyhow!(e))
    }

    #[inline]
    fn process_hint_decompress_bls12_381(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::decompress_bls12_381_hint(&hint.data)
            .map_err(|e| anyhow::anyhow!(e))
    }

    #[inline]
    fn process_hint_is_on_curve_bls12_381(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::is_on_curve_bls12_381_hint(&hint.data)
            .map_err(|e| anyhow::anyhow!(e))
    }

    #[inline]
    fn process_hint_is_on_subgroup_bls12_381(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::is_on_subgroup_bls12_381_hint(&hint.data)
            .map_err(|e| anyhow::anyhow!(e))
    }

    #[inline]
    fn process_hint_add_bls12_381(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::add_bls12_381_hint(&hint.data).map_err(|e| anyhow::anyhow!(e))
    }

    #[inline]
    fn process_hint_scalar_mul_bls12_381(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::scalar_mul_bls12_381_hint(&hint.data)
            .map_err(|e| anyhow::anyhow!(e))
    }

    #[inline]
    fn process_hint_decompress_twist_bls12_381(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::decompress_twist_bls12_381_hint(&hint.data)
            .map_err(|e| anyhow::anyhow!(e))
    }

    #[inline]
    fn process_hint_is_on_curve_twist_bls12_381(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::is_on_curve_twist_bls12_381_hint(&hint.data)
            .map_err(|e| anyhow::anyhow!(e))
    }

    #[inline]
    fn process_hint_is_on_subgroup_twist_bls12_381(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::is_on_subgroup_twist_bls12_381_hint(&hint.data)
            .map_err(|e| anyhow::anyhow!(e))
    }

    #[inline]
    fn process_hint_add_twist_bls12_381(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::add_twist_bls12_381_hint(&hint.data).map_err(|e| anyhow::anyhow!(e))
    }

    #[inline]
    fn process_hint_scalar_mul_twist_bls12_381(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::scalar_mul_twist_bls12_381_hint(&hint.data)
            .map_err(|e| anyhow::anyhow!(e))
    }

    #[inline]
    fn process_hint_miller_loop_bls12_381(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::miller_loop_bls12_381_hint(&hint.data)
            .map_err(|e| anyhow::anyhow!(e))
    }

    #[inline]
    fn process_hint_final_exp_bls12_381(hint: &PrecompileHint) -> Result<Vec<u64>> {
        ziskos_hints::handlers::final_exp_bls12_381_hint(&hint.data).map_err(|e| anyhow::anyhow!(e))
    }
}

impl<HS: StreamSink + Send + Sync + 'static> Drop for HintsProcessor<HS> {
    fn drop(&mut self) {
        // Signal drainer thread to shut down
        self.state.shutdown.store(true, Ordering::Release);
        self.state.drain_signal.notify_all();

        // Join the drainer thread to ensure clean shutdown
        // Safety: We only take the value once in drop
        unsafe {
            let handle = ManuallyDrop::take(&mut self.drainer_thread);
            let _ = handle.join();
        }
    }
}

impl<HS: StreamSink + Send + Sync + 'static> StreamProcessor for HintsProcessor<HS> {
    fn process(&self, data: &[u64], first_batch: bool) -> Result<bool> {
        self.process_hints(data, first_batch)
    }
}

#[cfg(test)]
mod tests {
    use zisk_common::HintCode;

    use super::*;

    struct NullHints;

    impl StreamSink for NullHints {
        fn submit(&self, _processed: Vec<u64>) -> Result<()> {
            Ok(())
        }
    }

    fn make_header(hint_type: u32, length: u32) -> u64 {
        ((hint_type as u64) << 32) | (length as u64)
    }

    fn make_ctrl_header(ctrl: u32, length: u32) -> u64 {
        make_header(ctrl, length)
    }

    fn processor() -> HintsProcessor<NullHints> {
        HintsProcessor::builder(NullHints).num_threads(2).build().unwrap()
    }

    // Positive tests
    #[test]
    fn test_single_result_hint_non_blocking() {
        let p = processor();
        let data =
            vec![make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 2), 0x111, 0x222];

        // Dispatch should succeed and be non-blocking
        assert!(p.process_hints(&data, false).is_ok());
        // Wait for completion should succeed
        assert!(p.wait_for_completion().is_ok());

        // Buffer should be empty after completion
        let queue = p.state.queue.lock().unwrap();
        assert!(queue.buffer.is_empty());
        assert_eq!(queue.next_drain_seq, 1);
    }

    #[test]
    fn test_multiple_hints_ordered_output() {
        let p = processor();
        let data = vec![
            make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 1),
            0x111,
            make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 1),
            0x222,
            make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 1),
            0x333,
        ];
        assert!(p.process_hints(&data, false).is_ok());
        assert!(p.wait_for_completion().is_ok());

        // Verify all hints were processed (buffer empty, next_drain_seq advanced)
        let queue = p.state.queue.lock().unwrap();
        assert!(queue.buffer.is_empty());
        assert_eq!(queue.next_drain_seq, 3);
    }

    #[test]
    fn test_multiple_calls_global_sequence() {
        let p = processor();
        let data1 = vec![make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 1), 0xAAA];
        let data2 = vec![make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 1), 0xBBB];

        assert!(p.process_hints(&data1, false).is_ok());
        assert!(p.process_hints(&data2, false).is_ok());
        assert!(p.wait_for_completion().is_ok());

        // Verify sequence continued across calls
        let queue = p.state.queue.lock().unwrap();
        assert_eq!(queue.next_drain_seq, 2);
        assert!(queue.buffer.is_empty());
    }

    #[test]
    fn test_empty_input_ok() {
        let p = processor();
        let data: Vec<u64> = vec![];
        assert!(p.process_hints(&data, false).is_ok());
        assert!(p.wait_for_completion().is_ok());

        // No hints processed
        let queue = p.state.queue.lock().unwrap();
        assert_eq!(queue.next_drain_seq, 0);
    }

    // Negative tests
    #[test]
    fn test_unknown_hint_type_returns_error() {
        let p = processor();
        let data = vec![make_header(999, 1), 0x1234];

        // Should return error immediately during validation
        let result = p.process_hints(&data, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown custom hint code"));
    }

    #[test]
    fn test_error_stops_wait() {
        let p = processor();
        // First valid, then invalid type
        let data = vec![
            make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 1),
            0x111,
            make_header(999, 0),
        ];

        // Should error immediately when encountering invalid hint type
        let result = p.process_hints(&data, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown custom hint code"));
    }

    #[test]
    fn test_reset_clears_error() {
        let p = processor();
        let bad = vec![make_header(999, 0)];
        let result = p.process_hints(&bad, false);

        // Should get synchronous error for invalid hint type
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown custom hint code"));

        // Reset should clear any error state
        p.reset();
        assert!(!p.state.error_flag.load(Ordering::Acquire));

        // Should be able to process new hints after reset
        let good = vec![make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 1), 0x42];
        assert!(p.process_hints(&good, false).is_ok());
        assert!(p.wait_for_completion().is_ok());

        let queue = p.state.queue.lock().unwrap();
        assert_eq!(queue.next_drain_seq, 1);
    }

    // Stream control tests
    #[test]
    fn test_stream_start_resets_state() {
        let p = processor();

        // First batch increments sequence
        let batch1 = vec![make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 1), 0x01];
        p.process_hints(&batch1, false).unwrap();
        p.wait_for_completion().unwrap();

        // Sequence should be at 1
        {
            let queue = p.state.queue.lock().unwrap();
            assert_eq!(queue.next_drain_seq, 1);
        }

        // Send START control - should reset sequence
        let start = vec![make_ctrl_header(HintCode::Ctrl(CtrlHint::Start).to_u32(), 0)];
        p.process_hints(&start, true).unwrap();

        // Sequence should be reset to 0
        {
            let queue = p.state.queue.lock().unwrap();
            assert_eq!(queue.next_drain_seq, 0);
            assert!(queue.buffer.is_empty());
        }

        // Process new batch
        let batch2 = vec![make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 1), 0x02];
        p.process_hints(&batch2, false).unwrap();

        let end = vec![make_ctrl_header(HintCode::Ctrl(CtrlHint::End).to_u32(), 0)];
        p.process_hints(&end, false).unwrap();

        // Should have processed 1 hint (starting from 0 again)
        let queue = p.state.queue.lock().unwrap();
        assert_eq!(queue.next_drain_seq, 1);
    }

    #[test]
    fn test_stream_end_waits_until_completion() {
        let p = processor();

        // Dispatch hints
        let data = vec![
            make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 1),
            0x10,
            make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 1),
            0x20,
        ];
        p.process_hints(&data, false).unwrap();

        // END should wait internally
        let end = vec![make_ctrl_header(HintCode::Ctrl(CtrlHint::End).to_u32(), 0)];
        p.process_hints(&end, false).unwrap();

        // Buffer should already be empty
        {
            let queue = p.state.queue.lock().unwrap();
            assert!(queue.buffer.is_empty());
            assert_eq!(queue.next_drain_seq, 2);
        }

        // Explicit wait should be instant
        assert!(p.wait_for_completion().is_ok());
    }

    #[test]
    fn test_stream_cancel_returns_error() {
        let p = processor();
        let cancel = vec![make_ctrl_header(HintCode::Ctrl(CtrlHint::Cancel).to_u32(), 0)];

        let result = p.process_hints(&cancel, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cancelled"));

        // Error flag should be set
        assert!(p.state.error_flag.load(Ordering::Acquire));
    }

    #[test]
    fn test_stream_error_signal_returns_error() {
        let p = processor();
        let signal_err = vec![make_ctrl_header(HintCode::Ctrl(CtrlHint::Error).to_u32(), 0)];

        let result = p.process_hints(&signal_err, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("error"));

        // Error flag should be set
        assert!(p.state.error_flag.load(Ordering::Acquire));
    }

    #[test]
    fn test_ctrl_start_must_be_first_in_batch() {
        let p = processor();

        // CTRL_START not at position 0 should fail
        let data = vec![
            make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 1),
            0x42,
            make_ctrl_header(HintCode::Ctrl(CtrlHint::Start).to_u32(), 0),
        ];

        let result = p.process_hints(&data, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be the first hint"));
    }

    #[test]
    fn test_ctrl_start_only_in_first_batch() {
        let p = processor();

        // First batch is ok
        let batch1 = vec![make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 1), 0x01];
        p.process_hints(&batch1, false).unwrap();

        // CTRL_START in non-first batch should fail
        let start = vec![make_ctrl_header(HintCode::Ctrl(CtrlHint::Start).to_u32(), 0)];
        let result = p.process_hints(&start, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("first message in the stream"));
    }

    #[test]
    fn test_ctrl_end_must_be_last() {
        let p = processor();

        // CTRL_END not at end should fail
        let data = vec![
            make_ctrl_header(HintCode::Ctrl(CtrlHint::End).to_u32(), 0),
            make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 1),
            0x42,
        ];

        let result = p.process_hints(&data, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be the last hint"));
    }

    #[test]
    fn test_sink_receives_correct_data() {
        use std::sync::{Arc, Mutex};

        struct RecordingSink {
            received: Arc<Mutex<Vec<Vec<u64>>>>,
        }

        impl StreamSink for RecordingSink {
            fn submit(&self, processed: Vec<u64>) -> Result<()> {
                self.received.lock().unwrap().push(processed);
                Ok(())
            }
        }

        let received = Arc::new(Mutex::new(Vec::new()));
        let sink = RecordingSink { received: Arc::clone(&received) };
        let p = HintsProcessor::builder(sink).num_threads(2).build().unwrap();

        // Send some data
        let data = vec![
            make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 2),
            0xAAA,
            0xBBB,
            make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 1),
            0xCCC,
        ];

        p.process_hints(&data, false).unwrap();
        p.wait_for_completion().unwrap();

        // Verify sink received correct data in order
        let received = received.lock().unwrap();
        assert_eq!(received.len(), 2);
        assert_eq!(received[0], vec![0xAAA, 0xBBB]);
        assert_eq!(received[1], vec![0xCCC]);
    }

    #[test]
    fn test_sink_error_stops_processing() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        struct FailingSink {
            should_fail: Arc<AtomicBool>,
        }

        impl StreamSink for FailingSink {
            fn submit(&self, _processed: Vec<u64>) -> Result<()> {
                if self.should_fail.load(Ordering::Acquire) {
                    Err(anyhow::anyhow!("Sink error"))
                } else {
                    Ok(())
                }
            }
        }

        let should_fail = Arc::new(AtomicBool::new(false));
        let sink = FailingSink { should_fail: Arc::clone(&should_fail) };
        let p = HintsProcessor::builder(sink).num_threads(2).build().unwrap();

        // First batch succeeds
        let data1 = vec![make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 1), 0x01];
        assert!(p.process_hints(&data1, false).is_ok());
        assert!(p.wait_for_completion().is_ok());

        // Make sink fail
        should_fail.store(true, Ordering::Release);

        // Second batch should trigger sink error
        let data2 = vec![make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 1), 0x02];
        assert!(p.process_hints(&data2, false).is_ok());

        // Wait should detect the error from drainer thread
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(p.state.error_flag.load(Ordering::Acquire));
    }

    // Builder tests
    #[test]
    fn test_builder_configuration() {
        // Default builder - stats disabled
        let p1 = HintsProcessor::builder(NullHints).build().unwrap();
        assert!(p1.stats.is_none());

        // Explicitly disabled stats
        let p2 = HintsProcessor::builder(NullHints).enable_stats(false).build().unwrap();
        assert!(p2.stats.is_none());

        // Stats enabled
        let p3 = HintsProcessor::builder(NullHints).enable_stats(true).build().unwrap();
        assert!(p3.stats.is_some());

        // Custom threads
        let p4 = HintsProcessor::builder(NullHints).num_threads(4).build().unwrap();
        let data = vec![make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 1), 0x42];
        assert!(p4.process_hints(&data, false).is_ok());
        assert!(p4.wait_for_completion().is_ok());

        // Chaining multiple options
        let p5 =
            HintsProcessor::builder(NullHints).num_threads(8).enable_stats(true).build().unwrap();
        assert!(p5.stats.is_some());
    }

    // Stress test
    #[test]
    fn test_stress_throughput() {
        use std::time::Instant;

        let p = HintsProcessor::builder(NullHints).num_threads(32).build().unwrap();

        // Generate a large batch of hints
        const NUM_HINTS: usize = 100_000;
        let mut data = Vec::with_capacity(NUM_HINTS * 2);

        for i in 0..NUM_HINTS {
            data.push(make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 1));
            data.push(i as u64);
        }

        let start = Instant::now();
        p.process_hints(&data, false).unwrap();
        p.wait_for_completion().unwrap();
        let duration = start.elapsed();

        let ops_per_sec = NUM_HINTS as f64 / duration.as_secs_f64();
        println!("\n========================================");
        println!("Stress Test Results:");
        println!("  Total hints: {}", NUM_HINTS);
        println!("  Duration: {:.3}s", duration.as_secs_f64());
        println!("  Throughput: {:.0} ops/sec", ops_per_sec);
        println!("  Avg latency: {:.2}µs per hint", duration.as_micros() as f64 / NUM_HINTS as f64);
        println!("========================================\n");

        // Sanity check: should be able to process at least 10k ops/sec
        assert!(ops_per_sec > 10_000.0, "Throughput too low: {:.0} ops/sec", ops_per_sec);
    }

    #[test]
    fn test_stress_concurrent_batches() {
        use std::time::Instant;

        let p = HintsProcessor::builder(NullHints).num_threads(32).build().unwrap();

        const NUM_BATCHES: usize = 1_000;
        const HINTS_PER_BATCH: usize = 100;

        let start = Instant::now();

        // Call process_hints multiple times with small batches
        for batch_id in 0..NUM_BATCHES {
            let mut data = Vec::with_capacity(HINTS_PER_BATCH * 2);
            for i in 0..HINTS_PER_BATCH {
                data.push(make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 1));
                data.push((batch_id * HINTS_PER_BATCH + i) as u64);
            }
            p.process_hints(&data, false).unwrap();
        }

        p.wait_for_completion().unwrap();
        let duration = start.elapsed();

        let total_hints = NUM_BATCHES * HINTS_PER_BATCH;
        let ops_per_sec = total_hints as f64 / duration.as_secs_f64();

        println!("\n========================================");
        println!("Multiple Batches Stress Test:");
        println!("  Number of batches: {}", NUM_BATCHES);
        println!("  Hints per batch: {}", HINTS_PER_BATCH);
        println!("  Total hints: {}", total_hints);
        println!("  Duration: {:.3}s", duration.as_secs_f64());
        println!("  Throughput: {:.0} ops/sec", ops_per_sec);
        println!("========================================\n");

        assert!(ops_per_sec > 10_000.0, "Throughput too low: {:.0} ops/sec", ops_per_sec);
    }

    #[test]
    fn test_stress_with_resets() {
        use std::time::Instant;

        let p = HintsProcessor::builder(NullHints).num_threads(32).build().unwrap();

        const ITERATIONS: usize = 100;
        const HINTS_PER_ITER: usize = 1_000;

        let start = Instant::now();

        for _iter in 0..ITERATIONS {
            // Reset at start of each iteration
            let reset = vec![make_ctrl_header(HintCode::Ctrl(CtrlHint::Start).to_u32(), 0)];
            p.process_hints(&reset, true).unwrap();

            // Process batch
            let mut data = Vec::with_capacity(HINTS_PER_ITER * 2);
            for i in 0..HINTS_PER_ITER {
                data.push(make_header(HintCode::BuiltIn(BuiltInHint::Noop).to_u32(), 1));
                data.push(i as u64);
            }
            p.process_hints(&data, false).unwrap();

            // End stream
            let end = vec![make_ctrl_header(HintCode::Ctrl(CtrlHint::End).to_u32(), 0)];
            p.process_hints(&end, false).unwrap();
        }

        let duration = start.elapsed();
        let total_hints = ITERATIONS * HINTS_PER_ITER;
        let ops_per_sec = total_hints as f64 / duration.as_secs_f64();

        println!("\n========================================");
        println!("Reset Stress Test:");
        println!("  Iterations: {}", ITERATIONS);
        println!("  Hints per iteration: {}", HINTS_PER_ITER);
        println!("  Total hints: {}", total_hints);
        println!("  Duration: {:.3}s", duration.as_secs_f64());
        println!("  Throughput: {:.0} ops/sec", ops_per_sec);
        println!("========================================\n");

        assert!(
            ops_per_sec > 5_000.0,
            "Throughput too low with resets: {:.0} ops/sec",
            ops_per_sec
        );
    }

    #[test]
    fn test_custom_handlers_ordered_with_delays() {
        use std::sync::{Arc, Mutex};
        use std::thread;
        use std::time::Duration;

        struct RecordingSink {
            received: Arc<Mutex<Vec<Vec<u64>>>>,
        }

        impl StreamSink for RecordingSink {
            fn submit(&self, processed: Vec<u64>) -> Result<()> {
                self.received.lock().unwrap().push(processed);
                Ok(())
            }
        }

        let received = Arc::new(Mutex::new(Vec::new()));
        let sink = RecordingSink { received: Arc::clone(&received) };

        // Custom hint codes
        const FAST_HINT: u32 = 0x100; // Processes instantly
        const SLOW_HINT: u32 = 0x101; // Delays 10ms
        const MED_HINT: u32 = 0x102; // Delays 5ms

        let p = HintsProcessor::builder(sink)
            .num_threads(8)
            .custom_hint(FAST_HINT, |data| {
                // No delay - returns immediately
                Ok(vec![data[0] * 2])
            })
            .custom_hint(SLOW_HINT, |data| {
                // Long delay to complete last
                thread::sleep(Duration::from_millis(10));
                Ok(vec![data[0] * 3])
            })
            .custom_hint(MED_HINT, |data| {
                // Medium delay
                thread::sleep(Duration::from_millis(5));
                Ok(vec![data[0] * 4])
            })
            .build()
            .unwrap();

        // Send hints in order: SLOW, FAST, MED
        // They should complete in order: FAST, MED, SLOW
        // But results should be returned in submission order: SLOW, FAST, MED
        let data = vec![
            make_header(SLOW_HINT, 1),
            10, // Will complete last but should be first result
            make_header(FAST_HINT, 1),
            20, // Will complete first but should be second result
            make_header(MED_HINT, 1),
            30, // Will complete second but should be third result
            make_header(FAST_HINT, 1),
            40, // Fast again
            make_header(SLOW_HINT, 1),
            50, // Slow again
        ];

        p.process_hints(&data, false).unwrap();
        p.wait_for_completion().unwrap();

        // Verify results are in submission order, not completion order
        let results = received.lock().unwrap();
        assert_eq!(results.len(), 5);
        assert_eq!(results[0], vec![30]); // SLOW: 10 * 3
        assert_eq!(results[1], vec![40]); // FAST: 20 * 2
        assert_eq!(results[2], vec![120]); // MED: 30 * 4
        assert_eq!(results[3], vec![80]); // FAST: 40 * 2
        assert_eq!(results[4], vec![150]); // SLOW: 50 * 3
    }

    #[test]
    fn test_custom_handlers_stress_ordering() {
        use std::sync::{Arc, Mutex};
        use std::thread;
        use std::time::Duration;

        struct RecordingSink {
            received: Arc<Mutex<Vec<Vec<u64>>>>,
        }

        impl StreamSink for RecordingSink {
            fn submit(&self, processed: Vec<u64>) -> Result<()> {
                self.received.lock().unwrap().push(processed);
                Ok(())
            }
        }

        let received = Arc::new(Mutex::new(Vec::new()));
        let sink = RecordingSink { received: Arc::clone(&received) };

        const VARIABLE_HINT: u32 = 0x200;

        let p = HintsProcessor::builder(sink)
            .num_threads(16)
            .custom_hint(VARIABLE_HINT, |data| {
                // Pseudo-random delay based on hash of input value (0-15ms range)
                // This creates unpredictable completion order across runs
                let hash = data[0].wrapping_mul(2654435761);
                let delay_ms = hash % 16;
                if delay_ms > 0 {
                    thread::sleep(Duration::from_millis(delay_ms));
                }
                Ok(vec![data[0] + 1000])
            })
            .build()
            .unwrap();

        // Generate pseudo-random number of hints between 100 and 500
        // Using current time as seed for variation across test runs
        use std::time::SystemTime;
        let seed =
            SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos() as u64;
        let num_hints = 100 + (seed % 401) as usize; // 100 to 500 inclusive

        let mut data = Vec::with_capacity(num_hints * 2);
        for i in 0..num_hints {
            data.push(make_header(VARIABLE_HINT, 1));
            data.push(i as u64);
        }

        p.process_hints(&data, false).unwrap();
        p.wait_for_completion().unwrap();

        // Verify all results are in correct order despite random completion times
        let results = received.lock().unwrap();
        assert_eq!(results.len(), num_hints, "Expected {} results", num_hints);
        for i in 0..num_hints {
            assert_eq!(results[i][0], i as u64 + 1000, "Result {} out of order", i);
        }
    }
}
