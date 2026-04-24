use std::sync::{Arc, Mutex};

use anyhow::Result;
use asm_runner::{AsmServices, ControlShmem, HintsShmem, InputsShmemWriter};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use asm_runner::{MOShMemReader, MTShMemReader, RHShMemReader};
use precompiles_hints::{HintsProcessor, MpiBroadcastFn};
use zisk_common::io::{StreamSink, StreamSource, ZiskStdin, ZiskStream};

/// Configuration for assembly resources.
#[derive(Clone)]
pub struct AsmResourcesConfig {
    pub world_rank: i32,
    pub local_rank: i32,
    pub unlock_mapped_memory: bool,
}

impl std::fmt::Debug for AsmResourcesConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsmResourcesConfig")
            .field("local_rank", &self.local_rank)
            .field("unlock_mapped_memory", &self.unlock_mapped_memory)
            .finish_non_exhaustive()
    }
}

/// Shmem segments mapped once at worker startup. Shared across all programs via `Arc`.
pub struct AsmSharedResources {
    config: AsmResourcesConfig,

    /// Shared memory writer for inputs (shmem mapped once; semaphores bound per-program).
    pub inputs_shmem_writer: Arc<InputsShmemWriter>,

    /// Hints processing pipeline (shmem mapped once; semaphores bound per-program).
    hints_stream: Option<Arc<Mutex<ZiskStream<HintsProcessor<HintsShmem>>>>>,

    /// Pipeline for receiving inputs over a stream transport (QUIC, Unix socket).
    inputs_stream: Arc<Mutex<ZiskStream<InputsShmemWriter>>>,

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub mt_shmem_reader: Arc<Mutex<MTShMemReader>>,
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub mo_shmem_reader: Arc<Mutex<MOShMemReader>>,
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub rh_shmem_reader: Arc<Mutex<Option<RHShMemReader>>>,

    use_hints: bool,
}

impl std::fmt::Debug for AsmSharedResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsmSharedResources")
            .field("config", &self.config)
            .field("hints_stream", &self.hints_stream.is_some())
            .finish_non_exhaustive()
    }
}

impl AsmSharedResources {
    /// Map all shmem segments. `shm_prefix` must already have been created via Phase 1.
    /// Semaphores are NOT opened here — call `bind_semaphores` on `AsmResources` before use.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        world_rank: i32,
        local_rank: i32,
        unlock_mapped_memory: bool,
        verbose_mode: proofman_common::VerboseMode,
        use_hints: bool,
        mpi_broadcast_fn: Option<MpiBroadcastFn>,
        init_rom: bool,
        shm_prefix: &str,
    ) -> Result<Self> {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let asm_shmem_mt = MTShMemReader::new(shm_prefix, unlock_mapped_memory)?;

        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let asm_shmem_mo = MOShMemReader::new(shm_prefix, unlock_mapped_memory)?;

        let control_writer = Arc::new(ControlShmem::new(shm_prefix, unlock_mapped_memory)?);

        let config = AsmResourcesConfig { world_rank, local_rank, unlock_mapped_memory };

        let inputs_shmem_writer = Arc::new(InputsShmemWriter::new(
            shm_prefix,
            unlock_mapped_memory,
            control_writer.clone(),
        )?);

        let inputs_stream =
            Arc::new(Mutex::new(ZiskStream::from_arc(Arc::clone(&inputs_shmem_writer))));

        let hints_stream = if use_hints {
            let active_services =
                if init_rom { &AsmServices::SERVICES[..] } else { &AsmServices::SERVICES[..2] };

            let hints_shmem = Arc::new(HintsShmem::new(
                shm_prefix,
                unlock_mapped_memory,
                control_writer,
                active_services,
            )?);

            let mut builder =
                HintsProcessor::builder(hints_shmem, Some(inputs_shmem_writer.clone()))
                    .enable_stats(verbose_mode != proofman_common::VerboseMode::Info);

            if let Some(broadcast_fn) = mpi_broadcast_fn {
                builder = builder.with_mpi_broadcast(move |data| broadcast_fn(data));
            }

            let hints_processor = builder.build()?;

            Some(Arc::new(Mutex::new(ZiskStream::new(hints_processor))))
        } else {
            None
        };

        Ok(Self {
            config,
            hints_stream,
            inputs_stream,
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            mt_shmem_reader: Arc::new(Mutex::new(asm_shmem_mt)),
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            mo_shmem_reader: Arc::new(Mutex::new(asm_shmem_mo)),
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            rh_shmem_reader: Arc::new(Mutex::new(None)),
            inputs_shmem_writer,
            use_hints,
        })
    }
}

/// Per-program assembly resources. Wraps `Arc<AsmSharedResources>` (shmem) and
/// `AsmServices` (process handles + sem_prefix). Semaphores are bound at construction
/// and unbound on drop.
pub struct AsmResources {
    shared: Arc<AsmSharedResources>,
    asm_services: AsmServices,
}

impl std::fmt::Debug for AsmResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsmResources").field("shared", &self.shared).finish_non_exhaustive()
    }
}

impl AsmResources {
    /// Create per-program resources by binding semaphores on the shared shmem.
    pub fn new(shared: Arc<AsmSharedResources>, asm_services: AsmServices) -> Result<Self> {
        let sem_prefix = asm_services.sem_prefix();
        shared.inputs_shmem_writer.bind_semaphores(sem_prefix)?;
        if shared.use_hints {
            if let Some(stream) = &shared.hints_stream {
                let processor = stream.lock().unwrap().get_processor();
                processor.hints_sink().bind_semaphores(sem_prefix)?;
            }
        }
        Ok(Self { shared, asm_services })
    }

