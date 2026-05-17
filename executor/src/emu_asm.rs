use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    pub_outs_collector::PubOutsCollector,
    {AsmResources, AsmRunnerSupervisor, AsmTransport, StaticDataBus},
    {BackendArtifacts, CountersChunkMetrics, StaticSMBundle, TraceOutput, MAX_NUM_STEPS},
};
use asm_runner::{AsmRunnerMT, HintsShmem};
use data_bus::DataBusTrait;
use fields::PrimeField64;
use precompiles_hints::HintsProcessor;
use proofman_common::ProofCtx;
use zisk_common::{
    io::StreamSource, io::ZiskStdin, stats_begin, stats_end, AsmExecutionInfo, ChunkId, EmuTrace,
    ExecutorStatsHandle, StatsScope,
};
use zisk_core::ZiskRom;
use ziskemu::ZiskEmulator;

use anyhow::Result;

pub struct EmulatorAsm {
    /// Chunk size for processing.
    chunk_size: u64,

    /// Facade over the worker-supplied [`AsmResources`]. Owns the
    /// "may not be installed yet" state and exposes every per-resource
    /// operation as a thin forwarding method.
    transport: AsmTransport,

    asm_execution_info: Mutex<Option<AsmExecutionInfo>>,
}

impl EmulatorAsm {
    #[allow(clippy::too_many_arguments)]
    pub fn new(chunk_size: u64) -> Self {
        Self {
            chunk_size,
            transport: AsmTransport::new(),
            asm_execution_info: Mutex::new(None),
        }
    }

    pub fn get_chunk_size(&self) -> u64 {
        self.chunk_size
    }

    pub fn get_asm_execution_info(&self) -> Result<Option<AsmExecutionInfo>> {
        Ok(self
            .asm_execution_info
            .lock()
            .map_err(|e| anyhow::anyhow!("asm_execution_info lock poisoned: {e}"))?
            .clone())
    }

    /// Borrows the inner [`AsmTransport`]. The worker / coordinator
    /// uses this when it wants to drive only the resource-facade
    /// surface (streams, hints, cancellation) without reaching for
    /// threading / MT-chunk APIs.
    pub fn transport(&self) -> &AsmTransport {
        &self.transport
    }

    pub fn set_asm_resources(&self, asm_resources: Arc<AsmResources>) -> Result<()> {
        self.transport.set_asm_resources(asm_resources)
    }

    /// Resets the hints stream pipeline and the input shmem writer for the next job.
    pub fn reset(&self) -> Result<()> {
        self.transport.reset()
    }

    /// Poke the ASM children: set `ResetFlag` and post the wait semaphores
    /// so any child currently blocked in `_wait_for_input_avail` /
    /// `_wait_for_prec_avail` aborts and unwinds `emulator_start`. Used at
    /// cancel time so a stuck `execute` returns Err promptly.
    pub fn signal_cancellation(&self) -> Result<()> {
        self.transport.signal_cancellation()
    }

    pub fn get_hints_processor(&self) -> Result<Arc<HintsProcessor<HintsShmem>>> {
        self.transport.get_hints_processor()
    }

    pub fn set_active_services(&self, is_first_partition: bool) -> Result<()> {
        self.transport.set_active_services(is_first_partition)
    }

    pub fn set_hints_stream_src(&self, stream: StreamSource) -> Result<()> {
        self.transport.set_hints_stream_src(stream)
    }

    pub fn set_inputs_stream_src(&self, stream: StreamSource) -> Result<()> {
        self.transport.set_inputs_stream_src(stream)
    }

    /// Submits hint data directly to the shmem sink, bypassing the `ZiskStream` pipeline.
    ///
    /// Used in the gRPC streaming path where hint ordering is handled externally by the
    /// coordinator before data arrives here.
    pub fn submit_hint_direct(&self, data: &[u64]) -> Result<()> {
        self.transport.submit_hint_direct(data)
    }

    /// Appends a raw byte chunk to the input shmem writer.
    ///
    /// Used in the gRPC streaming path where input data arrives in chunks. Unlike
    /// `write_input` (which writes the full stdin at once for local execution), this
    /// appends incrementally as chunks arrive over the wire.
    pub fn append_raw_input(&self, bytes: &[u8]) -> Result<()> {
        self.transport.append_raw_input(bytes)
    }

