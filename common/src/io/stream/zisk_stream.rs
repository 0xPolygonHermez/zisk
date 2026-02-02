//! ZiskStream is responsible for reading precompile hints from a stream source and sent to a hints processor.

use anyhow::Result;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use crate::io::{StreamRead, StreamSource};

pub trait StreamProcessor: Send + Sync + 'static {
    /// Process data and return the processed result along with a flag indicating if CTRL_END was encountered.
    ///
    /// # Returns
    /// A tuple of (processed_data, has_ctrl_end) where:
    /// - processed_data: Vec<u64> - The processed data
    /// - has_ctrl_end: bool - True if CTRL_END was found (signals end of batch)
    fn process(&self, data: &[u64], first_batch: bool) -> anyhow::Result<bool>;
}

/// Trait for submitting processed hints to a sink.
///
/// # Arguments
/// * `processed` - A vector of processed hints as u64 values.
///
/// # Returns
/// * `Ok(())` - If hints were successfully submitted
/// * `Err` - If submission fails
pub trait StreamSink: Send + Sync + 'static {
    fn submit(&self, processed: Vec<u64>) -> anyhow::Result<()>;
}

enum ThreadCommand {
    Process,
    Shutdown,
}

/// ZiskStream struct manages the processing of precompile hints and writing them to shared memory.
pub struct ZiskStream {
    /// The hints processor used to process hints before writing.
    hints_processor: Arc<dyn StreamProcessor>,

    /// Channel sender to communicate with the background thread.
    tx: Option<Sender<ThreadCommand>>,

    /// Join handle for the background thread.
    thread_handle: Option<JoinHandle<()>>,
}

impl ZiskStream {
    /// Create a new ZiskStream with the given processor.
    ///
    /// # Arguments
    /// * `hints_processor` - The processor used to process hints.
    ///
    /// # Returns
    /// A new `ZiskStream` instance without a running thread.
    pub fn new(hints_processor: impl StreamProcessor) -> Self {
        Self { hints_processor: Arc::new(hints_processor), tx: None, thread_handle: None }
    }

    /// Stop the current background thread if running.
    fn stop_thread(&mut self) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(ThreadCommand::Shutdown);
        }
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    /// Set a new StreamSource for the pipeline and spawn a background thread to process hints.
    ///
    /// This will stop any existing background thread and start a new one with the new stream.
    ///
    /// # Arguments
    /// * `stream` - The new StreamSource source for reading hints.
    pub fn set_hints_stream_src(&mut self, mut stream: StreamSource) -> Result<()> {
        if !stream.is_active() {
            // Stop the existing thread if running
            self.stop_thread();
            stream.open()?;
        }

        // Create a new channel for communication with the thread
        let (tx, rx) = std::sync::mpsc::channel();
        self.tx = Some(tx);

        // Clone Arc references for the thread
        let hints_processor = Arc::clone(&self.hints_processor);

        // Spawn the background thread
        let thread_handle = thread::spawn(move || {
            Self::background_thread(stream, hints_processor, rx);
        });

        self.thread_handle = Some(thread_handle);

        Ok(())
    }

    /// Background thread function that processes hints when requested.
    fn background_thread(
        mut stream: StreamSource,
        hints_processor: Arc<dyn StreamProcessor>,
        rx: Receiver<ThreadCommand>,
    ) {
        while let Ok(ThreadCommand::Process) = rx.recv() {
            if let Err(e) = Self::process_stream(&mut stream, &*hints_processor) {
                panic!("Error processing hints in background thread: {:?}", e);
            }
        }
        // Loop exits when Shutdown is received or channel is closed
    }

    /// Process all hints from the stream.
    ///
    /// Processes hints in batches until CTRL_END is encountered or the stream ends.
    fn process_stream(
        stream: &mut StreamSource,
        hints_processor: &dyn StreamProcessor,
    ) -> Result<()> {
        let mut first_batch = true;

        while let Some(hints) = stream.next()? {
            let hints = crate::reinterpret_vec(hints)?;
            let has_ctrl_end = hints_processor.process(&hints, first_batch)?;

            first_batch = false;

            // Break if CTRL_END was encountered
            if has_ctrl_end {
                break;
            }
        }

        Ok(())
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
        if let Some(tx) = &self.tx {
            tx.send(ThreadCommand::Process).map_err(|e| {
                anyhow::anyhow!("Failed to send process command to background thread: {}", e)
            })?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("No background thread running. Call set_hints_stream first."))
        }
    }
}

impl Drop for ZiskStream {
    fn drop(&mut self) {
        self.stop_thread();
    }
}