    pub fn use_hints(&self) -> bool {
        self.shared.use_hints
    }

    /// Returns the concrete hints processor for passing to `StreamOrderingActor`.
    pub fn get_hints_processor(&self) -> Option<Arc<HintsProcessor<HintsShmem>>> {
        self.shared.hints_stream.as_ref().map(|s| s.lock().unwrap().get_processor())
    }

    /// Update the active ASM services for this partition.
    ///
    /// Call once per partition start (not per stream reset).
    /// `is_first_partition` controls whether the ROM histogram service (RH) is active.
    pub fn set_active_services(&self, is_first_partition: bool) -> Result<()> {
        if let Some(stream) = &self.shared.hints_stream {
            let processor = stream
                .lock()
                .map_err(|e| anyhow::anyhow!("Mutex lock poisoned: {e}"))?
                .get_processor();
            let services = if is_first_partition {
                &AsmServices::SERVICES[..]
            } else {
                &AsmServices::SERVICES[..2]
            };
            processor.hints_sink().set_active_services(services)?;
        }
        Ok(())
    }

    /// Submit hint data directly to the shmem sink, bypassing the processing pipeline.
    ///
    /// Used in the gRPC streaming path where hints arrive pre-processed.
    pub fn submit_hint_direct(&self, data: &[u64]) -> Result<()> {
        if let Some(stream) = &self.shared.hints_stream {
            let processor = stream
                .lock()
                .map_err(|e| anyhow::anyhow!("Mutex lock poisoned: {e}"))?
                .get_processor();
            processor.hints_sink().submit(data)?;

            Ok(())
        } else {
            Err(anyhow::anyhow!("Hints stream not initialized"))
        }
    }

    pub fn start_stream(&self) -> Result<()> {
        if let Some(hints_stream) = &self.shared.hints_stream {
            hints_stream
                .lock()
                .map_err(|e| anyhow::anyhow!("Mutex lock poisoned: {e}"))?
                .start_stream()?;
        }

        Ok(())
    }

    pub fn set_hints_stream_src(&self, stream: StreamSource) -> Result<()> {
        if let Some(hints_stream) = &self.shared.hints_stream {
            hints_stream
                .lock()
                .map_err(|e| anyhow::anyhow!("Mutex lock poisoned: {e}"))?
                .set_stream_src(stream)?;

            Ok(())
        } else {
            Err(anyhow::anyhow!("Hints stream not initialized"))
        }
    }

    pub fn is_hints_stream_initialized(&self) -> bool {
        self.shared
            .hints_stream
            .as_ref()
            .map(|s| s.lock().unwrap().is_initialized())
            .unwrap_or(false)
    }

    pub fn reset(&self) {
        if let Some(hints_stream) = &self.shared.hints_stream {
            hints_stream.lock().unwrap().reset();
        }
        // Full reset: clear shmem data and size.  Every job re-streams its
        // input via the relay, so there is nothing to preserve.
        self.shared.inputs_shmem_writer.reset();
    }

    pub fn config(&self) -> &AsmResourcesConfig {
        &self.shared.config
    }

    pub fn asm_services(&self) -> &AsmServices {
        &self.asm_services
    }

    pub fn set_inputs_stream_src(&self, stream: StreamSource) -> Result<()> {
        self.shared.inputs_stream
            .lock()
            .map_err(|e| anyhow::anyhow!("Mutex lock poisoned: {e}"))?
            .set_stream_src(stream)?;
        Ok(())
    }

    pub fn start_inputs_stream(&self) -> Result<()> {
        self.shared.inputs_stream
            .lock()
            .map_err(|e| anyhow::anyhow!("Mutex lock poisoned: {e}"))?
            .start_stream()
    }

    pub fn is_inputs_stream_initialized(&self) -> bool {
        self.shared.inputs_stream.lock().map(|s| s.is_initialized()).unwrap_or(false)
    }

    pub fn write_input(&self, stdin: &ZiskStdin) -> Result<()> {
        self.shared.inputs_shmem_writer.write_input(&stdin.read_data())
    }

    pub fn append_raw_input(&self, bytes: &[u8]) -> Result<()> {
        self.shared.inputs_shmem_writer.append_input(bytes)
    }

    // Delegated shmem reader accessors (used by EmulatorAsm)
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub fn mt_shmem_reader(&self) -> &Arc<Mutex<MTShMemReader>> {
        &self.shared.mt_shmem_reader
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub fn mo_shmem_reader(&self) -> &Arc<Mutex<MOShMemReader>> {
        &self.shared.mo_shmem_reader
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub fn rh_shmem_reader(&self) -> &Arc<Mutex<Option<RHShMemReader>>> {
        &self.shared.rh_shmem_reader
    }
}

impl Drop for AsmResources {
    fn drop(&mut self) {
        // Shut down ASM microservices
        self.shared.inputs_shmem_writer.unbind_semaphores();
        if self.shared.use_hints {
            if let Some(stream) = &self.shared.hints_stream {
                if let Ok(processor) = stream.lock().map(|g| g.get_processor()) {
                    processor.hints_sink().unbind_semaphores();
                }
            }
        }
        tracing::info!(">>> [{}] Stopping ASM microservices.", self.shared.config.world_rank);
        if let Err(e) = self.asm_services.stop_asm_services() {
            tracing::error!(
                ">>> [{}] Failed to stop ASM microservices: {}",
                self.shared.config.world_rank,
                e
            );
        }
    }
}
