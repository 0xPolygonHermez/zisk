use std::path::Path;
use std::sync::{Arc, Mutex};

use zisk_asm_runner::{
    AsmRunnerOptions, AsmService, AsmServices, ControlShmem, GpuBufferSource, HintsShmem,
    InputsShmemWriter, SegmentsClaim, SharedSegments,
};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use zisk_asm_runner::{MOShmemReader, MTShmemReader, RHShmemReader};
use zisk_common::io::{StreamSink, StreamSource, ZiskStdin, ZiskStream};
use zisk_common::RandomSeed;
use zisk_precomp_hints::{HintsProcessor, MpiBroadcastFn};

use crate::error::{ExecutorError, ExecutorResult, MutexExt};

/// Configuration for assembly resources.
#[derive(Clone)]
pub struct AsmResourcesConfig {
    /// Local rank of the process (e.g., within a node).
    pub local_rank: i32,
    /// Whether to unlock mapped memory.
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

/// Output-side shmem readers for the three ASM services, mapped when a prover's first program
/// of a hints mode is set up and shared by every program it sets up in that mode.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub struct AsmShmemReaders {
    /// Reader for the minimal trace shmem segment (MT).
    pub mt: Arc<Mutex<MTShmemReader>>,
    /// Reader for the memory-ops shmem segment (MO).
    pub mo: Arc<Mutex<MOShmemReader>>,
    /// Reader for the ROM histogram shmem segment (RH), mapped on its first read and grown
    /// when a program with a larger histogram runs.
    pub rh: Arc<Mutex<Option<RHShmemReader>>>,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl AsmShmemReaders {
    fn new(
        shm_prefix: &str,
        unlock_mapped_memory: bool,
        gpu_buffer_src: GpuBufferSource,
    ) -> ExecutorResult<Self> {
        Ok(Self {
            mt: Arc::new(Mutex::new(
                MTShmemReader::new(shm_prefix, unlock_mapped_memory)
                    .map_err(ExecutorError::asm_backend)?,
            )),
            mo: Arc::new(Mutex::new(
                MOShmemReader::new(shm_prefix, unlock_mapped_memory, gpu_buffer_src)
                    .map_err(ExecutorError::asm_backend)?,
            )),
            rh: Arc::new(Mutex::new(None)),
        })
    }
}

/// The shmem mappings of one hints mode: built when a prover's first program in it is set up,
/// and shared via `Arc` by every program that prover sets up in that mode. A standalone client
/// builds its own.
pub struct AsmSharedResources {
    config: AsmResourcesConfig,

    /// Shared memory writer for inputs (shmem mapped once; semaphores bound
    /// per-program by `AsmResources::activate`).
    pub shmem_inputs: Arc<InputsShmemWriter>,

    /// Hints processing pipeline — `Some` only in the hints mode.
    /// The precompile shmem segments are created by the C binary only in hints mode;
    /// attempting to open them without hints causes an immediate crash.
    hints_stream: Option<Arc<Mutex<ZiskStream<HintsProcessor<HintsShmem>>>>>,

    /// Pipeline for receiving inputs over a stream transport (QUIC, Unix socket).
    inputs_stream: Arc<Mutex<ZiskStream<InputsShmemWriter>>>,

    /// Readers for output shmem segments.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub readers: AsmShmemReaders,

