//! [`EmulatorAsm`] — x86_64-only ASM-backed emulator.
//!
//! On non-x86_64 the sibling `stub` module exposes a stub struct of the
//! same name so [`crate::execution::ExecutionPhase`] stays
//! platform-agnostic; the stub panics with a clear message if anyone
//! actually tries to use it.

use std::sync::{Arc, Mutex};

use crate::bus::pub_outs_collector::PubOutsCollector;
use crate::error::{ExecutorError, ExecutorResult, MutexExt};
use crate::execution::output::{BackendArtifacts, ExecutionOutput};
use crate::{CountersChunkMetrics, MAX_NUM_STEPS};

use super::{AsmResources, AsmRunnerSupervisor, AsmTransport, MtChunkProcessor};
use asm_runner::{AsmRunnerMT, HintsShmem};
use fields::PrimeField64;
use precompiles_hints::HintsProcessor;
use zisk_common::{
    io::StreamSource, io::ZiskStdin, stats_begin, stats_end, AsmExecutionInfo, ChunkId, EmuTrace,
    ExecutorStatsHandle, StatsScope,
};
use zisk_core::ZiskRom;

/// ASM-backend emulator. Wraps an `AsmTransport` for resource access
/// and threads MT/MO/RH runner lifecycles through `AsmRunnerSupervisor`
/// during `execute`.
pub struct EmulatorAsm {
    /// Chunk size for processing.
    chunk_size: u64,

    /// Facade over the worker-supplied [`AsmResources`]. Owns the
    /// "may not be installed yet" state and exposes every per-resource
    /// operation as a thin forwarding method.
    transport: AsmTransport,

    /// ASM execution info captured from the most recent successful `execute` call.
    asm_execution_info: Mutex<Option<AsmExecutionInfo>>,
}

impl EmulatorAsm {
    /// Construct an emulator that will process traces in `chunk_size`-row
    /// segments. The transport starts uninstalled — call
    /// [`Self::set_asm_resources`] before [`Self::execute`].
    pub fn new(chunk_size: u64) -> Self {
        Self { chunk_size, transport: AsmTransport::new(), asm_execution_info: Mutex::new(None) }
    }

    /// Returns a clone of the [`AsmExecutionInfo`] captured by the
    /// most recent successful `execute` call, or `None` if no
    /// execution has completed yet.
    pub fn get_asm_execution_info(&self) -> ExecutorResult<Option<AsmExecutionInfo>> {
        Ok(self.asm_execution_info.lock_or_poison("asm_execution_info")?.clone())
    }

    /// Installs the worker-supplied [`AsmResources`] handle on the
    /// transport. Must be called before [`Self::execute`].
    pub fn set_asm_resources(&self, asm_resources: Arc<AsmResources>) -> ExecutorResult<()> {
        self.transport.set_asm_resources(asm_resources)
    }

    /// Reset the hints stream pipeline and the input shmem writer.
    pub fn reset(&self) -> ExecutorResult<()> {
        self.transport.reset()
    }

    /// Poke the ASM children so any thread blocked on shmem semaphores
    /// unwinds promptly.
    pub fn signal_cancellation(&self) -> ExecutorResult<()> {
        self.transport.signal_cancellation()
    }

    /// Returns the `HintsProcessor` installed on the transport.
    pub fn get_hints_processor(&self) -> ExecutorResult<Arc<HintsProcessor<HintsShmem>>> {
        self.transport.get_hints_processor()
    }

    /// Toggle which ASM services participate in the next execution.
    pub fn set_active_services(&self, is_first_process: bool) -> ExecutorResult<()> {
        self.transport.set_active_services(is_first_process)
    }

    /// Replace the hints stream source to swap in a fresh stream.
    pub fn set_hints_stream_src(&self, stream: StreamSource) -> ExecutorResult<()> {
        self.transport.set_hints_stream_src(stream)
    }

    /// Replace the inputs stream source to swap in a fresh stream.
    pub fn set_inputs_stream_src(&self, stream: StreamSource) -> ExecutorResult<()> {
        self.transport.set_inputs_stream_src(stream)
    }

    /// Submit a hint payload directly to the shmem sink, bypassing the
    /// `ZiskStream` pipeline.
    /// Used in the gRPC streaming path where hint ordering is handled externally by the
    /// coordinator before data arrives here.
    pub fn submit_hint_direct(&self, data: &[u64]) -> ExecutorResult<()> {
        self.transport.submit_hint_direct(data)
    }

    /// Append a raw byte chunk to the input shmem writer.
    ///
    /// Used in the gRPC streaming path where input data arrives in chunks. Unlike
    /// `write_input` (which writes the full stdin at once for local execution), this
    /// appends incrementally as chunks arrive over the wire.
    pub fn append_raw_input(&self, bytes: &[u8]) -> ExecutorResult<()> {
        self.transport.append_raw_input(bytes)
    }

