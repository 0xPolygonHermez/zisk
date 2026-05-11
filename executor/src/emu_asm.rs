use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
    thread::JoinHandle,
};

use crate::AsmResources;
use crate::{
    DeviceMetricsList, DummyCounter, NestedDeviceMetricsList, StaticSMBundle, MAX_NUM_STEPS,
};
use asm_runner::{AsmRunnerMO, AsmRunnerMT, AsmRunnerRH, HintsShmem};
use data_bus::DataBusTrait;
use fields::PrimeField64;
use precompiles_hints::HintsProcessor;
use proofman_common::ProofCtx;
use zisk_common::io::StreamSource;
use zisk_common::{
    io::ZiskStdin, stats_begin, stats_end, AsmExecutionInfo, ChunkId, EmuTrace,
    ExecutorStatsHandle, StatsScope,
};
use zisk_core::ZiskRom;
use ziskemu::ZiskEmulator;

use anyhow::Result;

pub struct EmulatorAsm {
    /// Chunk size for processing.
    chunk_size: u64,

    /// Assembly resources including shared memory and hints stream.
    asm_resources: RwLock<Option<Arc<AsmResources>>>,

    asm_execution_info: Mutex<Option<AsmExecutionInfo>>,
}

impl EmulatorAsm {
    #[allow(clippy::too_many_arguments)]
    pub fn new(chunk_size: u64) -> Self {
        Self { chunk_size, asm_resources: RwLock::new(None), asm_execution_info: Mutex::new(None) }
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

    pub fn set_asm_resources(&self, asm_resources: Arc<AsmResources>) -> Result<()> {
        *self
            .asm_resources
            .write()
            .map_err(|e| anyhow::anyhow!("asm_resources lock poisoned: {e}"))? =
            Some(asm_resources);
        Ok(())
    }

    /// Resets the hints stream pipeline and the input shmem writer for the next job.
    pub fn reset(&self) -> Result<()> {
        if let Some(resources) = self
            .asm_resources
            .read()
            .map_err(|e| anyhow::anyhow!("asm_resources lock poisoned: {e}"))?
            .as_ref()
        {
            resources.reset();
        }
        Ok(())
    }

    /// Poke the ASM children: set `ResetFlag` and post the wait semaphores
    /// so any child currently blocked in `_wait_for_input_avail` /
    /// `_wait_for_prec_avail` aborts and unwinds `emulator_start`. Used at
    /// cancel time so a stuck `execute` returns Err promptly.
    pub fn signal_children_reset(&self) -> Result<()> {
        if let Some(resources) = self
            .asm_resources
            .read()
            .map_err(|e| anyhow::anyhow!("asm_resources lock poisoned: {e}"))?
            .as_ref()
        {
            resources.signal_children_reset()?;
        }
        Ok(())
    }

    pub fn get_hints_processor(&self) -> Result<Arc<HintsProcessor<HintsShmem>>> {
        self.asm_resources
            .read()
            .map_err(|e| anyhow::anyhow!("asm_resources lock poisoned: {e}"))?
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("AsmResources not initialized"))?
            .get_hints_processor()
    }

    pub fn set_active_services(&self, is_first_partition: bool) -> Result<()> {
        if let Some(resources) = self
            .asm_resources
            .read()
            .map_err(|e| anyhow::anyhow!("asm_resources lock poisoned: {e}"))?
            .as_ref()
        {
            resources.set_active_services(is_first_partition)
        } else {
            Ok(())
        }
    }

