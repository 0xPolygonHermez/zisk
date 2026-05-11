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

/// Output-side shmem readers for the three ASM services, mapped once at worker startup.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub struct AsmShmemReaders {
    pub mt: Arc<Mutex<MTShMemReader>>,
    pub mo: Arc<Mutex<MOShMemReader>>,
    pub rh: Arc<Mutex<Option<RHShMemReader>>>,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl AsmShmemReaders {
    fn new(shm_prefix: &str, unlock_mapped_memory: bool) -> Result<Self> {
        Ok(Self {
            mt: Arc::new(Mutex::new(MTShMemReader::new(shm_prefix, unlock_mapped_memory)?)),
            mo: Arc::new(Mutex::new(MOShMemReader::new(shm_prefix, unlock_mapped_memory)?)),
            rh: Arc::new(Mutex::new(None)),
        })
    }
}

/// Shmem segments mapped once at worker startup. Shared across all programs via `Arc`.
pub struct AsmSharedResources {
    config: AsmResourcesConfig,

    /// Shared memory writer for inputs (shmem mapped once; semaphores bound per-program).
    pub inputs_shmem_writer: Arc<InputsShmemWriter>,

    /// Hints processing pipeline — `Some` only when the program was set up with hints.
    /// The precompile shmem segments are created by the C binary only in hints mode;
    /// attempting to open them without hints causes an immediate crash.
    hints_stream: Option<Arc<Mutex<ZiskStream<HintsProcessor<HintsShmem>>>>>,

    /// Pipeline for receiving inputs over a stream transport (QUIC, Unix socket).
    inputs_stream: Arc<Mutex<ZiskStream<InputsShmemWriter>>>,

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub readers: AsmShmemReaders,
}

impl std::fmt::Debug for AsmSharedResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hints_init =
            self.hints_stream.as_ref().and_then(|s| s.lock().ok().map(|g| g.is_initialized()));
        f.debug_struct("AsmSharedResources")
            .field("config", &self.config)
            .field("hints_stream", &hints_init)
            .finish_non_exhaustive()
    }
}

