use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use asm_runner::AsmRunnerMO;
use data_bus::DataBusTrait;
use fields::PrimeField64;
use proofman_common::ProofCtx;
use proofman_util::{timer_start_info, timer_stop_and_log_info};
use rayon::prelude::*;
use zisk_common::{
    io::{ZiskIO, ZiskStdin},
    ChunkId, EmuTrace, ExecutorStatsHandle,
};
use zisk_core::ZiskRom;
use ziskemu::{EmuOptions, ZiskEmulator};

use crate::{
    DeviceMetricsList, DummyCounter, NestedDeviceMetricsList, StaticSMBundle, MAX_NUM_STEPS,
};

pub struct EmulatorRust {
    /// ZisK ROM, a binary file containing the ZisK program to be executed.
    pub zisk_rom: Arc<ZiskRom>,

    /// Chunk size for processing.
    chunk_size: u64,
}

impl EmulatorRust {
    /// The number of threads to use for parallel processing when computing minimal traces.
    const NUM_THREADS: usize = 16;

    pub fn new(zisk_rom: Arc<ZiskRom>, chunk_size: u64) -> Self {
        Self { zisk_rom, chunk_size }
    }

    /// Computes minimal traces by processing the ZisK ROM with the given public inputs.
    ///
    /// # Arguments
    /// * `stdin` - Shared standard input source used to feed data into the emulator.
    /// * `_pctx` - Proof context carrying field-parameterized configuration for execution.
    /// * `sm_bundle` - Static state machine bundle used for counting device metrics.
    /// * `_stats` - Handle to executor statistics collection.
    /// * `_caller_stats_scope` - Stats scope used to associate collected statistics with the caller.
    ///
    /// # Returns
    /// A tuple containing:
    /// * `Vec<EmuTrace>` - The minimal traces produced by the emulator.
    /// * `DeviceMetricsList` - Metrics for primary devices.
    /// * `NestedDeviceMetricsList` - Metrics for secondary/nested devices.
    /// * `None`.
    /// * `u64` - Total number of steps.
    pub fn execute<F: PrimeField64>(
        &self,
        stdin: &Mutex<ZiskStdin>,
        sm_bundle: &StaticSMBundle<F>,
    ) -> (
        Vec<EmuTrace>,
        DeviceMetricsList,
        NestedDeviceMetricsList,
        Option<JoinHandle<AsmRunnerMO>>,
        u64,
    ) {
        let min_traces = self.run_emulator(Self::NUM_THREADS, &mut stdin.lock().unwrap());

        // Store execute steps
        let steps = min_traces.iter().map(|trace| trace.steps).sum::<u64>();

        timer_start_info!(COUNT);
        let (main_count, secn_count) = self.count(&min_traces, sm_bundle);
        timer_stop_and_log_info!(COUNT);

        (min_traces, main_count, secn_count, None, steps)
    }

    fn run_emulator(&self, num_threads: usize, stdin: &mut ZiskStdin) -> Vec<EmuTrace> {
        // Call emulate with these options
        let input_data = stdin.read_bytes();

        // Settings for the emulator
        let emu_options = EmuOptions {
            chunk_size: Some(self.chunk_size),
            max_steps: MAX_NUM_STEPS,
            ..EmuOptions::default()
        };

        ZiskEmulator::compute_minimal_traces(&self.zisk_rom, &input_data, &emu_options, num_threads)
            .expect("Error during emulator execution")
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
    fn count<F: PrimeField64>(
        &self,
        min_traces: &[EmuTrace],
        sm_bundle: &StaticSMBundle<F>,
    ) -> (DeviceMetricsList, NestedDeviceMetricsList) {
        let metrics_slices: Vec<_> = min_traces
            .par_iter()
            .map(|minimal_trace| {
                let mut data_bus = sm_bundle.build_data_bus_counters();

                ZiskEmulator::process_emu_trace::<F, _, _>(
                    &self.zisk_rom,
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

impl<F: PrimeField64> crate::Emulator<F> for EmulatorRust {
    fn execute(
        &self,
        stdin: &Mutex<ZiskStdin>,
        _pctx: &ProofCtx<F>,
        sm_bundle: &StaticSMBundle<F>,
        _stats: &ExecutorStatsHandle,
        _caller_stats_scope: &zisk_common::StatsScope,
    ) -> (
        Vec<EmuTrace>,
        DeviceMetricsList,
        NestedDeviceMetricsList,
        Option<JoinHandle<AsmRunnerMO>>,
        u64,
    ) {
        self.execute(stdin, sm_bundle)
    }
}