    /// Computes minimal traces by processing the ZisK ROM with given public inputs.
    ///
    /// # Arguments
    /// * `stdin` - Shared mutable access to the ZiskStdin providing public inputs.
    /// * `pctx` - Proof context used during execution.
    /// * `stats` - Handle for collecting executor statistics.
    /// * `_caller_stats_id` - Identifier used to attribute collected statistics to the caller.
    ///
    /// # Returns
    /// An `ExecutionOutput` whose `backend` field is the `Asm` variant carrying
    /// the spawned MO + (optionally) RH join handles.
    #[allow(clippy::too_many_arguments)]
    pub fn execute<F: PrimeField64>(
        &self,
        zisk_rom: &ZiskRom,
        stdin: &ZiskStdin,
        has_rom_sm: bool,
        use_hints: bool,
        stats: &ExecutorStatsHandle,
        _caller_stats_scope: &StatsScope,
    ) -> ExecutorResult<ExecutionOutput> {
        let asm_resources = self.transport.resources()?;

        let has_hints_stream = asm_resources.is_hints_stream_initialized();
        if use_hints && has_hints_stream {
            asm_resources.start_stream()?;
        }

        if asm_resources.is_inputs_stream_initialized() {
            asm_resources.start_inputs_stream()?;
        }

        stats_begin!(stats, _caller_stats_scope, _exec_scope, "EXECUTE_WITH_ASSEMBLY", 0);

        stats_begin!(stats, &_exec_scope, _write_scope, "ASM_WRITE_INPUT", 0);

        asm_resources.write_input(stdin)?;

        stats_end!(stats, &_write_scope);

        // Spawn the MO + RH runner threads. RH only on the rank 0, the one that computes the ROM histogram.
        let supervisor =
            AsmRunnerSupervisor::spawn_on(&asm_resources, self.chunk_size, has_rom_sm, stats);

        let mt_result = self.run_mt_assembly::<F>(zisk_rom, stats);

        let output = match mt_result {
            Ok((min_traces, counters, pub_outs)) => {
                let steps = min_traces.iter().map(|trace| trace.steps).sum::<u64>();
                let (handle_mo, handle_rh) = supervisor.into_handles();
                Ok(ExecutionOutput {
                    min_traces,
                    counters,
                    pub_outs,
                    steps,
                    backend: BackendArtifacts::Asm { mo: Some(handle_mo), rh: handle_rh },
                })
            }
            Err(e) => {
                // MT already self-cleaned (signaled reset + joined its stdio
                // thread). Hand the lifecycle to the supervisor: it pokes the
                // ASM children via the cancellation closure, then joins MO/RH
                // so their detached threads release the shmem-reader locks
                // before the next job begins.
                supervisor.cleanup_after_mt_failure(|| asm_resources.signal_cancellation());
                Err(e)
            }
        };

        stats_end!(stats, &_exec_scope);

        output
    }

    fn run_mt_assembly<F: PrimeField64>(
        &self,
        zisk_rom: &ZiskRom,
        stats: &ExecutorStatsHandle,
    ) -> ExecutorResult<(Vec<EmuTrace>, CountersChunkMetrics, PubOutsCollector)> {
        stats_begin!(stats, 0, _mt_scope, "RUN_MT_ASSEMBLY", 0);

        let processor: MtChunkProcessor<F> = MtChunkProcessor::new();

        // Capture the parent scope ID so it can be copied into the closure.
        #[allow(unused_variables)]
        let mt_scope_id = _mt_scope.id();

        let scope_result: ExecutorResult<_> = rayon::in_place_scope(|scope| {
            let processor_ref = &processor;
            let on_chunk = |idx: usize, emu_trace: std::sync::Arc<EmuTrace>| {
                let chunk_id = ChunkId(idx);
                scope.spawn(move |_| {
                    processor_ref.process_chunk(chunk_id, &emu_trace, zisk_rom, stats, mt_scope_id);
                });
            };

            let asm_resources = self.transport.resources()?;
            let mt_shmem = &mut asm_resources.readers().mt.lock_or_poison("mt_shmem_reader")?;
            let asm_resources_for_failure = asm_resources.clone();

            let result = AsmRunnerMT::run_and_count(
                mt_shmem,
                MAX_NUM_STEPS,
                self.chunk_size,
                on_chunk,
                move || {
                    asm_resources_for_failure.signal_cancellation().map_err(anyhow::Error::from)
                },
                asm_resources.asm_services().clone(),
                stats.clone(),
            )
            .map_err(ExecutorError::asm_backend)?;

            Ok(result)
        });

        let (emu_traces, asm_execution_info) = scope_result?;

        self.asm_execution_info.lock_or_poison("asm_execution_info")?.replace(asm_execution_info);

        // Unwrap the Arc pointers now that all rayon tasks have completed.
        let emu_traces = emu_traces
            .into_iter()
            .map(|arc| {
                Arc::try_unwrap(arc).map_err(|_| ExecutorError::ArcStillReferenced {
                    what: "EmuTrace after rayon scope",
                })
            })
            .collect::<ExecutorResult<Vec<_>>>()?;

        let (counters, pub_outs) = processor.finalize()?;

        stats_end!(stats, &_mt_scope);
        Ok((emu_traces, counters, pub_outs))
    }
}