impl AsmSharedResources {
    /// Map all shmem segments. `shm_prefix` must already have been created via Phase 1.
    /// Semaphores are NOT opened here — call `bind_semaphores` on `AsmResources` before use.
    ///
    /// `with_hints` must match the value used during setup: the C binary only creates the
    /// per-service precompile shmem segments when hints are enabled, so passing `true` here
    /// without the C binary having created them will panic in `SharedMemoryWriter::new`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        world_rank: i32,
        local_rank: i32,
        unlock_mapped_memory: bool,
        verbose_mode: proofman_common::VerboseMode,
        mpi_broadcast_fn: Option<MpiBroadcastFn>,
        init_rom: bool,
        with_hints: bool,
        shm_prefix: &str,
    ) -> Result<Self> {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let readers = AsmShmemReaders::new(shm_prefix, unlock_mapped_memory)?;

        let control_writer = Arc::new(ControlShmem::new(shm_prefix, unlock_mapped_memory)?);

        let config = AsmResourcesConfig { world_rank, local_rank, unlock_mapped_memory };

        let inputs_shmem_writer = Arc::new(InputsShmemWriter::new(
            shm_prefix,
            unlock_mapped_memory,
            control_writer.clone(),
        )?);

        let inputs_stream =
            Arc::new(Mutex::new(ZiskStream::from_arc(Arc::clone(&inputs_shmem_writer))));

        let hints_stream = if with_hints {
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
            readers,
            inputs_shmem_writer,
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

        if let Some(hints_stream) = &shared.hints_stream {
            let processor = hints_stream.lock().unwrap().get_processor();
            processor.hints_sink().bind_semaphores(sem_prefix)?;
        }

        Ok(Self { shared, asm_services })
    }

    /// Returns the concrete hints processor, or `Err` if not set up with hints.
    pub fn get_hints_processor(&self) -> Result<Arc<HintsProcessor<HintsShmem>>> {
        self.shared
            .hints_stream
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Program was not set up with hints"))?
            .lock()
            .map(|g| g.get_processor())
            .map_err(|e| anyhow::anyhow!("hints_stream lock poisoned: {e}"))
    }

    /// Update the active ASM services for this partition.
    ///
    /// Call once per partition start (not per stream reset).
    /// `is_first_partition` controls whether the ROM histogram service (RH) is active.
    pub fn set_active_services(&self, is_first_partition: bool) -> Result<()> {
        let Some(hints_stream) = &self.shared.hints_stream else { return Ok(()) };
        let processor = hints_stream
            .lock()
            .map_err(|e| anyhow::anyhow!("Mutex lock poisoned: {e}"))?
            .get_processor();
        let services = if is_first_partition {
            &AsmServices::SERVICES[..]
        } else {
            &AsmServices::SERVICES[..2]
        };
        processor.hints_sink().set_active_services(services)
    }

    /// Submit hint data directly to the shmem sink, bypassing the processing pipeline.
    ///
    /// Used in the gRPC streaming path where hints arrive pre-processed.
    pub fn submit_hint_direct(&self, data: &[u64]) -> Result<()> {
        let processor = self.get_hints_processor()?;
        processor.hints_sink().submit(data)
    }

    pub fn start_stream(&self) -> Result<()> {
        let Some(hints_stream) = &self.shared.hints_stream else {
            return Err(anyhow::anyhow!("Program was not set up with hints"));
        };
        hints_stream.lock().map_err(|e| anyhow::anyhow!("Mutex lock poisoned: {e}"))?.start_stream()
    }

    pub fn set_hints_stream_src(&self, stream: StreamSource) -> Result<()> {
        let Some(hints_stream) = &self.shared.hints_stream else {
            return Err(anyhow::anyhow!("Program was not set up with hints"));
        };
        hints_stream
            .lock()
            .map_err(|e| anyhow::anyhow!("Mutex lock poisoned: {e}"))?
            .set_stream_src(stream)
    }

    pub fn is_hints_stream_initialized(&self) -> bool {
        self.shared
            .hints_stream
            .as_ref()
            .and_then(|s| s.lock().ok().map(|g| g.is_initialized()))
            .unwrap_or(false)
    }

    /// Soft-reset the C children: set `ResetFlag=1` and post `sem_input_avail`
    /// so children stuck in `_wait_for_input_avail` wake immediately, see the
    /// flag, and abort. Children stuck in `_wait_for_prec_avail` are NOT woken
    /// explicitly — they exit on the next `sem_timedwait` expiry (≤ 5 s, see
    /// `c_provided.c::_wait_for_prec_avail`) when they re-check the flag.
    /// `RECOVERY_TIMEOUT` covers that slip.
    ///
    /// The flag MUST be set before posting the semaphore — without it, a
    /// child wakes from the post, sees flag=0, and goes back to sleep forever.
    pub fn signal_cancellation(&self) -> Result<()> {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        self.shared.inputs_shmem_writer.set_reset_flag();

        self.shared.inputs_shmem_writer.notify_all_services()?;
        Ok(())
    }

    pub fn reset(&self) {
        if let Some(s) = &self.shared.hints_stream {
            s.lock().expect("hints_stream mutex poisoned").reset();
        }
        self.shared.inputs_shmem_writer.reset();
    }

    pub fn config(&self) -> &AsmResourcesConfig {
        &self.shared.config
    }

    pub fn asm_services(&self) -> &AsmServices {
        &self.asm_services
    }

    pub fn set_inputs_stream_src(&self, stream: StreamSource) -> Result<()> {
        self.shared
            .inputs_stream
            .lock()
            .map_err(|e| anyhow::anyhow!("Mutex lock poisoned: {e}"))?
            .set_stream_src(stream)?;
        Ok(())
    }

    pub fn start_inputs_stream(&self) -> Result<()> {
        self.shared
            .inputs_stream
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

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub fn readers(&self) -> &AsmShmemReaders {
        &self.shared.readers
    }
}

impl Drop for AsmResources {
    fn drop(&mut self) {
        // Shut down ASM microservices
        self.shared.inputs_shmem_writer.unbind_semaphores();
        if let Some(hints_stream) = &self.shared.hints_stream {
            if let Ok(g) = hints_stream.lock() {
                g.get_processor().hints_sink().unbind_semaphores();
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
        // The ASM service children don't unlink shmem on exit (the
        // `delete_*_shm` flags aren't set for them), so the parent must
        // unlink the `/dev/shm/{shm_prefix}*` and `sem.{sem_prefix}*`
        // entries here. Otherwise GBs of `_input`, `_ram`, `_rom` files
        // leak until the next worker startup runs `cleanup_stale_shmem`.
        self.asm_services.cleanup_my_shmem();
    }
}