    /// Computes minimal traces by processing the ZisK ROM with given public inputs.
    ///
    /// # Arguments
    /// * `stdin` - Shared mutable access to the ZiskStdin providing public inputs.
    /// * `pctx` - Proof context used during execution.
    /// * `sm_bundle` - Static shared-memory bundle used by the executor.
    /// * `stats` - Handle for collecting executor statistics.
    /// * `_caller_stats_id` - Identifier used to attribute collected statistics to the caller.
    ///
    /// # Returns
    /// A [`TraceOutput`] whose `backend` field is the `Asm` variant carrying
    /// the spawned MO + (optionally) RH join handles.
    #[allow(clippy::too_many_arguments)]
    pub fn execute<F: PrimeField64>(
        &self,
        zisk_rom: &ZiskRom,
        stdin: &ZiskStdin,
        pctx: &ProofCtx<F>,
        sm_bundle: &StaticSMBundle<F>,
        use_hints: bool,
        stats: &ExecutorStatsHandle,
        _caller_stats_scope: &StatsScope,
    ) -> Result<TraceOutput> {
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

        // Spawn the MO + (optionally) RH runner threads. RH only on the
        // first rank — that's the one that computes the ROM histogram.
        let has_rom_sm = pctx.dctx_is_first_process();
        let supervisor =
            AsmRunnerSupervisor::spawn_on(&asm_resources, self.chunk_size, has_rom_sm, stats);

        let mt_result = self.run_mt_assembly(zisk_rom, sm_bundle, stats);

        match mt_result {
            Ok((min_traces, counters, pub_outs)) => {
                let steps = min_traces.iter().map(|trace| trace.steps).sum::<u64>();
                stats_end!(stats, &_exec_scope);
                let (handle_mo, handle_rh) = supervisor.into_handles();
                Ok(TraceOutput {
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
                //
                // We enter this arm for ANY MT failure — cancellation, shmem
                // corruption, poisoned mutex, ASM child non-zero exit — so any
                // join-failure log inside `cleanup_after_mt_failure` deliberately
                // says "MT-failure cleanup" rather than "after cancel" to avoid
                // misattributing root cause.
                supervisor.cleanup_after_mt_failure(|| asm_resources.signal_cancellation());
                stats_end!(stats, &_exec_scope);
                Err(e)
            }
        }
    }

    fn run_mt_assembly<F: PrimeField64>(
        &self,
        zisk_rom: &ZiskRom,
        sm_bundle: &StaticSMBundle<F>,
        stats: &ExecutorStatsHandle,
    ) -> Result<(Vec<EmuTrace>, CountersChunkMetrics, PubOutsCollector)> {
        stats_begin!(stats, 0, _mt_scope, "RUN_MT_ASSEMBLY", 0);

        let results_mu: Mutex<Vec<(ChunkId, _)>> = Mutex::new(Vec::new());
        let errors: Mutex<Vec<anyhow::Error>> = Mutex::new(Vec::new());

        // Capture the parent scope ID so it can be copied into the closure
        #[allow(unused_variables)]
        let mt_scope_id = _mt_scope.id();

        let scope_result: Result<_> = rayon::in_place_scope(|scope| {
            let on_chunk = |idx: usize, emu_trace: std::sync::Arc<EmuTrace>| {
                let chunk_id = ChunkId(idx);
                let results_ref = &results_mu;
                let errors_ref = &errors;
                scope.spawn(move |_| {
                    stats_begin!(stats, mt_scope_id, _chunk_scope, "MT_CHUNK_PLAYER", 0);

                    let mut data_bus = match StaticDataBus::from_bundle(sm_bundle, true) {
                        Ok(db) => db,
                        Err(e) => {
                            let _ = errors_ref.lock().map(|mut errs| {
                                errs.push(anyhow::anyhow!(
                                    "StaticDataBus::from_bundle failed for chunk {}: {e}",
                                    chunk_id.0
                                ));
                            });
                            return;
                        }
                    };

                    ZiskEmulator::process_emu_trace::<F, _, _>(
                        zisk_rom,
                        &emu_trace,
                        &mut data_bus,
                        false,
                    );

                    data_bus.on_close();

                    stats_end!(stats, &_chunk_scope);

                    match results_ref.lock() {
                        Ok(mut guard) => guard.push((chunk_id, data_bus)),
                        Err(e) => {
                            let _ = errors_ref.lock().map(|mut errs| {
                                errs.push(anyhow::anyhow!(
                                    "results_mu lock poisoned for chunk {}: {e}",
                                    chunk_id.0
                                ));
                            });
                        }
                    }
                });
            };

            let asm_resources = self.transport.resources()?;
            let mt_shmem = &mut asm_resources
                .readers()
                .mt
                .lock()
                .map_err(|e| anyhow::anyhow!("mt_shmem_reader lock poisoned: {e}"))?;
            let asm_resources_for_failure = asm_resources.clone();
            let result = AsmRunnerMT::run_and_count(
                mt_shmem,
                MAX_NUM_STEPS,
                self.chunk_size,
                on_chunk,
                move || asm_resources_for_failure.signal_cancellation(),
                asm_resources.asm_services().clone(),
                stats.clone(),
            )?;
            Ok(result)
        });

        let (emu_traces, asm_execution_info) = scope_result?;

        // Check for errors collected during parallel execution
        let err_vec =
            errors.into_inner().map_err(|e| anyhow::anyhow!("errors mutex poisoned: {e}"))?;
        if !err_vec.is_empty() {
            let combined = err_vec
                .iter()
                .enumerate()
                .map(|(i, e)| format!("[Error {}] {:#}", i + 1, e))
                .collect::<Vec<_>>()
                .join("\n");
            return Err(anyhow::anyhow!(
                "MT assembly chunk processing failed ({} errors):\n{}",
                err_vec.len(),
                combined
            ));
        }

        self.asm_execution_info
            .lock()
            .map_err(|e| anyhow::anyhow!("asm_execution_info lock poisoned: {e}"))?
            .replace(asm_execution_info);

        // Unwrap the Arc pointers now that all rayon tasks have completed
        let emu_traces = emu_traces
            .into_iter()
            .map(|arc| {
                Arc::try_unwrap(arc)
                    .map_err(|_| anyhow::anyhow!("Arc still has multiple owners after scope"))
            })
            .collect::<Result<Vec<_>>>()?;

        let mut data_buses = results_mu
            .into_inner()
            .map_err(|e| anyhow::anyhow!("results_mu lock poisoned: {e}"))?;

        data_buses.sort_by_key(|(chunk_id, _)| chunk_id.0);

        let mut counters = HashMap::new();
        let mut pub_outs = PubOutsCollector::new();

        for (chunk_id, mut data_bus) in data_buses {
            pub_outs.0.extend(data_bus.take_pub_outs().0);
            let databus_counters = data_bus.into_devices(false);

            for (idx, counter) in databus_counters.into_iter() {
                let idx = idx.ok_or_else(|| {
                    anyhow::anyhow!("unexpected unindexed counter for chunk {}", chunk_id.0)
                })?;
                let counter = counter.ok_or_else(|| {
                    anyhow::anyhow!("secondary counter is None for chunk {} idx {idx}", chunk_id.0)
                })?;
                counters.entry(idx).or_insert_with(Vec::new).push((chunk_id, counter));
            }
        }

        stats_end!(stats, &_mt_scope);
        Ok((emu_traces, counters, pub_outs))
    }
}

impl<F: PrimeField64> crate::Emulator<F> for EmulatorAsm {
    fn execute(
        &self,
        zisk_rom: &ZiskRom,
        stdin: &ZiskStdin,
        pctx: &ProofCtx<F>,
        sm_bundle: &StaticSMBundle<F>,
        use_hints: bool,
        stats: &ExecutorStatsHandle,
        caller_stats_scope: &StatsScope,
    ) -> Result<crate::TraceOutput> {
        self.execute(zisk_rom, stdin, pctx, sm_bundle, use_hints, stats, caller_stats_scope)
    }
}
