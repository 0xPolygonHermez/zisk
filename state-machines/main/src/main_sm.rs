//! The `MainSM` module implements the Main State Machine,
//! responsible for computing witness main state machine.
//!
//! Key components of this module include:
//! - The `MainSM` struct, which handles the main execution trace computation.
//! - The `MainInstance` struct, representing the execution context of a specific main trace
//!   segment.
//! - Methods for computing the witness and setting up trace rows.

use std::sync::Arc;

use crate::MainSmError;
use fields::PrimeField64;
use mem_common::{MemHelpers, MEM_REGS_MAX_DIFF, MEM_STEPS_BY_MAIN_STEP};
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofCtx, SetupCtx};
use rayon::prelude::*;
use zisk_common::{EmuTrace, InstanceCtx, Plan, SegmentId};
use zisk_core::{ZiskRom, DEFAULT_MAX_STEPS, REGS_IN_MAIN, REGS_IN_MAIN_FROM, REGS_IN_MAIN_TO};
use zisk_pil::MainAirValues;
use ziskemu::{Emu, EmuRegTrace};

use zisk_pil::{MainTrace, MainTraceRowOps};

/// Represents an instance of the main state machine,
/// containing context for managing a specific segment of the main trace.
pub struct MainInstance<F: PrimeField64> {
    /// Instance Context
    pub ictx: InstanceCtx,

    /// Standard library for the main instance, used for range checks operations.
    pub std: Arc<Std<F>>,
}

impl<F: PrimeField64> MainInstance<F> {
    const MAX_SEGMENT_ID: usize =
        ((DEFAULT_MAX_STEPS + 1) as usize / MainTrace::<()>::NUM_ROWS) - 1;

    /// Creates a new `MainInstance`.
    ///
    /// # Arguments
    /// * `ictx` - The instance context for this main instance.
    ///
    /// # Returns
    /// A new `MainInstance`.
    pub fn new(ictx: InstanceCtx, std: Arc<Std<F>>) -> Self {
        Self { ictx, std }
    }

