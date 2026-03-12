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
    /// Optional baseline port to communicate with assembly microservices.
    pub base_port: Option<u16>,

    /// Local rank for distributed execution.
    pub local_rank: i32,

    /// Map unlocked flag.
    pub unlock_mapped_memory: bool,
}

impl std::fmt::Debug for AsmResourcesConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsmResources")
            .field("base_port", &self.base_port)
            .field("local_rank", &self.local_rank)
            .field("unlock_mapped_memory", &self.unlock_mapped_memory)
            .finish_non_exhaustive()
    }
}

/// Encapsulates assembly-related resources including shared memory and hints stream.
#[derive(Clone)]
pub struct AsmResources {
    /// Configuration for assembly resources.
    config: AsmResourcesConfig,

    /// Shared memory for writing inputs to the assembly microservices.
    pub inputs_shmem_writer: Arc<InputsShmemWriter>,

    /// Pipeline for handling precompile hints.
    hints_stream: Option<Arc<Mutex<ZiskStream<HintsProcessor<HintsShmem>>>>>,

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub mt_shmem_reader: Arc<Mutex<MTShMemReader>>,
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub mo_shmem_reader: Arc<Mutex<MOShMemReader>>,
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub rh_shmem_reader: Arc<Mutex<Option<RHShMemReader>>>,
}

impl std::fmt::Debug for AsmResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsmResources")
            .field("config", &self.config)
            .field("hints_stream", &self.hints_stream.is_some())
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
        init_rom: bool,
    ) -> Result<Self> {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let asm_shmem_mt = MTShMemReader::new(local_rank, base_port, unlock_mapped_memory)?;

        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let asm_shmem_mo = MOShMemReader::new(local_rank, base_port, unlock_mapped_memory)?;

        let control_writer =
            Arc::new(ControlShmem::new(base_port, local_rank, unlock_mapped_memory)?);

        let config = AsmResourcesConfig { base_port, local_rank, unlock_mapped_memory };

        let inputs_shmem_writer = Arc::new(InputsShmemWriter::new(
            base_port,
            local_rank,
            unlock_mapped_memory,
            control_writer.clone(),
        )?);

        let hints_stream = if with_hints {
            let active_services =
                if init_rom { &AsmServices::SERVICES[..] } else { &AsmServices::SERVICES[..2] };

            let hints_shmem = Arc::new(HintsShmem::new(
                base_port,
                local_rank,
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
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            mt_shmem_reader: Arc::new(Mutex::new(asm_shmem_mt)),
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            mo_shmem_reader: Arc::new(Mutex::new(asm_shmem_mo)),
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            rh_shmem_reader: Arc::new(Mutex::new(None)),
            inputs_shmem_writer,
        })
    }

    /// Returns the concrete hints processor for passing to `StreamOrderingActor`.
    pub fn get_hints_processor(&self) -> Option<Arc<HintsProcessor<HintsShmem>>> {
        self.hints_stream.as_ref().map(|stream| stream.lock().unwrap().get_processor())
    }

    /// Update the active ASM services for this partition.
    ///
    /// Call once per partition start (not per stream reset).
    /// `is_first_partition` controls whether the ROM histogram service (RH) is active.
    pub fn set_active_services(&self, is_first_partition: bool) -> Result<()> {
        if let Some(stream) = &self.hints_stream {
            let processor = stream.lock().unwrap().get_processor();
            let sink = processor.hints_sink();
            let services = if is_first_partition {
                &AsmServices::SERVICES[..]
            } else {
                &AsmServices::SERVICES[..2]
            };
            sink.set_active_services(services)?;
        }
        Ok(())
    }

    /// Submit hint data directly to the shmem sink, bypassing the processing pipeline.
    ///
    /// Used in the gRPC streaming path where hints arrive pre-processed.
    pub fn submit_hint_direct(&self, data: &[u64]) -> Result<()> {
        if let Some(stream) = &self.hints_stream {
            let processor = stream.lock().unwrap().get_processor();
            processor.hints_sink().submit(data)
        } else {
            Err(anyhow::anyhow!("Hints stream not configured"))
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
            hints_stream.lock().unwrap().set_hints_stream_src(stream)
        } else {
            Err(anyhow::anyhow!("Hints stream not initialized"))
        }
    }

    pub fn is_hints_stream_initialized(&self) -> bool {
        self.hints_stream.as_ref().map(|s| s.lock().unwrap().is_initialized()).unwrap_or(false)
    }

    pub fn reset(&self) {
        if let Some(hints_stream) = &self.hints_stream {
            hints_stream.lock().unwrap().reset();
        }
        self.inputs_shmem_writer.reset();
    }

    pub fn config(&self) -> &AsmResourcesConfig {
        &self.config
    }

    pub fn write_input(&self, stdin: &ZiskStdin) -> Result<()> {
        let inputs = stdin.read_raw_bytes();

        self.inputs_shmem_writer.write_input(&inputs)
    }
}
