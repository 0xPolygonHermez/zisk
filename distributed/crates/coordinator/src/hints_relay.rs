//! Precompile Hints Relay

use anyhow::Result;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use zisk_common::{
    io::StreamProcessor, CtrlHint, HintCode, PartialPrecompileHint, PrecHintParseResult,
    PrecompileHint,
};
use zisk_distributed_common::StreamMessageKind;

type AsyncDispatcher = Arc<
    dyn Fn(u32, StreamMessageKind, Vec<u8>) -> Pin<Box<dyn Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

pub struct PrecompileHintsRelay {
    sequence_number: Arc<AtomicU32>,
    dispatcher: AsyncDispatcher,
    runtime_handle: tokio::runtime::Handle,

    /// Buffer for incomplete hint data between batches
    pending_partial: Mutex<Option<PartialPrecompileHint>>,

    /// Maximum allowed buffer size in bytes (to prevent unbounded growth)
    max_buffer_size: usize,
}

impl PrecompileHintsRelay {
    pub fn new<F, Fut>(dispatcher: F) -> Self
    where
        F: Fn(u32, StreamMessageKind, Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let dispatcher = Arc::new(
            move |seq: u32,
                  stream_type: StreamMessageKind,
                  payload: Vec<u8>|
                  -> Pin<Box<dyn Future<Output = ()> + Send>> {
                Box::pin(dispatcher(seq, stream_type, payload))
            },
        );

        Self {
            sequence_number: Arc::new(AtomicU32::new(0)),
            dispatcher,
            runtime_handle: tokio::runtime::Handle::current(),
            pending_partial: Mutex::new(None),
            max_buffer_size: Self::DEFAULT_MAX_BUFFER_SIZE,
        }
    }

    /// Default maximum buffer size: 128KB in bytes
    const DEFAULT_MAX_BUFFER_SIZE: usize = 128 * 1024;

    pub fn process_hints(&self, hints: &[u64], first_batch: bool) -> Result<bool> {
        let mut has_ctrl_start = false;
        let mut has_ctrl_end = false;

        // Take any pending partial hint from previous batch
        let mut pending_partial = self.pending_partial.lock().unwrap().take();

        // Parse hints and dispatch to pool
        let mut idx = 0;
        while idx < hints.len() {
            let (parsed_hint, consumed) = PrecompileHint::from_u64_slice(
                hints,
                idx,
                true,
                pending_partial.take(),
                self.max_buffer_size,
            )?;
            let hint = match parsed_hint {
                PrecHintParseResult::Complete(hint) => hint,
                PrecHintParseResult::Partial(partial) => {
                    // Store partial for next batch and exit loop
                    *self.pending_partial.lock().unwrap() = Some(partial);
                    break;
                }
            };
            let length = consumed;

            // Validate hint type is in valid range before accessing stats array

            // CTRL_START must be the first message of the first batch
            if hint.hint_code == HintCode::Ctrl(CtrlHint::Start) {
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
                has_ctrl_start = true;
            }

            if has_ctrl_end {
                return Err(anyhow::anyhow!(
                    "Received hint after CTRL_END: type {} at index {}",
                    hint.hint_code,
                    idx
                ));
            }
            has_ctrl_end = hint.hint_code == HintCode::Ctrl(CtrlHint::End);

            idx += length + 1;
        }

        if has_ctrl_start {
            self.send_hints_start();
        }

        // Call async dispatcher - blocks on async work for zero overhead
        self.send_hints_data(hints.to_vec());

        if has_ctrl_end {
            self.send_hints_end();
        }

        Ok(has_ctrl_end)
    }

    fn send_hints_start(&self) {
        let seq_num = self.sequence_number.fetch_add(1, Ordering::SeqCst);

        self.runtime_handle.block_on((self.dispatcher)(seq_num, StreamMessageKind::Start, vec![]));
    }

    fn send_hints_data(&self, hints: Vec<u64>) {
        let seq_num = self.sequence_number.fetch_add(1, Ordering::SeqCst);

        // Convert Vec<u64> to Vec<u8> for wire protocol
        let payload = unsafe {
            let mut hints_vec = hints.to_vec();
            let ptr = hints_vec.as_mut_ptr() as *mut u8;
            let len = hints_vec.len() * std::mem::size_of::<u64>();
            let capacity = hints_vec.capacity() * std::mem::size_of::<u64>();
            std::mem::forget(hints_vec);
            Vec::from_raw_parts(ptr, len, capacity)
        };

        self.runtime_handle.block_on((self.dispatcher)(seq_num, StreamMessageKind::Data, payload));
    }

    fn send_hints_end(&self) {
        let seq_num = self.sequence_number.fetch_add(1, Ordering::SeqCst);

        self.runtime_handle.block_on((self.dispatcher)(seq_num, StreamMessageKind::End, vec![]));
    }
}

impl StreamProcessor for PrecompileHintsRelay {
    fn process(&self, data: &[u64], first_batch: bool) -> Result<bool> {
        self.process_hints(data, first_batch)
    }
}
