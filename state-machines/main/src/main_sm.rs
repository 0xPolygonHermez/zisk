//! The `MainSM` module implements the Main State Machine,
//! responsible for computing witness main state machine.
//!
//! Key components of this module include:
//! - The `MainSM` struct, which handles the main execution trace computation.
//! - The `MainInstance` struct, representing the execution context of a specific main trace
//!   segment.
//! - Methods for computing the witness and setting up trace rows.

use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use log::info;

use p3_field::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofCtx, SetupCtx};
use rayon::iter::{IndexedParallelIterator, ParallelIterator};
use sm_mem::{MemHelpers, MEMORY_MAX_DIFF, MEM_STEPS_BY_MAIN_STEP};
use zisk_common::{BusDeviceMetrics, EmuTrace, InstanceCtx};
use zisk_core::{ZiskRom, REGS_IN_MAIN, REGS_IN_MAIN_FROM, REGS_IN_MAIN_TO};
use zisk_pil::{MainAirValues, MainTrace, MainTraceRow};
use ziskemu::{Emu, EmuRegTrace};

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
    pub fn compute_witness<F: PrimeField64>(
        zisk_rom: &ZiskRom,
        min_traces: &[EmuTrace],
        min_trace_size: u64,
        main_instance: &mut MainInstance,
        std: Arc<Std<F>>,
    ) -> AirInstance<F> {
        // Create the main trace buffer
        let mut main_trace = MainTrace::new();

        let segment_id = main_instance.ictx.plan.segment_id.unwrap();

        // Determine the number of minimal traces per segment
        let num_within = MainTrace::<F>::NUM_ROWS / min_trace_size as usize;
        let num_rows = MainTrace::<F>::NUM_ROWS;

        // Determine trace slice for the current segment
        let start_idx = segment_id.as_usize() * num_within;
        let end_idx = (start_idx + num_within).min(min_traces.len());
        let segment_min_traces = &min_traces[start_idx..end_idx];

        // Calculate total filled rows
        let filled_rows: usize =
            segment_min_traces.iter().map(|min_trace| min_trace.steps as usize).sum();

        info!(
            "{}: ··· Creating Main segment #{} [{} / {} rows filled {:.2}%]",
            Self::MY_NAME,
            segment_id,
            filled_rows,
            num_rows,
            filled_rows as f64 / num_rows as f64 * 100.0
        );

        // Calculate final_step of instance, last mem slot of last row. The initial_step is 0 for the
        // first segment, for the rest is the final_step of the previous segment

        let last_row_previous_segment =
            if segment_id == 0 { 0 } else { (segment_id.as_usize() * num_rows) as u64 - 1 };

        let initial_step = MemHelpers::main_step_to_special_mem_step(last_row_previous_segment);

        let final_step = MemHelpers::main_step_to_special_mem_step(
            ((segment_id.as_usize() + 1) * num_rows) as u64 - 1,
        );

        // To reduce memory used, only take memory for the maximum range of mem_step inside the
        // minimal trace.
        let max_range = min_trace_size * MEM_STEPS_BY_MAIN_STEP;

        // Vector of atomics of u32, it's enough to count all range check values of the trace.
        let step_range_check =
            Arc::new((0..max_range).map(|_| AtomicU32::new(0)).collect::<Vec<_>>());

        // We know each register's previous step, but only by instance. We don't have this
        // information by chunk, so we need to store in the EmuRegTrace the location of the
        // first mem_step register is used in the chunk and information about the last step
        // where the register is used. The register's last steps of one chunk are the initial
        // steps of the next chunk. In the end, we need to update with the correct values.

        let fill_trace_outputs = main_trace
            .par_iter_mut_chunks(num_within)
            .enumerate()
            .take(segment_min_traces.len())
            .map(|(chunk_id, chunk)| {
                let init_chunk_step = if chunk_id == 0 { initial_step } else { 0 };
                let mut reg_trace = EmuRegTrace::from_init_step(init_chunk_step, chunk_id == 0);
                let (pc, regs) = Self::fill_partial_trace(
                    zisk_rom,
                    chunk,
                    &segment_min_traces[chunk_id],
                    &mut reg_trace,
                    step_range_check.clone(),
                    chunk_id == (end_idx - start_idx - 1),
                );
                (pc, regs, reg_trace)
            })
            .collect::<Vec<(u64, Vec<u64>, EmuRegTrace)>>();
        let last_result = fill_trace_outputs.last().unwrap();
        let next_pc = last_result.0;

        // In the range checks are values too large to store in steps_range_check, but there
        // are only a few values that exceed this limit, for this reason, are stored in a vector

        let mut reg_steps = [initial_step; REGS_IN_MAIN];
        let mut large_range_checks = Self::complete_trace_with_initial_reg_steps_per_chunk(
            num_rows,
            &fill_trace_outputs,
            &mut main_trace,
            step_range_check.clone(),
            &mut reg_steps,
        );

        Self::update_reg_steps_with_last_chunk(&last_result.2, &mut reg_steps);

        // Pad remaining rows with the last valid row
        // In padding row must be clear of registers access, if not need to calculate previous
        // register step and range check conntribution
        let last_row = main_trace.buffer[filled_rows - 1];
        main_trace.buffer[filled_rows..num_rows].fill(last_row);

        // Determine the last row of the previous segment
        let prev_segment_last_c = if start_idx > 0 {
            Emu::intermediate_value(min_traces[start_idx - 1].last_c)
        } else {
            [F::ZERO, F::ZERO]
        };

        // Prepare main AIR values
        let mut air_values = MainAirValues::<F>::new();

        air_values.main_segment = F::from_usize(segment_id.into());
        air_values.main_last_segment = F::from_bool(main_instance.is_last_segment);
        air_values.segment_initial_pc = main_trace.buffer[0].pc;
        air_values.segment_next_pc = F::from_u64(next_pc);
        air_values.segment_previous_c = prev_segment_last_c;
        air_values.segment_last_c = last_row.c;

        Self::update_reg_airvalues(
            &mut air_values,
            final_step,
            &last_result.1,
            &reg_steps,
            step_range_check.clone(),
            &mut large_range_checks,
        );
        Self::update_std_range_checks(std, step_range_check, &large_range_checks);

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
    fn fill_partial_trace<F: PrimeField64>(
        zisk_rom: &ZiskRom,
        main_trace: &mut [MainTraceRow<F>],
        min_trace: &EmuTrace,
        reg_trace: &mut EmuRegTrace,
        step_range_check: Arc<Vec<AtomicU32>>,
        last_reg_values: bool,
    ) -> (u64, Vec<u64>) {
        // Initialize the emulator with the start state of the emu trace
        let mut emu = Emu::from_emu_trace_start(zisk_rom, &min_trace.start_state);
        let mut mem_reads_index: usize = 0;

        // Total number of rows to fill from the emu trace
        let total_rows = min_trace.steps as usize;

        const BATCH_SIZE: usize = 1 << 12; // 2^12 rows per batch

        // Process rows in batches
        let mut batch_buffer = MainTrace::with_capacity(BATCH_SIZE);

        for batch_start in (0..total_rows).step_by(BATCH_SIZE) {
            let batch_end = (batch_start + BATCH_SIZE).min(total_rows);
            let batch_size = batch_end - batch_start;

            // Process the batch
            for i in 0..batch_size {
                batch_buffer.buffer[i] = emu.step_slice_full_trace(
                    &min_trace.mem_reads,
                    &mut mem_reads_index,
                    reg_trace,
                    Some(&**step_range_check),
                );
            }

            // Copy processed batch into main trace buffer
            main_trace[batch_start..batch_end].copy_from_slice(&batch_buffer.buffer[..batch_size]);
        }

        (
            emu.ctx.inst_ctx.pc,
            if last_reg_values {
                emu.ctx.inst_ctx.regs[REGS_IN_MAIN_FROM..=REGS_IN_MAIN_TO].to_vec()
            } else {
                vec![]
            },
        )
    }

    fn complete_trace_with_initial_reg_steps_per_chunk<F: PrimeField64>(
        num_rows: usize,
        fill_trace_outputs: &[(u64, Vec<u64>, EmuRegTrace)],
        main_trace: &mut MainTrace<F>,
        step_range_check: Arc<Vec<AtomicU32>>,
        reg_steps: &mut [u64; REGS_IN_MAIN],
    ) -> Vec<u32> {
        let mut large_range_checks: Vec<u32> = vec![];
        let max_range = step_range_check.len() as u64;
        for (index, (_, _, reg_trace)) in fill_trace_outputs.iter().enumerate().skip(1) {
            #[allow(clippy::needless_range_loop)]
            for reg_index in 0..REGS_IN_MAIN {
                let reg_prev_mem_step = if fill_trace_outputs[index - 1].2.reg_steps[reg_index] == 0
                {
                    reg_steps[reg_index]
                } else {
                    fill_trace_outputs[index - 1].2.reg_steps[reg_index]
                };
                reg_steps[reg_index] = reg_prev_mem_step;
                if reg_trace.first_step_uses[reg_index].is_some() {
                    let mem_step = reg_trace.first_step_uses[reg_index].unwrap();
                    let slot = MemHelpers::mem_step_to_slot(mem_step);
                    let row = MemHelpers::mem_step_to_row(mem_step) % num_rows;
                    let range = mem_step - reg_prev_mem_step;
                    if range > max_range {
                        large_range_checks.push(range as u32);
                    } else {
                        step_range_check[(range - 1) as usize].fetch_add(1, Ordering::Relaxed);
                    }
                    match slot {
                        0 => {
                            main_trace.buffer[row].a_reg_prev_mem_step =
                                F::from_u64(reg_prev_mem_step)
                        }
                        1 => {
                            main_trace.buffer[row].b_reg_prev_mem_step =
                                F::from_u64(reg_prev_mem_step)
                        }
                        2 => {
                            main_trace.buffer[row].store_reg_prev_mem_step =
                                F::from_u64(reg_prev_mem_step)
                        }
                        _ => panic!("Invalid slot {}", slot),
                    }
                    // TODO: range_check mem_step - reg_prev_mem_step
                }
            }
        }
        large_range_checks
    }
    fn update_reg_steps_with_last_chunk(
        last_emu_reg_trace: &EmuRegTrace,
        reg_steps: &mut [u64; REGS_IN_MAIN],
    ) {
        #[allow(clippy::needless_range_loop)]
        for reg_index in 0..REGS_IN_MAIN {
            let reg_prev_mem_step = if last_emu_reg_trace.reg_steps[reg_index] == 0 {
                reg_steps[reg_index]
            } else {
                last_emu_reg_trace.reg_steps[reg_index]
            };
            reg_steps[reg_index] = reg_prev_mem_step;
        }
    }
    fn update_reg_airvalues<F: PrimeField64>(
        air_values: &mut MainAirValues<'_, F>,
        final_step: u64,
        last_reg_values: &[u64],
        reg_steps: &[u64; REGS_IN_MAIN],
        step_range_check: Arc<Vec<AtomicU32>>,
        large_range_checks: &mut Vec<u32>,
    ) {
        let max_range = step_range_check.len() as u64;
        for ireg in 0..REGS_IN_MAIN {
            let reg_value = last_reg_values[ireg];
            let values = [F::from_u32(reg_value as u32), F::from_u32((reg_value >> 32) as u32)];
            air_values.last_reg_value[ireg] = values;
            air_values.last_reg_mem_step[ireg] = F::from_u64(reg_steps[ireg]);
            let range = (final_step - reg_steps[ireg]) as usize;
            if range > max_range as usize {
                large_range_checks.push(range as u32);
            } else {
                step_range_check[range - 1].fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    fn update_std_range_checks<F: PrimeField64>(
        std: Arc<Std<F>>,
        step_range_check: Arc<Vec<AtomicU32>>,
        large_range_checks: &[u32],
    ) {
        let range_id = std.get_range(1, MEMORY_MAX_DIFF as i64, None);
        for (value, _multiplicity) in step_range_check.iter().enumerate() {
            let multiplicity = _multiplicity.load(Ordering::Relaxed);
            if multiplicity != 0 {
                std.range_check((value + 1) as i64, multiplicity as u64, range_id);
            }
        }
        for range in large_range_checks {
            std.range_check(*range as i64, 1, range_id);
        }
    }

    /// Debug method for the main state machine.
    pub fn debug<F: PrimeField64>(_pctx: &ProofCtx<F>, _sctx: &SetupCtx<F>) {
        // No debug information to display
    }

    pub fn build_counter() -> Box<dyn BusDeviceMetrics> {
        Box::new(MainCounter::new())
    }
}
