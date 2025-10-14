use std::{collections::HashMap, fs, path::PathBuf};

use asm_runner::MinimalTraces;
use data_bus::DataBusTrait;
use fields::PrimeField64;
use proofman_common::ProofCtx;
use proofman_util::{timer_start_info, timer_stop_and_log_info};
use zisk_common::{ChunkId, ExecutorStatsHandle};
use zisk_core::ZiskRom;
use ziskemu::{EmuOptions, ZiskEmulator};

use crate::{
    DeviceMetricsList, DummyCounter, ExecutionResult, ExecutionResultEnum, ExecutorRunner,
    NestedDeviceMetricsList, StaticSMBundle,
};

use rayon::prelude::*;

#[derive(Debug)]
pub struct EmulatorRunner<F: PrimeField64> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField64> Default for EmulatorRunner<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: PrimeField64> EmulatorRunner<F> {
    /// The number of threads to use for parallel processing when computing minimal traces.
    const NUM_THREADS: usize = 16;

    pub fn new() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }

    /// Computes minimal traces by processing the ZisK ROM with given public inputs.
    ///
    /// # Arguments
    /// * `input_data` - Input data for the ROM execution.
    /// * `num_threads` - Number of threads to use for parallel execution.
    ///
    /// # Returns
    /// A vector of `EmuTrace` instances representing minimal traces.
    pub fn run(
        &mut self,
        zisk_rom: &ZiskRom,
        input_data_path: Option<PathBuf>,
        chunk_size: u64,
        sm_bundle: &StaticSMBundle<F>,
        _stats: &ExecutorStatsHandle,
        #[cfg(feature = "stats")] _caller_stats_id: u64,
    ) -> (MinimalTraces, DeviceMetricsList, NestedDeviceMetricsList, u64) {
        let min_traces = self.run_emulator(input_data_path, zisk_rom, chunk_size);

        // Store execute steps
        let steps = if let MinimalTraces::EmuTrace(min_traces) = &min_traces {
            min_traces.iter().map(|trace| trace.steps).sum::<u64>()
        } else {
            panic!("Expected EmuTrace, got something else");
        };

        timer_start_info!(COUNT);
        let (main_count, secn_count) = self.count(zisk_rom, &min_traces, sm_bundle);
        timer_stop_and_log_info!(COUNT);

        (min_traces, main_count, secn_count, steps)
    }

    fn run_emulator(
        &self,
        input_data_path: Option<PathBuf>,
        zisk_rom: &ZiskRom,
        chunk_size: u64,
    ) -> MinimalTraces {
        // Call emulate with these options
        let input_data = if let Some(path) = &input_data_path {
            // Read inputs data from the provided inputs path
            let path = PathBuf::from(path.display().to_string());
            fs::read(path).expect("Could not read inputs file")
        } else {
            Vec::new()
        };

        // Settings for the emulator
        let emu_options = EmuOptions {
            chunk_size: Some(chunk_size),
            max_steps: crate::MAX_NUM_STEPS,
            ..EmuOptions::default()
        };

        let min_traces = ZiskEmulator::compute_minimal_traces(
            zisk_rom,
            &input_data,
            &emu_options,
            Self::NUM_THREADS,
        )
        .expect("Error during emulator execution");

        MinimalTraces::EmuTrace(min_traces)
    }

    /// Counts metrics for secondary state machines based on minimal traces.
    ///
    /// # Arguments
    /// * `min_traces` - Minimal traces obtained from the ROM execution.
    ///
    /// # Returns
    /// A tuple containing two vectors:
    /// * A vector of main state machine metrics grouped by chunk ID.
    /// * A vector of secondary state machine metrics grouped by chunk ID. The vector is nested,
    ///   with the outer vector representing the secondary state machines and the inner vector
    ///   containing the metrics for each chunk.
    fn count(
        &self,
        zisk_rom: &ZiskRom,
        min_traces: &MinimalTraces,
        sm_bundle: &StaticSMBundle<F>,
    ) -> (DeviceMetricsList, NestedDeviceMetricsList) {
        let min_traces = match min_traces {
            MinimalTraces::EmuTrace(min_traces) => min_traces,
            MinimalTraces::AsmEmuTrace(asm_min_traces) => &asm_min_traces.vec_chunks,
            _ => unreachable!(),
        };

        let metrics_slices: Vec<_> = min_traces
            .par_iter()
            .map(|minimal_trace| {
                let mut data_bus = sm_bundle.build_data_bus_counters();

                ZiskEmulator::process_emu_trace::<F, _, _>(
                    zisk_rom,
                    minimal_trace,
                    &mut data_bus,
                    true,
                );

                let mut counters = Vec::new();

                let databus_counters = data_bus.into_devices(true);
                for counter in databus_counters.into_iter() {
                    counters.push(counter);
                }

                counters
            })
            .collect();

        let mut main_count = Vec::new();
        let mut secn_count = HashMap::new();

        for (chunk_id, counter_slice) in metrics_slices.into_iter().enumerate() {
            for (idx, counter) in counter_slice.into_iter() {
                match idx {
                    None => {
                        main_count.push((
                            ChunkId(chunk_id),
                            counter.unwrap_or_else(|| Box::new(DummyCounter {})),
                        ));
                    }
                    Some(idx) => {
                        secn_count
                            .entry(idx)
                            .or_insert_with(Vec::new)
                            .push((ChunkId(chunk_id), counter.unwrap()));
                    }
                }
            }
        }

        (main_count, secn_count)
    }
}

impl<F: PrimeField64> ExecutorRunner<F> for EmulatorRunner<F> {
    fn run(
        &mut self,
        _pctx: &ProofCtx<F>,
        zisk_rom: &ZiskRom,
        input_data_path: Option<PathBuf>,
        chunk_size: u64,
        sm_bundle: &StaticSMBundle<F>,
        stats: &ExecutorStatsHandle,
        #[cfg(feature = "stats")] _caller_stats_id: u64,
    ) -> ExecutionResultEnum {
        let (minimal_traces, main, secn, steps) = self.run(
            zisk_rom,
            input_data_path,
            chunk_size,
            sm_bundle,
            stats,
            #[cfg(feature = "stats")]
            _caller_stats_id,
        );

        ExecutionResultEnum::FinalResult(ExecutionResult { minimal_traces, main, secn, steps })
    }
}