    /// Computes the main witness trace for a given segment based on the provided proof context,
    /// ROM, and emulation traces.
    ///
    /// # Arguments
    /// * `zisk_rom` - Reference to the Zisk ROM used for execution.
    /// * `min_traces` - A vector of the minimal traces, each segment has num_within minimal traces
    ///   inside.
    /// * `chunk_size` - The size of the minimal traces.
    ///
    /// The computed trace is added to the proof context's air instance repository.
    ///
    /// # Errors
    /// Returns a [`MainSmError`] when:
    /// - A Proofman error ([`MainSmError::Proofman`]).
    /// - The plan is missing a `segment_id` ([`MainSmError::MissingSegmentId`]).
    /// - The plan metadata is not the expected `bool`
    ///   ([`MainSmError::InvalidSegmentMetadata`]).
    /// - The segment has no minimal traces to process
    ///   ([`MainSmError::EmptyFillTraceOutput`]).
    /// - `MemHelpers::mem_step_to_slot` returned a slot outside `0..=2`
    ///   ([`MainSmError::InvalidSlot`]).
    pub fn compute_witness<R: MainTraceRowOps<F>>(
        &self,
        zisk_rom: &ZiskRom,
        min_traces: &[EmuTrace],
        chunk_size: u64,
        trace_buffer: Vec<F>,
    ) -> Result<AirInstance<F>, MainSmError> {
        const NUM_ROWS: usize = MainTrace::<()>::NUM_ROWS;

        // Compile-time assertion to ensure `MainTrace::NUM_ROWS` is a power of two.
        const _: () =
            assert!(NUM_ROWS.is_power_of_two(), "MainTrace::NUM_ROWS must be a power of two",);

        let chunk_size: usize = chunk_size.try_into()?;

        if !chunk_size.is_power_of_two() {
            return Err(MainSmError::ChunkSizeNotPowerOfTwo { size: chunk_size });
        }

        if NUM_ROWS < chunk_size {
            return Err(MainSmError::ChunkSizeTooBig { chunk_size, num_rows: NUM_ROWS });
        }

        // Create the main trace buffer
        let mut main_trace = MainTrace::<R>::new_from_vec(trace_buffer)?;

        let (segment_id, is_last_segment) = Self::decode_plan(&self.ictx.plan)?;

        // Determine the number of minimal traces per segment
        let num_within = NUM_ROWS / chunk_size;

        // Determine trace slice for the current segment
        let start_idx = segment_id.as_usize() * num_within;
        let end_idx = (start_idx + num_within).min(min_traces.len());
        let segment_min_traces = &min_traces[start_idx..end_idx];

        // Calculate total filled rows
        let filled_rows: usize =
            segment_min_traces.iter().map(|min_trace| min_trace.steps as usize).sum();

        tracing::debug!(
            "··· Creating Main segment #{} [{} / {} rows filled {:.2}%]",
            segment_id,
            filled_rows,
            NUM_ROWS,
            filled_rows as f64 / NUM_ROWS as f64 * 100.0
        );

        // Compute the segment's boundary mem-steps. `initial_step` is the mem-step at the
        // end of the previous segment (0 for the first segment); `final_step` is the
        // mem-step at the last row of this segment.
        let (initial_step, final_step) = Self::mem_steps_for_segment(segment_id, NUM_ROWS);

        // To reduce memory used, only take memory for the maximum range of mem_step inside the
        // minimal trace. `chunk_size <= NUM_ROWS` and `MEM_STEPS_BY_MAIN_STEP` is a small
        // constant, so `chunk_size * MEM_STEPS_BY_MAIN_STEP` fits in usize by construction.
        let max_range = chunk_size * MEM_STEPS_BY_MAIN_STEP as usize;

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
                let mut step_range_check = vec![0; max_range];
                let init_chunk_step = if chunk_id == 0 { initial_step } else { 0 };
                let mut reg_trace = EmuRegTrace::from_init_step(init_chunk_step, chunk_id == 0);
                let (pc, regs) = Self::fill_partial_trace::<R>(
                    zisk_rom,
                    chunk,
                    &segment_min_traces[chunk_id],
                    &mut reg_trace,
                    &mut step_range_check,
                    chunk_id == (end_idx - start_idx - 1),
                );
                (pc, regs, reg_trace, step_range_check)
            })
            .collect::<Vec<(u64, Vec<u64>, EmuRegTrace, Vec<u32>)>>();
        let last_result = fill_trace_outputs.last().ok_or(MainSmError::EmptyFillTraceOutput)?;
        let next_pc = last_result.0;

        let mut step_range_check: Vec<u32> = (0..max_range)
            .into_par_iter()
            .map(|i| fill_trace_outputs.iter().map(|(_, _, _, local)| local[i]).sum())
            .collect();

        // In the range checks are values too large to store in steps_range_check, but there
        // are only a few values that exceed this limit, for this reason, are stored in a vector

        let mut reg_steps = [initial_step; REGS_IN_MAIN];
        let mut large_range_checks = Self::complete_trace_with_initial_reg_steps_per_chunk::<R>(
            NUM_ROWS,
            &fill_trace_outputs,
            &mut main_trace,
            &mut step_range_check,
            &mut reg_steps,
        )?;

        Self::update_reg_steps_with_last_chunk(&last_result.2, &mut reg_steps);

        // Pad remaining rows with the last valid row.
        // In padding row must be clear of registers access, if not need to calculate previous
        // register step and range check conntribution.
        let last_row = Self::pad_trailing_rows(&mut main_trace.buffer, filled_rows, NUM_ROWS);

        // Determine the last row of the previous segment
        let prev_segment_last_c = if start_idx > 0 {
            Emu::intermediate_value(min_traces[start_idx - 1].last_c)
        } else {
            [F::ZERO, F::ZERO]
        };

        // Prepare main AIR values
        let mut air_values = MainAirValues::<F>::new();

        air_values.main_segment = F::from_usize(segment_id.into());
        air_values.main_last_segment = F::from_bool(is_last_segment);
        air_values.segment_initial_pc = F::from_u32(main_trace[0].get_pc());
        air_values.segment_next_pc = F::from_u64(next_pc);
        air_values.segment_previous_c = prev_segment_last_c;
        air_values.segment_last_c[0] = F::from_u32(last_row.get_c(0));
        air_values.segment_last_c[1] = F::from_u32(last_row.get_c(1));

        Self::update_reg_airvalues(
            &mut air_values,
            final_step,
            &last_result.1,
            &reg_steps,
            &mut step_range_check,
            &mut large_range_checks,
        );
        self.update_std_range_checks(segment_id, step_range_check, &large_range_checks)?;
        // Generate and add the AIR instance
        let from_trace = FromTrace::new(&mut main_trace).with_air_values(&mut air_values);
        Ok(AirInstance::new_from_trace(from_trace))
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
    fn fill_partial_trace<R: MainTraceRowOps<F>>(
        zisk_rom: &ZiskRom,
        main_trace: &mut [R],
        min_trace: &EmuTrace,
        reg_trace: &mut EmuRegTrace,
        step_range_check: &mut [u32],
        last_reg_values: bool,
    ) -> (u64, Vec<u64>) {
        // Initialize the emulator with the start state of the emu trace
        let mut emu = Emu::from_emu_trace_start(zisk_rom, &min_trace.start_state);
        let mut mem_reads_index: usize = 0;

        for trace in main_trace {
            emu.step_slice_full_trace::<R, F>(
                trace,
                &min_trace.mem_reads,
                &mut mem_reads_index,
                reg_trace,
                Some(step_range_check),
            );
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

    /// Propagates per-register previous mem-step state across consecutive chunks of
    /// the segment, mutating `main_trace` and `step_range_check` in place. Returns
    /// the vector of out-of-range values (`large_range_checks`) for the caller to
    /// fold into the std range-check pipeline.
    ///
    /// # Errors
    /// Returns [`MainSmError::InvalidSlot`] if `MemHelpers::mem_step_to_slot`
    /// produces a value outside `0..=2`.
    fn complete_trace_with_initial_reg_steps_per_chunk<R: MainTraceRowOps<F>>(
        num_rows: usize,
        fill_trace_outputs: &[(u64, Vec<u64>, EmuRegTrace, Vec<u32>)],
        main_trace: &mut MainTrace<R>,
        step_range_check: &mut [u32],
        reg_steps: &mut [u64; REGS_IN_MAIN],
    ) -> Result<Vec<u32>, MainSmError> {
        let mut large_range_checks: Vec<u32> = vec![];
        let max_range = step_range_check.len() as u64;
        for (index, (_, _, reg_trace, _)) in fill_trace_outputs.iter().enumerate().skip(1) {
            #[allow(clippy::needless_range_loop)]
            for reg_index in 0..REGS_IN_MAIN {
                let reg_prev_mem_step = if fill_trace_outputs[index - 1].2.reg_steps[reg_index] == 0
                {
                    reg_steps[reg_index]
                } else {
                    fill_trace_outputs[index - 1].2.reg_steps[reg_index]
                };
                reg_steps[reg_index] = reg_prev_mem_step;
                if let Some(mem_step) = reg_trace.first_step_uses[reg_index] {
                    let slot = MemHelpers::mem_step_to_slot(mem_step);
                    let row = MemHelpers::mem_step_to_row(mem_step) % num_rows;
                    let range = mem_step - reg_prev_mem_step - 1;
                    if range >= max_range {
                        large_range_checks.push(range as u32);
                    } else {
                        step_range_check[range as usize] += 1;
                    }
                    match slot {
                        0 => {
                            main_trace.buffer[row].set_a_reg_prev_mem_step(reg_prev_mem_step);
                        }
                        1 => {
                            main_trace.buffer[row].set_b_reg_prev_mem_step(reg_prev_mem_step);
                        }
                        2 => {
                            main_trace.buffer[row].set_store_reg_prev_mem_step(reg_prev_mem_step);
                        }
                        _ => return Err(MainSmError::InvalidSlot { slot }),
                    }
                    // TODO: range_check mem_step - reg_prev_mem_step
                }
            }
        }
        Ok(large_range_checks)
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
    fn update_reg_airvalues(
        air_values: &mut MainAirValues<'_, F>,
        final_step: u64,
        last_reg_values: &[u64],
        reg_steps: &[u64; REGS_IN_MAIN],
        step_range_check: &mut [u32],
        large_range_checks: &mut Vec<u32>,
    ) {
        let max_range = step_range_check.len() as u64;
        for ireg in 0..REGS_IN_MAIN {
            let reg_value = last_reg_values[ireg];
            let values = [F::from_u32(reg_value as u32), F::from_u32((reg_value >> 32) as u32)];
            air_values.last_reg_value[ireg] = values;
            air_values.last_reg_mem_step[ireg] = F::from_u64(reg_steps[ireg]);
            let range = (final_step - reg_steps[ireg] - 1) as usize;
            if range >= max_range as usize {
                large_range_checks.push(range as u32);
            } else {
                step_range_check[range] += 1;
            }
        }
    }

    /// Updates the standard library range checks for the main instance
    /// based on the provided segment ID, step range checks, and large range checks.
    ///
    /// # Errors
    /// Returns [`MainSmError::Proofman`] if `pil_std_lib::Std::get_range_id` fails
    /// to resolve the range IDs for the `mem_step` or `segment_id` range checks.
    /// This indicates a setup-time misconfiguration of the standard library.
    fn update_std_range_checks(
        &self,
        segment_id: SegmentId,
        step_range_check: Vec<u32>,
        large_range_checks: &[u32],
    ) -> Result<(), MainSmError> {
        let range_id = self.std.get_range_id(0, MEM_REGS_MAX_DIFF as i64, None)?;
        self.std.range_checks(range_id, step_range_check);

        for range in large_range_checks {
            self.std.range_check(range_id, *range as i64, 1);
        }

        let range_id = self.std.get_range_id(0, Self::MAX_SEGMENT_ID as i64, None)?;
        self.std.range_check(range_id, segment_id.as_usize() as i64, 1);
        Ok(())
    }

    /// Computes the boundary mem-steps for a given segment of the main trace.
    ///
    /// Returns `(initial_step, final_step)`:
    /// - `initial_step`: mem-step at the last row of segment `segment_id - 1`
    ///   (or at row 0 for `segment_id == 0`, since there is no previous segment).
    /// - `final_step`: mem-step at the last row of segment `segment_id`.
    ///
    /// Adjacent segments are contiguous in mem-step space:
    /// `mem_steps_for_segment(s, n).1 == mem_steps_for_segment(s + 1, n).0`.
    fn mem_steps_for_segment(segment_id: SegmentId, num_rows: usize) -> (u64, u64) {
        let last_row_previous_segment =
            if segment_id == 0 { 0 } else { (segment_id.as_usize() * num_rows) as u64 - 1 };
        let initial_step = MemHelpers::main_step_to_special_mem_step(last_row_previous_segment);
        let final_step = MemHelpers::main_step_to_special_mem_step(
            ((segment_id.as_usize() + 1) * num_rows) as u64 - 1,
        );
        (initial_step, final_step)
    }

    /// Decodes `segment_id` and `is_last_segment` from the plan handed to this
    /// instance.
    ///
    /// # Errors
    /// - [`MainSmError::MissingSegmentId`] if the plan has no `segment_id`.
    /// - [`MainSmError::InvalidSegmentMetadata`] if the metadata is missing or
    ///   isn't a `bool`.
    fn decode_plan(plan: &Plan) -> Result<(SegmentId, bool), MainSmError> {
        let segment_id = plan.segment_id.ok_or(MainSmError::MissingSegmentId)?;
        let is_last_segment = plan
            .meta
            .as_ref()
            .and_then(|m| m.downcast_ref::<bool>())
            .copied()
            .ok_or(MainSmError::InvalidSegmentMetadata)?;
        Ok((segment_id, is_last_segment))
    }

    /// Pads `buffer[filled_rows..num_rows]` with `buffer[filled_rows - 1]` (the
    /// last filled row), in parallel. Returns the row used for padding so the
    /// caller can reuse it (e.g. for AIR values).
    ///
    /// Caller must ensure `1 <= filled_rows <= num_rows <= buffer.len()`.
    fn pad_trailing_rows<R: Copy + Send + Sync>(
        buffer: &mut [R],
        filled_rows: usize,
        num_rows: usize,
    ) -> R {
        let last_row = buffer[filled_rows - 1];
        buffer[filled_rows..num_rows].par_iter_mut().for_each(|row| *row = last_row);
        last_row
    }
}

/// The `MainSM` struct represents the Main State Machine,
/// responsible for generating the main witness.
pub struct MainSM {}

impl MainSM {
    /// Debug method for the main state machine.
    pub fn debug<F: PrimeField64>(_pctx: &ProofCtx<F>, _sctx: &SetupCtx<F>) {
        // No debug information to display
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fields::Goldilocks;
    use std::any::Any;
    use zisk_common::{CheckPoint, ChunkId, InstanceType};

    // `mem_steps_for_segment` doesn't use `F`, but it's now an associated fn on
    // `MainInstance<F>`, so the call site has to pick some concrete `F`.
    type MI = MainInstance<Goldilocks>;

    fn make_plan(segment_id: Option<SegmentId>, meta: Option<Box<dyn Any>>) -> Plan {
        Plan::new(0, 0, segment_id, InstanceType::Instance, CheckPoint::Single(ChunkId(0)), meta)
    }

    #[test]
    fn segment_zero_starts_at_row_zero() {
        let num_rows = 1 << 8; // 256 rows
        let (initial, final_) = MI::mem_steps_for_segment(SegmentId(0), num_rows);
        assert_eq!(initial, MemHelpers::main_step_to_special_mem_step(0));
        assert_eq!(final_, MemHelpers::main_step_to_special_mem_step(num_rows as u64 - 1));
    }

    #[test]
    fn segment_one_starts_at_end_of_segment_zero() {
        let num_rows = 1 << 8; // 256 rows
        let (initial, final_) = MI::mem_steps_for_segment(SegmentId(1), num_rows);
        assert_eq!(initial, MemHelpers::main_step_to_special_mem_step(num_rows as u64 - 1));
        assert_eq!(final_, MemHelpers::main_step_to_special_mem_step(2 * num_rows as u64 - 1));
    }

    #[test]
    fn arbitrary_segment_uses_correct_row_indices() {
        let num_rows = 1 << 8; // 256 rows
        let s = 5usize;
        let (initial, final_) = MI::mem_steps_for_segment(SegmentId(s), num_rows);
        let expected_last_row_prev = (s * num_rows) as u64 - 1;
        let expected_final_row = ((s + 1) * num_rows) as u64 - 1;
        assert_eq!(initial, MemHelpers::main_step_to_special_mem_step(expected_last_row_prev));
        assert_eq!(final_, MemHelpers::main_step_to_special_mem_step(expected_final_row));
    }

    #[test]
    fn consecutive_segments_are_contiguous_in_mem_step_space() {
        // The invariant the planner + witness pipeline rely on: segment `s`'s `final_step`
        // is the same mem-step as segment `s + 1`'s `initial_step`.
        let num_rows = 1 << 8; // 256 rows
        for s in 0..4 {
            let (_, final_s) = MI::mem_steps_for_segment(SegmentId(s), num_rows);
            let (initial_next, _) = MI::mem_steps_for_segment(SegmentId(s + 1), num_rows);
            assert_eq!(final_s, initial_next, "discontinuity between segment {s} and {}", s + 1);
        }
    }

    #[test]
    fn num_rows_one_does_not_panic() {
        // Degenerate but valid: a single-row segment.
        let (initial, final_) = MI::mem_steps_for_segment(SegmentId(0), 1);
        assert_eq!(initial, MemHelpers::main_step_to_special_mem_step(0));
        assert_eq!(final_, MemHelpers::main_step_to_special_mem_step(0));
    }

    #[test]
    fn decode_plan_returns_segment_id_and_last_flag() {
        let plan = make_plan(Some(SegmentId(5)), Some(Box::new(true)));
        let (id, is_last) = MI::decode_plan(&plan).expect("valid plan");
        assert_eq!(id, SegmentId(5));
        assert!(is_last);
    }

    #[test]
    fn decode_plan_returns_false_for_non_last_segment() {
        let plan = make_plan(Some(SegmentId(2)), Some(Box::new(false)));
        let (_id, is_last) = MI::decode_plan(&plan).expect("valid plan");
        assert!(!is_last);
    }

    #[test]
    fn decode_plan_missing_segment_id_errors() {
        let plan = make_plan(None, Some(Box::new(true)));
        let err = MI::decode_plan(&plan).unwrap_err();
        assert!(matches!(err, MainSmError::MissingSegmentId));
    }

    #[test]
    fn decode_plan_wrong_metadata_type_errors() {
        let plan = make_plan(Some(SegmentId(0)), Some(Box::new(42i32)));
        let err = MI::decode_plan(&plan).unwrap_err();
        assert!(matches!(err, MainSmError::InvalidSegmentMetadata));
    }

    #[test]
    fn decode_plan_missing_metadata_errors() {
        let plan = make_plan(Some(SegmentId(0)), None);
        let err = MI::decode_plan(&plan).unwrap_err();
        assert!(matches!(err, MainSmError::InvalidSegmentMetadata));
    }

    #[test]
    fn pad_trailing_rows_fills_tail_with_last_filled_row() {
        let mut buf = [1u32, 2, 3, 4, 5, 0, 0, 0, 0, 0];
        let last = MI::pad_trailing_rows(&mut buf, 5, 10);
        assert_eq!(last, 5);
        assert_eq!(buf, [1, 2, 3, 4, 5, 5, 5, 5, 5, 5]);
    }

    #[test]
    fn pad_trailing_rows_filled_equals_num_rows_is_noop() {
        let mut buf = [1u32, 2, 3, 4, 5];
        let before = buf;
        let last = MI::pad_trailing_rows(&mut buf, 5, 5);
        assert_eq!(last, 5);
        assert_eq!(buf, before);
    }

    #[test]
    fn pad_trailing_rows_single_filled_row_pads_rest() {
        let mut buf = [42u32, 0, 0, 0];
        let last = MI::pad_trailing_rows(&mut buf, 1, 4);
        assert_eq!(last, 42);
        assert_eq!(buf, [42, 42, 42, 42]);
    }
}
