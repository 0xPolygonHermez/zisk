//! The `MainSM` module implements the Main State Machine,
//! responsible for computing witness main state machine.
//!
//! Key components of this module include:
//! - The `MainSM` struct, which handles the main execution trace computation.
//! - The `MainInstance` struct, representing the execution context of a specific main trace
//!   segment.
//! - Methods for computing the witness and setting up trace rows.

use log::info;
use p3_field::PrimeField;
use sm_common::InstanceCtx;

use zisk_core::ZiskRom;

use proofman_common::{AirInstance, FromTrace, ProofCtx};

use zisk_pil::{MainAirValues, MainTrace};
use ziskemu::{Emu, EmuTrace};

/// Represents an instance of the main state machine,
/// containing context for managing a specific segment of the main trace.
pub struct MainInstance {
    /// Instance Context
    ictx: InstanceCtx,
}

impl MainInstance {
    /// Creates a new `MainInstance`.
    ///
    /// # Arguments
    /// * `ictx` - The instance context for this main instance.
    ///
    /// # Returns
    /// A new `MainInstance`.
    pub fn new(ictx: InstanceCtx) -> Self {
        Self { ictx }
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
    /// * `pctx` - Shared proof context for managing instances and air values.
    /// * `zisk_rom` - Reference to the Zisk ROM used for execution.
    /// * `vec_traces` - A vector of the minimal traces, one for each segment.
    /// * `main_instance` - Reference to the `MainInstance` representing the current segment.
    ///
    /// The computed trace is added to the proof context's air instance repository.
    pub fn prove_main<F: PrimeField>(
        pctx: &ProofCtx<F>,
        zisk_rom: &ZiskRom,
        min_traces: &[EmuTrace],
        min_traces_size: u64,
        main_instance: &mut MainInstance,
    ) {
        // Initialize the main trace buffer
        let mut main_trace = MainTrace::new();

        // Extract segment information
        let current_segment = main_instance.ictx.plan.segment_id.unwrap();
        let num_rows = MainTrace::<F>::NUM_ROWS;
        let traces_per_segment = (num_rows as u64 / min_traces_size) as usize;

        // Determine trace slice for the current segment
        let start_idx = current_segment * traces_per_segment;
        let end_idx = (start_idx + traces_per_segment).min(min_traces.len());
        let segment_min_traces = &min_traces[start_idx..end_idx];

        // Calculate total filled rows
        let filled_rows: usize =
            segment_min_traces.iter().map(|min_trace| min_trace.steps.steps as usize).sum();

        info!(
            "{}: ··· Creating Main segment #{} [{} / {} rows filled {:.2}%]",
            Self::MY_NAME,
            current_segment,
            filled_rows,
            num_rows,
            filled_rows as f64 / num_rows as f64 * 100.0
        );

        // Compute witness for the current segment
        let mut row_idx = 0;
        let mut next_pc: u64 = 0;

        // Preallocate a shared batch buffer to avoid multiple reallocations
        let mut preallocated_batch_buffer = MainTrace::with_capacity(1 << 12);

        segment_min_traces.iter().for_each(|trace| {
            Self::fill_partial_trace(
                zisk_rom,
                &mut preallocated_batch_buffer,
                &mut main_trace,
                &mut row_idx,
                &mut next_pc,
                trace,
            );
        });

        // Pad remaining rows with the last valid row
        let last_row = main_trace.buffer[filled_rows - 1];
        main_trace.buffer[filled_rows..num_rows].fill(last_row);

        // Determine the last row of the previous segment
        let prev_segment_last_c = if start_idx > 0 {
            let prev_trace = &min_traces[start_idx - 1];
            let mut emu = Emu::from_emu_trace_start(zisk_rom, &prev_trace.last_state);
            let mut mem_reads_index = prev_trace.last_state.mem_reads_index;
            emu.step_slice_full_trace(&prev_trace.steps, &mut mem_reads_index).c
        } else {
            [F::zero(), F::zero()]
        };

        // Prepare main AIR values
        let is_last_segment = end_idx == min_traces.len();
        let mut air_values = MainAirValues::<F>::new();

        air_values.main_segment = F::from_canonical_usize(current_segment);
        air_values.main_last_segment = F::from_bool(is_last_segment);
        air_values.segment_initial_pc = main_trace.buffer[0].pc;
        air_values.segment_next_pc = F::from_canonical_u64(next_pc);
        air_values.segment_previous_c = prev_segment_last_c;
        air_values.segment_last_c = last_row.c;

        // Generate and add the AIR instance
        let air_instance = AirInstance::new_from_trace(
            FromTrace::new(&mut main_trace).with_air_values(&mut air_values),
        );
        pctx.air_instance_repo.add_air_instance(air_instance, Some(main_instance.ictx.global_idx));
    }

    fn fill_partial_trace<F: PrimeField>(
        zisk_rom: &ZiskRom,
        batch_buffer: &mut MainTrace<F>,
        main_trace: &mut MainTrace<F>,
        main_trace_idx: &mut usize,
        next_pc: &mut u64,
        min_trace: &EmuTrace,
    ) {
        // Initialize the emulator with the start state of the emu trace
        let mut emu = Emu::from_emu_trace_start(zisk_rom, &min_trace.start_state);
        let emu_trace_step = &min_trace.steps;
        let mut mem_reads_index: usize = 0;

        // Total number of rows to fill from the emu trace
        let total_rows = min_trace.steps.steps as usize;

        // Process rows in batches
        for batch_start in (0..total_rows).step_by(Self::BATCH_SIZE) {
            // Determine the size of the current batch
            let batch_size = (batch_start + Self::BATCH_SIZE).min(total_rows) - batch_start;

            // Fill the batch buffer
            batch_buffer.buffer.iter_mut().take(batch_size).for_each(|row| {
                *row = emu.step_slice_full_trace(emu_trace_step, &mut mem_reads_index);
            });

            // Copy the processed batch into the main trace buffer
            let trace_start = batch_start + *main_trace_idx;
            let trace_end = trace_start + batch_size;
            main_trace.buffer[trace_start..trace_end]
                .copy_from_slice(&batch_buffer.buffer[..batch_size]);
        }

        *main_trace_idx += total_rows;
        *next_pc = emu.ctx.inst_ctx.pc;
    }
}
