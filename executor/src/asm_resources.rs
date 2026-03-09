use std::sync::{Arc, Mutex};

use anyhow::Result;
use asm_runner::HintsFile;
use asm_runner::HintsShmem;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use asm_runner::{MOOutputShmem, MTOutputShmem, RHOutputShmem, SharedMemoryWriter};
use precompiles_hints::{HintsProcessor, MpiBroadcastFn};
use std::sync::atomic::{AtomicBool, Ordering};
use zisk_common::io::StreamSink;
use zisk_common::io::{StreamProcessor, StreamSource, ZiskStream};

/// Encapsulates assembly-related resources including shared memory and hints stream.
#[derive(Clone)]
pub struct AsmResources {
    /// Optional baseline port to communicate with assembly microservices.
    pub base_port: Option<u16>,

    /// Local rank for distributed execution.
    pub local_rank: i32,

    /// Map unlocked flag.
    pub unlock_mapped_memory: bool,

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub asm_shmem_mt: Arc<Mutex<MTOutputShmem>>,
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub asm_shmem_mo: Arc<Mutex<MOOutputShmem>>,
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub asm_shmem_rh: Arc<Mutex<Option<RHOutputShmem>>>,
    /// Shared memory writers for each assembly service.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub shmem_input_writer: Arc<Mutex<Option<SharedMemoryWriter>>>,

    pub hints_sink: Option<Arc<dyn StreamSink>>,

    /// Pipeline for handling precompile hints.
    pub hints_stream: Option<Arc<Mutex<ZiskStream>>>,

    hints_stream_initialized: Arc<AtomicBool>,
}

impl std::fmt::Debug for AsmResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsmResources")
            .field("base_port", &self.base_port)
            .field("local_rank", &self.local_rank)
            .field("unlock_mapped_memory", &self.unlock_mapped_memory)
            .finish_non_exhaustive()
    }
}

impl AsmResources {
    pub fn new(
        local_rank: i32,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
        verbose_mode: proofman_common::VerboseMode,
        with_hints: bool,
        mpi_broadcast_fn: Option<MpiBroadcastFn>,
    ) -> Self {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let asm_shmem_mt = MTOutputShmem::new(local_rank, base_port, unlock_mapped_memory)
            .expect("asm_resources: Failed to create PreloadedMT");

        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let asm_shmem_mo = MOOutputShmem::new(local_rank, base_port, unlock_mapped_memory)
            .expect("asm_resources: Failed to create PreloadedMO");

        // Create hints pipeline with null hints stream initially.
        // Debug flag: true = HintsShmem (shared memory), false = HintsFile (file output)

        const USE_SHARED_MEMORY_HINTS: bool = true;

        let (hints_stream, hints_sink) = if with_hints {
            let (hints_processor, hints_sink): (HintsProcessor, Arc<dyn StreamSink>) =
                if USE_SHARED_MEMORY_HINTS {
                    let hints_shmem = Arc::new(
                        HintsShmem::new(base_port, local_rank, unlock_mapped_memory)
                            .expect("Failed to create HintsShmem"),
                    );

                    let mut builder = HintsProcessor::builder(hints_shmem.clone())
                        .enable_stats(verbose_mode != proofman_common::VerboseMode::Info);

                    if let Some(broadcast_fn) = mpi_broadcast_fn.clone() {
                        builder = builder.with_mpi_broadcast(move |data| broadcast_fn(data));
                    }

                    (builder.build().expect("Failed to build HintsProcessor"), hints_shmem)
                } else {
                    let hints_file = Arc::new(
                        HintsFile::new(format!("hints_results_{}.bin", local_rank))
                            .expect("Failed to create HintsFile"),
                    );

                    let mut builder = HintsProcessor::builder(hints_file.clone())
                        .enable_stats(verbose_mode != proofman_common::VerboseMode::Info);

                    if let Some(broadcast_fn) = mpi_broadcast_fn.clone() {
                        builder = builder.with_mpi_broadcast(move |data| broadcast_fn(data));
                    }

                    (builder.build().expect("Failed to build HintsProcessor"), hints_file)
                };

            (Some(Arc::new(Mutex::new(ZiskStream::new(hints_processor)))), Some(hints_sink))
        } else {
            (None, None)
        };

        Self {
            hints_stream,
            hints_stream_initialized: Arc::new(AtomicBool::new(false)),
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            asm_shmem_mt: Arc::new(Mutex::new(asm_shmem_mt)),
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            asm_shmem_mo: Arc::new(Mutex::new(asm_shmem_mo)),
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            asm_shmem_rh: Arc::new(Mutex::new(None)),
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            shmem_input_writer: Arc::new(Mutex::new(None)),
            base_port,
            local_rank,
            unlock_mapped_memory,
            hints_sink,
        }
    }

    pub fn start_stream(&self) -> Result<()> {
        if let Some(hints_stream) = &self.hints_stream {
            hints_stream.lock().unwrap().start_stream()
        } else {
            Ok(())
        }
    }

    pub fn set_hints_stream_src(&self, stream: StreamSource) -> Result<()> {
        if let Some(hints_stream) = &self.hints_stream {
            hints_stream.lock().unwrap().set_hints_stream_src(stream)?;
        } else {
            return Err(anyhow::anyhow!("Hints stream not initialized"));
        }
        self.hints_stream_initialized.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn is_hints_stream_initialized(&self) -> bool {
        self.hints_stream_initialized.load(Ordering::SeqCst)
    }

    pub fn get_hints_processor(&self) -> Option<Arc<dyn StreamProcessor>> {
        self.hints_stream.as_ref().map(|stream| stream.lock().unwrap().get_processor())
    }

    pub fn reset(&self) {
        if let Some(hints_stream) = &self.hints_stream {
            hints_stream.lock().unwrap().reset();
            self.hints_stream_initialized.store(false, Ordering::SeqCst);
        }
    }
}