    pub fn set_hints_stream_src(&self, stream: StreamSource) -> Result<()> {
        self.asm_resources
            .read()
            .map_err(|e| anyhow::anyhow!("asm_resources lock poisoned: {e}"))?
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("AsmResources not initialized"))?
            .set_hints_stream_src(stream)
    }

    pub fn set_inputs_stream_src(&self, stream: StreamSource) -> Result<()> {
        self.asm_resources
            .read()
            .map_err(|e| anyhow::anyhow!("asm_resources lock poisoned: {e}"))?
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("AsmResources not initialized"))?
            .set_inputs_stream_src(stream)
    }

    /// Submits hint data directly to the shmem sink, bypassing the `ZiskStream` pipeline.
    ///
    /// Used in the gRPC streaming path where hint ordering is handled externally by the
    /// coordinator before data arrives here.
    pub fn submit_hint_direct(&self, data: &[u64]) -> Result<()> {
        self.asm_resources
            .read()
            .map_err(|e| anyhow::anyhow!("asm_resources lock poisoned: {e}"))?
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("AsmResources not initialized"))?
            .submit_hint_direct(data)
    }

    /// Appends a raw byte chunk to the input shmem writer.
    ///
    /// Used in the gRPC streaming path where input data arrives in chunks. Unlike
    /// `write_input` (which writes the full stdin at once for local execution), this
    /// appends incrementally as chunks arrive over the wire.
    pub fn append_raw_input(&self, bytes: &[u8]) -> Result<()> {
        self.asm_resources
            .read()
            .map_err(|e| anyhow::anyhow!("asm_resources lock poisoned: {e}"))?
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("AsmResources not initialized"))?
            .append_raw_input(bytes)
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
    /// A tuple containing:
    /// * `Vec<EmuTrace>` - The computed minimal traces.
    /// * `DeviceMetricsList` - Flat device metrics collected during execution.
    /// * `NestedDeviceMetricsList` - Hierarchical device metrics collected during execution.
    /// * `Option<JoinHandle<AsmRunnerMO>>` - Optional join handle for the memory-only ASM runner.
    /// * `u64` - Total number of steps.
    #[allow(clippy::type_complexity)]
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
    ) -> Result<(
        Vec<EmuTrace>,
        DeviceMetricsList,
        NestedDeviceMetricsList,
        Option<JoinHandle<Result<AsmRunnerMO>>>,
        Option<JoinHandle<Result<AsmRunnerRH>>>,
        u64,
    )> {
        let asm_resources = self
            .asm_resources
            .read()
            .map_err(|e| anyhow::anyhow!("asm_resources lock poisoned: {e}"))?
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("AsmResources not initialized"))?
            .clone();

        self.execute_inner(
            zisk_rom,
            stdin,
            pctx,
            sm_bundle,
            use_hints,
            stats,
            _caller_stats_scope,
            &asm_resources,
        )
    }

    #[allow(clippy::type_complexity)]
    #[allow(clippy::too_many_arguments)]
    fn execute_inner<F: PrimeField64>(
        &self,
        zisk_rom: &ZiskRom,
        stdin: &ZiskStdin,
        pctx: &ProofCtx<F>,
        sm_bundle: &StaticSMBundle<F>,
        use_hints: bool,
        stats: &ExecutorStatsHandle,
        _caller_stats_scope: &StatsScope,
        asm_resources: &Arc<AsmResources>,
    ) -> Result<(
        Vec<EmuTrace>,
        DeviceMetricsList,
        NestedDeviceMetricsList,
        Option<JoinHandle<Result<AsmRunnerMO>>>,
        Option<JoinHandle<Result<AsmRunnerRH>>>,
        u64,
    )> {
        let has_hints_stream = asm_resources.is_hints_stream_initialized();
        if use_hints && has_hints_stream {
            asm_resources.start_stream()?;
        }

        if asm_resources.is_inputs_stream_initialized() {
            asm_resources.start_inputs_stream()?;
        }

        stats_begin!(stats, _caller_stats_scope, _exec_scope, "EXECUTE_WITH_ASSEMBLY", 0);

        stats_begin!(stats, &_exec_scope, _write_scope, "ASM_WRITE_INPUT", 0);

        let config = asm_resources.config();

        asm_resources.write_input(stdin)?;

        stats_end!(stats, &_write_scope);

        let chunk_size = self.chunk_size;

        let _stats = stats.clone();

        // Run the assembly Memory Operations (MO) runner thread
        let handle_mo = std::thread::spawn({
            let asm_shmem_mo = asm_resources.readers().mo.clone();
            let asm_services = asm_resources.asm_services().clone();
            move || -> Result<AsmRunnerMO> {
                let mut guard = asm_shmem_mo
                    .lock()
                    .map_err(|e| anyhow::anyhow!("MO shmem lock poisoned: {e}"))?;
                AsmRunnerMO::run(&mut guard, MAX_NUM_STEPS, chunk_size, asm_services, _stats)
            }
        });

        // Run the ROM histogram only on partition 0 as it is always computed by this partition
        let has_rom_sm = pctx.dctx_is_first_process();

        let _stats = stats.clone();

        let handle_rh = (has_rom_sm).then(|| {
            let asm_shmem_rh = asm_resources.readers().rh.clone();
            let asm_services = asm_resources.asm_services().clone();
            let unlock_mapped_memory = config.unlock_mapped_memory;
            std::thread::spawn(move || -> Result<AsmRunnerRH> {
                let mut guard = asm_shmem_rh
                    .lock()
                    .map_err(|e| anyhow::anyhow!("RH shmem lock poisoned: {e}"))?;

                AsmRunnerRH::run(
                    &mut guard,
                    MAX_NUM_STEPS,
                    asm_services,
                    unlock_mapped_memory,
                    _stats,
                )
            })
        });

        let mt_result = self.run_mt_assembly(zisk_rom, sm_bundle, stats);

        match mt_result {
            Ok((min_traces, main_count, secn_count)) => {
                let steps = min_traces.iter().map(|trace| trace.steps).sum::<u64>();
                stats_end!(stats, &_exec_scope);
                Ok((min_traces, main_count, secn_count, Some(handle_mo), handle_rh, steps))
            }
            Err(e) => {
                // MT already self-cleaned (signaled reset + joined its stdio
                // thread). Wake MO/RH in case they're still parked, then join
                // so their detached threads release the shmem-reader locks
                // before the next job begins.
                if let Err(reset_err) = asm_resources.signal_children_reset() {
                    tracing::error!("execute_inner: signal_children_reset failed: {reset_err:#}");
                }
                let _ = handle_mo.join();
                if let Some(h) = handle_rh {
                    let _ = h.join();
                }
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
    ) -> Result<(Vec<EmuTrace>, DeviceMetricsList, NestedDeviceMetricsList)> {
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

                    let mut data_bus = match sm_bundle.build_data_bus_counters(true) {
                        Ok(db) => db,
                        Err(e) => {
                            let _ = errors_ref.lock().map(|mut errs| {
                                errs.push(anyhow::anyhow!(
                                    "build_data_bus_counters failed for chunk {}: {e}",
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

            let asm_resources = self
                .asm_resources
                .read()
                .map_err(|e| anyhow::anyhow!("asm_resources lock poisoned: {e}"))?
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("AsmResources not initialized"))?
                .clone();
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
                move || asm_resources_for_failure.signal_children_reset(),
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

        let mut main_count = Vec::with_capacity(data_buses.len());
        let mut secn_count = HashMap::new();

        for (chunk_id, data_bus) in data_buses {
            let databus_counters = data_bus.into_devices(false);

            for (idx, counter) in databus_counters.into_iter() {
                match idx {
                    None => {
                        main_count.push((chunk_id, counter.unwrap_or(Box::new(DummyCounter {}))));
                    }
                    Some(idx) => {
                        let counter = counter.ok_or_else(|| {
                            anyhow::anyhow!(
                                "secondary counter is None for chunk {} idx {idx}",
                                chunk_id.0
                            )
                        })?;
                        secn_count.entry(idx).or_insert_with(Vec::new).push((chunk_id, counter));
                    }
                }
            }
        }

        stats_end!(stats, &_mt_scope);
        Ok((emu_traces, main_count, secn_count))
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
    ) -> Result<crate::EmulatorResult> {
        self.execute(zisk_rom, stdin, pctx, sm_bundle, use_hints, stats, caller_stats_scope)
    }
}