    /// The segments mapped here, kept alive for as long as they are: unlinked
    /// only once nothing maps them. Last, so dropped after the mappings.
    _segments: Arc<SharedSegments>,
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
    /// Map all of `segments`. Semaphores are NOT opened here — call
    /// `AsmResources::activate` before use.
    ///
    /// `with_hints` must match the value used during setup: the C binary only creates the
    /// per-service precompile shmem segments when hints are enabled, so passing `true` here without
    /// the C binary having created them makes the underlying `shm_open` fail and returns an `Err`
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        local_rank: i32,
        unlock_mapped_memory: bool,
        verbose_mode: proofman_common::VerboseMode,
        mpi_broadcast_fn: Option<MpiBroadcastFn>,
        init_rom: bool,
        with_hints: bool,
        segments: Arc<SharedSegments>,
        gpu_buffer_src: GpuBufferSource,
    ) -> ExecutorResult<Self> {
        let shm_prefix = segments.shm_prefix();

        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let readers = AsmShmemReaders::new(shm_prefix, unlock_mapped_memory, gpu_buffer_src)?;

        // Avoid "unused variable" warnings on non-x86_64/Linux targets where the readers aren't used.
        #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
        let _ = gpu_buffer_src;

        let control_writer = Arc::new(
            ControlShmem::new(shm_prefix, unlock_mapped_memory)
                .map_err(ExecutorError::asm_backend)?,
        );

        let config = AsmResourcesConfig { local_rank, unlock_mapped_memory };

        let shmem_inputs = Arc::new(
            InputsShmemWriter::new(shm_prefix, unlock_mapped_memory, control_writer.clone())
                .map_err(ExecutorError::asm_backend)?,
        );

        let inputs_stream = Arc::new(Mutex::new(ZiskStream::from_arc(Arc::clone(&shmem_inputs))));

        let hints_stream = if with_hints {
            let active_services =
                if init_rom { &AsmServices::SERVICES[..] } else { &AsmServices::SERVICES[..2] };

            let hints_shmem = Arc::new(
                HintsShmem::new(shm_prefix, unlock_mapped_memory, control_writer, active_services)
                    .map_err(ExecutorError::asm_backend)?,
            );

            let mut builder = HintsProcessor::builder(hints_shmem, Some(shmem_inputs.clone()))
                .enable_stats(verbose_mode != proofman_common::VerboseMode::Info);

            if let Some(broadcast_fn) = mpi_broadcast_fn {
                builder = builder.with_mpi_broadcast(move |data| broadcast_fn(data));
            }

            let hints_processor = builder.build().map_err(ExecutorError::asm_backend)?;
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
            shmem_inputs,
            _segments: segments,
        })
    }
}

