//! The `MainSM` module implements the Main State Machine,
//! responsible for computing witness main state machine.
//!
//! Key components of this module include:
//! - The `MainSM` struct, which handles the main execution trace computation.
//! - The `MainInstance` struct, representing the execution context of a specific main trace
//!   segment.
//! - Methods for computing the witness and setting up trace rows.

use std::sync::Arc;

use log::info;
use p3_field::PrimeField;
use rayon::iter::{IndexedParallelIterator, ParallelIterator};
use sm_common::{BusDeviceMetrics, InstanceCtx};

use data_bus::OPERATION_BUS_ID;
use zisk_core::ZiskRom;

use proofman_common::{AirInstance, FromTrace, ProofCtx, SetupCtx};

use zisk_pil::{MainAirValues, MainTrace, MainTraceRow};
use ziskemu::{Emu, EmuRegTrace, EmuTrace};

use crate::MainCounter;

/// Represents an instance of the main state machine,
/// containing context for managing a specific segment of the main trace.
pub struct MainInstance {
    /// Instance Context
    pub ictx: InstanceCtx,

    pub is_last_segment: bool,
}

impl MainInstance {
    /// Creates a new `MainInstance`.
    ///
    /// # Arguments
    /// * `ictx` - The instance context for this main instance.
    ///
    /// # Returns
    /// A new `MainInstance`.
    pub fn new(ictx: InstanceCtx, is_last_segment: bool) -> Self {
        Self { ictx, is_last_segment }
    }
}

/// The `MainSM` struct represents the Main State Machine,
/// responsible for generating the main witness.
pub struct MainSM {}

impl MainSM {
    const MY_NAME: &'static str = "MainSM  ";
    const BATCH_SIZE: usize = 1 << 12; // 2^12 rows per batch

    /// Computes the main witness trace for a given segment based on the provided proof context,
    /// ROM, and emulation traces.
    ///
    /// # Arguments
    /// * `zisk_rom` - Reference to the Zisk ROM used for execution.
    /// * `min_traces` - A vector of the minimal traces, each segment has num_within minimal traces
    ///   inside.
    /// * `min_trace_size` - The size of the minimal traces.
    /// * `main_instance` - Reference to the `MainInstance` representing the current segment.
    ///
    /// The computed trace is added to the proof context's air instance repository.
    pub fn compute_witness<F: PrimeField>(
        zisk_rom: &ZiskRom,
        min_traces: &[EmuTrace],
        min_trace_size: u64,
        main_instance: &mut MainInstance,
    ) -> AirInstance<F> {
        // Create the main trace buffer
        let mut main_trace = MainTrace::new();

        let segment_id = main_instance.ictx.plan.segment_id.unwrap();

        // Determine the number of minimal traces per segment
        let num_within = MainTrace::<F>::NUM_ROWS / min_trace_size as usize;
        let num_rows = MainTrace::<F>::NUM_ROWS;

        // Determine trace slice for the current segment
        let start_idx = segment_id * num_within;
        let end_idx = (start_idx + num_within).min(min_traces.len());
        let segment_min_traces = &min_traces[start_idx..end_idx];

        // Calculate total filled rows
        let filled_rows: usize =
            segment_min_traces.iter().map(|min_trace| min_trace.steps.steps as usize).sum();

        info!(
            "{}: ··· Creating Main segment #{} [{} / {} rows filled {:.2}%]",
            Self::MY_NAME,
            segment_id,
            filled_rows,
            num_rows,
            filled_rows as f64 / num_rows as f64 * 100.0
        );

        let next_pcs = main_trace
            .par_iter_mut_chunks(num_within)
            .enumerate()
            .take(segment_min_traces.len())
            .map(|(chunk_id, chunk)| {
                Self::fill_partial_trace(zisk_rom, chunk, &segment_min_traces[chunk_id])
            })
            .collect::<Vec<u64>>();

        let next_pc = next_pcs.last().unwrap();

        // Pad remaining rows with the last valid row
        let last_row = main_trace.buffer[filled_rows - 1];
        main_trace.buffer[filled_rows..num_rows].fill(last_row);

        let mut reg_trace = EmuRegTrace {
            reg_steps: [0; 32],
            reg_prev_steps: [0; 3],
            store_reg_prev_value: 0,
            first_step_uses: [None; 32],
            first_value_uses: [None; 32],
        };

        // Determine the last row of the previous segment
        let prev_segment_last_c = if start_idx > 0 {
            let prev_trace = &min_traces[start_idx - 1];
            let mut emu = Emu::from_emu_trace_start(zisk_rom, &prev_trace.last_state);
            let mut mem_reads_index = prev_trace.last_state.mem_reads_index;
            emu.step_slice_full_trace(&prev_trace.steps, &mut mem_reads_index, &mut reg_trace).c
        } else {
            [F::zero(), F::zero()]
        };

        // Prepare main AIR values
        let mut air_values = MainAirValues::<F>::new();

        air_values.main_segment = F::from_canonical_usize(segment_id);
        air_values.main_last_segment = F::from_bool(main_instance.is_last_segment);
        air_values.segment_initial_pc = main_trace.buffer[0].pc;
        air_values.segment_next_pc = F::from_canonical_u64(*next_pc);
        air_values.segment_previous_c = prev_segment_last_c;
        air_values.segment_last_c = last_row.c;

        // Generate and add the AIR instance
        let from_trace = FromTrace::new(&mut main_trace).with_air_values(&mut air_values);
        AirInstance::new_from_trace(from_trace)
    }

