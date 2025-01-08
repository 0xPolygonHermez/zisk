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

use zisk_core::{zisk_ops::ZiskOp, ZiskRom, ROM_ENTRY};

use proofman_common::{AirInstance, FromTrace, ProofCtx};

use zisk_pil::{MainAirValues, MainTrace, MainTraceRow};
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

    /// Returns the number of rows in the main trace excluding the continuation row.
    ///
    /// # Returns
    /// The number of rows used for main trace execution
    pub fn non_continuation_rows<F: PrimeField>() -> u64 {
        MainTrace::<F>::NUM_ROWS as u64 - 1
    }

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
        vec_traces: &[EmuTrace],
        main_instance: &mut MainInstance,
    ) {
        let mut main_trace = MainTrace::new();

        let ictx = &main_instance.ictx;
        let current_segment = ictx.plan.segment_id.unwrap();
        let num_rows = MainTrace::<F>::NUM_ROWS;

        let filled_rows = vec_traces[current_segment].steps.steps as usize;

        info!(
            "{}: ··· Creating Main segment #{} [{} / {} rows filled {:.2}%]",
            Self::MY_NAME,
            current_segment,
            filled_rows + 1,
            num_rows,
            (filled_rows + 1) as f64 / num_rows as f64 * 100.0
        );

        // Set Row 0 of the current segment
        let row0 = if current_segment == 0 {
            MainTraceRow::<F> {
                pc: F::from_canonical_u64(ROM_ENTRY),
                op: F::from_canonical_u8(ZiskOp::CopyB.code()),
                a_src_imm: F::one(),
                b_src_imm: F::one(),
                ..MainTraceRow::default()
            }
        } else {
            let mut emu =
                Emu::from_emu_trace_start(zisk_rom, &vec_traces[current_segment - 1].last_state);
            let mut mem_reads_index: usize =
                vec_traces[current_segment - 1].last_state.mem_reads_index;
            let row_previous = emu.step_slice_full_trace(
                &vec_traces[current_segment - 1].steps,
                &mut mem_reads_index,
            );

            MainTraceRow::<F> {
                set_pc: row_previous.set_pc,
                jmp_offset1: row_previous.jmp_offset1,
                jmp_offset2: if row_previous.flag == F::one() {
                    row_previous.jmp_offset1
                } else {
                    row_previous.jmp_offset2
                },
                a: row_previous.a,
                b: row_previous.c,
                c: row_previous.c,
                a_offset_imm0: row_previous.a[0],
                b_offset_imm0: row_previous.c[0],
                addr1: row_previous.c[0],
                a_imm1: row_previous.a[1],
                b_imm1: row_previous.c[1],
                op: F::from_canonical_u8(ZiskOp::CopyB.code()),
                pc: row_previous.pc,
                a_src_imm: F::one(),
                b_src_imm: F::one(),
                ..MainTraceRow::default()
            }
        };

        let mut emu = Emu::from_emu_trace_start(zisk_rom, &vec_traces[current_segment].start_state);

        main_trace.buffer[0] = row0;

        // Set Rows 1 to N of the current segment (N = maximum number of air rows)
        let emu_trace_step = &vec_traces[current_segment].steps;
        let mut mem_reads_index: usize = 0;

        // main_trace.buffer.iter_mut().skip(1).take(filled_rows).for_each(|value| {
        //     *value = emu.step_slice_full_trace(emu_trace_step, &mut mem_reads_index);
        // });

        const BATCH_SIZE: usize = 4096;
        let mut partial_buffer = MainTrace::with_capacity(BATCH_SIZE);

        // Calculate the number of full batches and the remaining steps
        let num_batches = filled_rows / BATCH_SIZE;
        let last_batch_steps = filled_rows % BATCH_SIZE;

        for batch_idx in 0..num_batches {
            // Accumulate rows for this batch buffer using iterators to avoid bounds checking
            partial_buffer.buffer.iter_mut().for_each(|value| {
                *value = emu.step_slice_full_trace(emu_trace_step, &mut mem_reads_index);
            });

            // Copy the accumulated batch to the main buffer
            let start_idx = batch_idx * BATCH_SIZE + 1;
            let end_idx = start_idx + BATCH_SIZE;
            main_trace.buffer[start_idx..end_idx].copy_from_slice(&partial_buffer.buffer);
        }

        if last_batch_steps > 0 {
            // Accumulate rows for this batch buffer using iterators to avoid bounds checking
            partial_buffer.buffer.iter_mut().take(last_batch_steps).for_each(|value| {
                *value = emu.step_slice_full_trace(emu_trace_step, &mut mem_reads_index);
            });

            // Copy the remaining steps to the main buffer
            let start_idx = num_batches * BATCH_SIZE + 1;
            let end_idx = start_idx + last_batch_steps;
            main_trace.buffer[start_idx..end_idx]
                .copy_from_slice(&partial_buffer.buffer[..last_batch_steps]);
        }

        let last_row = main_trace.buffer[filled_rows];
        // Fill the rest of the buffer with the last row
        for i in (filled_rows + 1)..num_rows {
            main_trace.buffer[i] = last_row;
        }

        let mut main_air_values = MainAirValues::<F>::new();
        main_air_values.main_last_segment = F::from_bool(current_segment == vec_traces.len() - 1);
        main_air_values.main_segment = F::from_canonical_usize(current_segment);

        let air_instance = AirInstance::new_from_trace(
            FromTrace::new(&mut main_trace).with_air_values(&mut main_air_values),
        );

        pctx.air_instance_repo.add_air_instance(air_instance, main_instance.ictx.global_idx);
    }
}
