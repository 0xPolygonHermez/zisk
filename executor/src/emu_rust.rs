use std::collections::HashMap;

use data_bus::DataBusTrait;
use fields::PrimeField64;
use proofman_common::ProofCtx;
use proofman_util::{timer_start_info, timer_stop_and_log_info};
use rayon::prelude::*;
use zisk_common::{io::ZiskStdin, ChunkId, EmuTrace, ExecutorStatsHandle};
use zisk_core::ZiskRom;
use ziskemu::{EmuOptions, ZiskEmulator};

use crate::{
    pub_outs_collector::PubOutsCollector, EmulatorResult, NestedDeviceMetricsList, StaticSMBundle,
    MAX_NUM_STEPS,
};

use anyhow::Result;

pub struct EmulatorRust {
    /// Chunk size for processing.
    chunk_size: u64,
}

impl EmulatorRust {
    /// The number of threads to use for parallel processing when computing minimal traces.
    const NUM_THREADS: usize = 16;

    pub fn new(chunk_size: u64) -> Self {
        Self { chunk_size }
    }

    pub fn get_chunk_size(&self) -> u64 {
        self.chunk_size
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
    /// * `NestedDeviceMetricsList` - Metrics for secondary/nested devices.
    /// * `None` - Placeholder for optional `AsmRunnerMO` join handle (not used in this implementation).
    /// * `None` - Placeholder for optional `AsmRunnerRH` join handle (not used in this implementation).
    /// * `u64` - Total number of steps.
    /// * `PubOutsCollector` - Collected public outputs from the emulator execution.
    #[allow(clippy::type_complexity)]
    pub fn execute<F: PrimeField64>(
        &self,
        zisk_rom: &ZiskRom,
        stdin: &ZiskStdin,
        sm_bundle: &StaticSMBundle<F>,
    ) -> Result<EmulatorResult> {
        let min_traces = self.run_emulator(zisk_rom, Self::NUM_THREADS, stdin)?;

        // Store execute steps
        let steps = min_traces.iter().map(|trace| trace.steps).sum::<u64>();

        timer_start_info!(COUNT);
        let (counters, pub_outs) = self.count(zisk_rom, &min_traces, sm_bundle)?;
        timer_stop_and_log_info!(COUNT);

        Ok((min_traces, counters, None, None, steps, pub_outs))
    }

    fn run_emulator(
        &self,
        zisk_rom: &ZiskRom,
        num_threads: usize,
        stdin: &ZiskStdin,
    ) -> Result<Vec<EmuTrace>> {
        // Call emulate with these options
        let input_data = stdin.read_data();

        // Settings for the emulator
        let emu_options = EmuOptions {
            chunk_size: Some(self.chunk_size),
            max_steps: MAX_NUM_STEPS,
            ..EmuOptions::default()
        };

        Ok(ZiskEmulator::compute_minimal_traces(zisk_rom, &input_data, &emu_options, num_threads)?)
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
        zisk_rom: &ZiskRom,
        min_traces: &[EmuTrace],
        sm_bundle: &StaticSMBundle<F>,
    ) -> Result<(NestedDeviceMetricsList, PubOutsCollector)> {
        let metrics_slices: Vec<_> = min_traces
            .par_iter()
            .map(|minimal_trace| {
                let mut data_bus = sm_bundle.build_data_bus_counters(false)?;

                ZiskEmulator::process_emu_trace::<F, _, _>(
                    zisk_rom,
                    minimal_trace,
                    &mut data_bus,
                    true,
                );

                let pub_outs_chunk = data_bus.take_pub_outs();
                let databus_counters = data_bus.into_devices(true);

                let mut counters = Vec::new();
                for counter in databus_counters.into_iter() {
                    counters.push(counter);
                }

                Ok((counters, pub_outs_chunk))
            })
            .collect::<Result<Vec<_>>>()?;

        let mut counters = HashMap::new();
        let mut pub_outs = PubOutsCollector::new();

        for (chunk_id, (counter_slice, pub_outs_chunk)) in metrics_slices.into_iter().enumerate() {
            pub_outs.0.extend(pub_outs_chunk.0);
            for (idx, counter) in counter_slice.into_iter() {
                let idx = idx.ok_or_else(|| {
                    anyhow::anyhow!("unexpected unindexed counter for chunk {chunk_id}")
                })?;
                counters.entry(idx).or_insert_with(Vec::new).push((
                    ChunkId(chunk_id),
                    counter.ok_or_else(|| {
                        anyhow::anyhow!("secondary counter is None for chunk {chunk_id}, idx {idx}")
                    })?,
                ));
            }
        }

        Ok((counters, pub_outs))
    }
}

impl<F: PrimeField64> crate::Emulator<F> for EmulatorRust {
    fn execute(
        &self,
        zisk_rom: &ZiskRom,
        stdin: &ZiskStdin,
        _pctx: &ProofCtx<F>,
        sm_bundle: &StaticSMBundle<F>,
        _use_hints: bool,
        _stats: &ExecutorStatsHandle,
        _caller_stats_scope: &zisk_common::StatsScope,
    ) -> Result<EmulatorResult> {
        self.execute(zisk_rom, stdin, sm_bundle)
    }
}