    /// Fills a partial trace in the main trace buffer based on the minimal trace.
    /// This method processes the minimal trace in batches to improve performance.
    ///
    /// # Arguments
    /// * `zisk_rom` - Reference to the Zisk ROM used for execution.
    /// * `main_trace` - Reference to the main trace buffer to fill.
    /// * `min_trace` - Reference to the minimal trace to process.
    ///
    /// # Returns
    /// The next program counter value after processing the minimal trace.
    fn fill_partial_trace<F: PrimeField>(
        zisk_rom: &ZiskRom,
        main_trace: &mut [MainTraceRow<F>],
        min_trace: &EmuTrace,
    ) -> u64 {
        // Initialize the emulator with the start state of the emu trace
        let mut emu = Emu::from_emu_trace_start(zisk_rom, &min_trace.start_state);
        let mut mem_reads_index: usize = 0;

        // Total number of rows to fill from the emu trace
        let total_rows = min_trace.steps.steps as usize;

        let mut reg_trace = EmuRegTrace {
            reg_steps: [0; 32],
            reg_prev_steps: [0; 3],
            store_reg_prev_value: 0,
            first_step_uses: [None; 32],
            first_value_uses: [None; 32],
        };

        // Process rows in batches
        let mut batch_buffer = MainTrace::with_capacity(1 << 12);
        for batch_start in (0..total_rows).step_by(Self::BATCH_SIZE) {
            // Determine the size of the current batch
            let batch_size = (batch_start + Self::BATCH_SIZE).min(total_rows) - batch_start;

            // Fill the batch buffer
            batch_buffer.buffer.iter_mut().take(batch_size).for_each(|row| {
                *row = emu.step_slice_full_trace(
                    &min_trace.steps,
                    &mut mem_reads_index,
                    &mut reg_trace,
                );
            });

            // Copy the processed batch into the main trace buffer
            let batch_end = batch_start + batch_size;
            main_trace[batch_start..batch_end].copy_from_slice(&batch_buffer.buffer[..batch_size]);
        }

        emu.ctx.inst_ctx.pc
    }

    pub fn debug<F: PrimeField>(_pctx: &ProofCtx<F>, _sctx: &SetupCtx<F>) {
        // No debug information to display
    }

    pub fn build_counter() -> Box<dyn BusDeviceMetrics> {
        Box::new(MainCounter::new(OPERATION_BUS_ID))
    }

    pub fn debug<F: PrimeField>(_pctx: Arc<ProofCtx<F>>, _sctx: Arc<SetupCtx<F>>) {
        // No debug information to display
    }

    pub fn build_counter() -> Box<dyn BusDeviceMetrics> {
        Box::new(MainCounter::new(OPERATION_BUS_ID))
    }
}