/// One program's assembly resources: the mappings of the shared segments, which it
/// shares with every program its prover (or standalone client) set up in its hints
/// mode, and its own services and semaphores. A job runs on them through
/// [`ZiskExecutor::begin_asm_job`](crate::ZiskExecutor::begin_asm_job).
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
    /// Create one program's resources over `shared`. Binds no semaphores:
    /// [`Self::activate`] does.
    pub fn new(shared: Arc<AsmSharedResources>, asm_services: AsmServices) -> Self {
        Self { shared, asm_services }
    }

    /// Wait until no setup or job uses the shared segments, and hold them until the
    /// returned claim is dropped; see [`SegmentsClaim`].
    pub fn claim(&self) -> SegmentsClaim {
        self.asm_services.claim()
    }

    /// Make the shared segments, and the writers mapping them, serve this program,
    /// unless they already do: [`AsmServices::activate`], binding the writers to
    /// this program's semaphores. `claim` must be held throughout.
    pub fn activate(&self, claim: &SegmentsClaim) -> ExecutorResult<()> {
        let hints_processor = match &self.shared.hints_stream {
            Some(hints_stream) => {
                Some(hints_stream.lock_or_poison("hints_stream")?.get_processor())
            }
            None => None,
        };
        let sem_prefix = self.asm_services.sem_prefix();

        self.asm_services
            .activate(claim, || {
                self.shared.shmem_inputs.bind_semaphores(sem_prefix)?;
                if let Some(processor) = &hints_processor {
                    processor.hints_sink().bind_semaphores(sem_prefix)?;
                }
                Ok(())
            })
            .map_err(ExecutorError::asm_backend)
    }

    /// Convenience constructor for the standalone path: spawns the ASM
    /// services and maps shmem segments without MPI/distributed/caching
    /// plumbing. Single process (`world_rank` / `local_rank` = 0), no MPI
    /// broadcast, owns ROM init.
    pub fn new_standalone(
        elf_hash: String,
        asm_mt_path: &Path,
        with_hints: bool,
        verbose: proofman_common::VerboseMode,
        gpu: bool,
    ) -> ExecutorResult<Self> {
        let options = AsmRunnerOptions::new().with_local_rank(0);
        // The claim that started the services is kept until the program is active:
        // mapping initializes the shared control and input segments, which another
        // client's job in this process may be using.
        let (services, claim) = AsmServices::new(0, 0, elf_hash, asm_mt_path, with_hints, options)
            .map_err(ExecutorError::asm_backend)?;
        let gpu_buffer_src =
            if gpu { GpuBufferSource::SelfAllocated } else { GpuBufferSource::Cpu };
        let shared = Arc::new(AsmSharedResources::new(
            0,
            false,
            verbose,
            None,
            true,
            with_hints,
            Arc::clone(services.segments()),
            gpu_buffer_src,
        )?);
        let resources = Self::new(shared, services);
        resources.activate(&claim)?;
        Ok(resources)
    }

    /// Returns the concrete hints processor, or `Err` if not set up with hints.
    pub fn get_hints_processor(&self) -> ExecutorResult<Arc<HintsProcessor<HintsShmem>>> {
        self.shared
            .hints_stream
            .as_ref()
            .ok_or(ExecutorError::HintsNotConfigured)?
            .lock()
            .map(|g| g.get_processor())
            .map_err(|_| ExecutorError::mutex_poisoned("hints_stream"))
    }

    /// Update the active ASM services for this partition.
    ///
    /// Call once per partition start (not per stream reset). RH participates
    /// in flow control only when `is_first_process` — the rank that actually
    /// spawns `AsmRunnerRH` (gated by the same flag, passed as `has_rom_sm`
    /// to `EmulatorAsm::execute`). On other ranks the C RH child is alive but
    /// idle: it never enters `_wait_for_prec_avail`, so its
    /// `precompile_read_address` stays at 0 forever. Including it in
    /// `HintsShmem::submit`'s slowest-consumer wait would wedge the producer
    /// once a job's cumulative hint output exceeds the ~128 MB buffer.
    pub fn set_active_services(&self, is_first_process: bool) -> ExecutorResult<()> {
        let Some(hints_stream) = &self.shared.hints_stream else { return Ok(()) };
        let processor = hints_stream.lock_or_poison("hints_stream")?.get_processor();
        let services =
            if is_first_process { &AsmServices::SERVICES[..] } else { &AsmServices::SERVICES[..2] };
        processor.hints_sink().set_active_services(services).map_err(ExecutorError::asm_backend)
    }

    /// Whether the ROM histogram service gets everything it reads, so that its runner
    /// finishes once the minimal trace has: always without hints, and with hints only
    /// while it is one of their consumers (see [`Self::set_active_services`]). An unfed
    /// one never finishes, so its runner is not started.
    pub fn rom_histogram_is_fed(&self) -> bool {
        let Some(hints_stream) = &self.shared.hints_stream else { return true };
        // A poisoned stream cannot say: take the runner as starved rather than risk
        // joining it for ever.
        hints_stream.lock().is_ok_and(|stream| {
            stream.get_processor().hints_sink().is_active_consumer(AsmService::RH)
        })
    }

    /// Submit hint data directly to the shmem sink, bypassing the processing pipeline.
    ///
    /// Used in the gRPC streaming path where hints arrive pre-processed.
    pub fn submit_hint_direct(&self, data: &[u64]) -> ExecutorResult<()> {
        let processor = self.get_hints_processor()?;
        processor.hints_sink().submit(data).map_err(ExecutorError::asm_backend)
    }

    /// Starts the hints stream.
    pub fn start_stream(&self) -> ExecutorResult<()> {
        let Some(hints_stream) = &self.shared.hints_stream else {
            return Err(ExecutorError::HintsNotConfigured);
        };
        hints_stream
            .lock_or_poison("hints_stream")?
            .start_stream()
            .map_err(ExecutorError::asm_backend)
    }

    /// Sets the stream source for the hints stream.
    pub fn set_hints_stream_src(&self, stream: StreamSource) -> ExecutorResult<()> {
        let Some(hints_stream) = &self.shared.hints_stream else {
            return Err(ExecutorError::HintsNotConfigured);
        };
        hints_stream
            .lock_or_poison("hints_stream")?
            .set_stream_src(stream)
            .map_err(ExecutorError::asm_backend)
    }

    /// Checks if the hints stream is initialized.
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
    pub fn signal_cancellation(&self) -> ExecutorResult<()> {
        self.shared.shmem_inputs.signal_reset().map_err(ExecutorError::asm_backend)
    }

    /// Clears what [`signal_cancellation`](Self::signal_cancellation) leaves in the
    /// segments, so that a cancel which lands after a job's end cannot abort the next one.
    pub fn clear_cancellation(&self) -> ExecutorResult<()> {
        self.shared.shmem_inputs.clear_reset().map_err(ExecutorError::asm_backend)
    }

    /// Rewinds the hints and the input, the shared memory and the streams registered for
    /// them alike, preparing for the next execution.
    ///
    /// A poisoned stream is reset all the same: this runs as a job ends, where a panic
    /// would abort an unwinding thread.
    pub fn reset(&self) {
        if let Some(s) = &self.shared.hints_stream {
            s.lock().unwrap_or_else(std::sync::PoisonError::into_inner).reset();
        }
        // Resets `shmem_inputs`, which the stream writes through.
        self.shared.inputs_stream.lock().unwrap_or_else(std::sync::PoisonError::into_inner).reset();
    }

    /// Returns the configuration for the ASM resources.
    pub fn config(&self) -> &AsmResourcesConfig {
        &self.shared.config
    }

    /// Returns the ASM services associated with these resources.
    pub fn asm_services(&self) -> &AsmServices {
        &self.asm_services
    }

    /// Sets the stream source for the inputs stream.
    pub fn set_inputs_stream_src(&self, stream: StreamSource) -> ExecutorResult<()> {
        self.shared
            .inputs_stream
            .lock_or_poison("inputs_stream")?
            .set_stream_src(stream)
            .map_err(ExecutorError::asm_backend)?;
        Ok(())
    }

    /// Starts the inputs stream.
    pub fn start_inputs_stream(&self) -> ExecutorResult<()> {
        self.shared
            .inputs_stream
            .lock_or_poison("inputs_stream")?
            .start_stream()
            .map_err(ExecutorError::asm_backend)
    }

    /// Checks if the inputs stream is initialized.
    pub fn is_inputs_stream_initialized(&self) -> bool {
        self.shared.inputs_stream.lock().map(|s| s.is_initialized()).unwrap_or(false)
    }

    /// Writes input data to the inputs shared memory segment.
    pub fn write_input(&self, stdin: &ZiskStdin) -> ExecutorResult<()> {
        // Borrowed: `write_input` only reads, and inputs are tens of MB.
        stdin
            .with_data(|data| self.shared.shmem_inputs.write_input(data))
            .map_err(ExecutorError::asm_backend)
    }

    /// Sets the host randomness seed for the next execution, which the ASM services read from the
    /// control shared memory when they start it. Call it before `execute` spawns them, like
    /// `write_input`.
    pub fn write_random_seed(&self, seed: &RandomSeed) -> ExecutorResult<()> {
        self.shared
            .shmem_inputs
            .set_random_seed(&seed.to_le_words())
            .map_err(ExecutorError::asm_backend)
    }

    /// Appends raw input data to the inputs shared memory segment.
    pub fn append_raw_input(&self, bytes: &[u8]) -> ExecutorResult<()> {
        self.shared.shmem_inputs.append_input(bytes).map_err(ExecutorError::asm_backend)
    }

    /// Gets the current ASM shmem readers.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub fn readers(&self) -> &AsmShmemReaders {
        &self.shared.readers
    }
}
