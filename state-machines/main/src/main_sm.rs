//! The `MainSM` module implements the Main State Machine,
//! responsible for computing witness main state machine.
//!
//! Key components of this module include:
//! - The `MainSM` struct, which handles the main execution trace computation.
//! - The `MainInstance` struct, representing the execution context of a specific main trace
//!   segment.
//! - Methods for computing the witness and setting up trace rows.

use std::sync::Arc;

use crate::{MainPlanner, MainSmError};
use pil2_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofCtx, SetupCtx};
use proofman_fields::PrimeField64;
use rayon::prelude::*;
use zisk_common::{EmuTrace, InstanceCtx, Plan, SegmentId};
use zisk_core::{ZiskRom, DEFAULT_MAX_STEPS, REGS_IN_MAIN, REGS_IN_MAIN_FROM, REGS_IN_MAIN_TO};
use zisk_pil::{MainAirValues, MAIN_LANES, MAIN_STEPS_PER_SEGMENT};
use zisk_sm_mem_common::{MemHelpers, MEM_REGS_MAX_DIFF, MEM_STEPS_BY_MAIN_STEP};
use ziskemu::{Emu, EmuRegTrace};

use zisk_pil::{IndexedFill, MainTrace, MainTraceRowOps};

/// What filling one minimal-trace chunk hands back to [`MainInstance::compute_witness`].
struct ChunkFill<R> {
    /// `pc` after the chunk's last step.
    next_pc: u64,
    /// Register values at the end of the chunk, when the caller asked for them.
    reg_values: Vec<u64>,
    /// Per-register mem-step bookkeeping, folded into the segment's chain.
    reg_trace: EmuRegTrace,
    /// This chunk's contribution to the mem-step range checks.
    step_range_check: Vec<u32>,
    /// A row of end-instruction steps to pad the segment's tail with. Only the last chunk
    /// of a short segment produces one.
    pad_row: Option<R>,
}

/// Represents an instance of the main state machine,
/// containing context for managing a specific segment of the main trace.
pub struct MainInstance<F: PrimeField64> {
    /// Instance Context
    pub ictx: InstanceCtx,

    /// Standard library for the main instance, used for range checks operations.
    pub std: Arc<Std<F>>,
}

impl<F: PrimeField64> MainInstance<F> {
    /// Maximum segment ID allowed, derived from `DEFAULT_MAX_STEPS` and the steps a
    /// segment covers.
    const MAX_SEGMENT_ID: usize =
        (((DEFAULT_MAX_STEPS + 1) / MAIN_STEPS_PER_SEGMENT as u64) - 1) as usize;

    /// Size in main steps of a register flush window (mirrors FLUSH_SIZE in main.pil):
    /// registers are reloaded every `min(MAIN_STEPS_PER_SEGMENT, FLUSH_WINDOW_STEPS)` steps.
    const FLUSH_WINDOW_STEPS: usize = 1 << 22;

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
    /// * `segment_min_traces` - This segment's minimal traces (at most `num_within`).
    /// * `prev_chunk_last_c` - `last_c` of the chunk preceding the segment (`None` for the
    ///   first segment).
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
    /// - A non-final segment was given fewer than `num_within` minimal traces
    ///   ([`MainSmError::IncompleteSegment`]).
    /// - `MemHelpers::mem_step_to_slot` returned a slot outside `0..=2`
    ///   ([`MainSmError::InvalidSlot`]).
    pub fn compute_witness<R: MainTraceRowOps<F> + IndexedFill>(
        &self,
        zisk_rom: &ZiskRom,
        segment_min_traces: &[std::sync::Arc<EmuTrace>],
        prev_chunk_last_c: Option<u64>,
        chunk_size: u64,
        trace_buffer: Vec<F>,
    ) -> Result<AirInstance<F>, MainSmError> {
        const NUM_ROWS: usize = MainTrace::<()>::NUM_ROWS;

        let chunk_size: usize = chunk_size.try_into()?;
        MainPlanner::validate_chunk_size(chunk_size)?;

        // Create the main trace buffer
        let mut main_trace = MainTrace::<R>::new_from_vec(trace_buffer)?;

        let (segment_id, is_last_segment) = Self::decode_plan(&self.ictx.plan)?;

        // Determine the number of minimal traces per segment
        let num_within = MAIN_STEPS_PER_SEGMENT / chunk_size;

        Self::check_segment_complete(
            segment_id,
            segment_min_traces.len(),
            num_within,
            is_last_segment,
        )?;

        // The execution steps that ran before this segment, i.e. the main step its first row
        // starts at. This is `segment_initial_step` in main.pil, and it is read off the segment's
        // first chunk rather than derived from the segment id: the chunk is what actually knows
        // where the segment starts, and the airval exists precisely so that segments need not all
        // be the same size.
        let first_min_trace =
            segment_min_traces.first().ok_or(MainSmError::EmptyFillTraceOutput)?;
        let segment_initial_step = first_min_trace.start_state.step;

        // Steps the minimal traces actually cover, and the rows they occupy. A row is filled
        // as soon as any of its lanes carries a real step: the lanes past the execution's end
        // hold the end instruction re-executed, which the padding below then repeats.
        let filled_steps: usize =
            segment_min_traces.iter().map(|min_trace| min_trace.steps as usize).sum();
        let filled_rows = filled_steps.div_ceil(MAIN_LANES);

        tracing::debug!(
            "··· Creating Main segment #{} [{} / {} steps filled {:.2}%]",
            segment_id,
            filled_steps,
            MAIN_STEPS_PER_SEGMENT,
            filled_steps as f64 / MAIN_STEPS_PER_SEGMENT as f64 * 100.0
        );

        // The mem-step this segment chains from: the one of the last step before it.
        let initial_step = Self::previous_mem_step(segment_initial_step);

        // Registers are reloaded (flushed) in windows of at most 2^22 main steps (mirrors
        // FLUSH_COUNT / FLUSH_SIZE in main.pil): each flush restarts the register range
        // check distance, keeping it below 2^24 regardless of the segment's step count.
        let flush_size = MAIN_STEPS_PER_SEGMENT.min(Self::FLUSH_WINDOW_STEPS);
        let flush_count = MAIN_STEPS_PER_SEGMENT / flush_size;
        if chunk_size > flush_size || flush_size % chunk_size != 0 {
            // flush windows must be aligned to chunk boundaries
            return Err(MainSmError::ChunkSizeTooBig { chunk_size, max_steps: flush_size });
        }
        let chunks_per_flush = flush_size / chunk_size;
        let flush_steps: Vec<u64> = (0..flush_count)
            .map(|flush_index| {
                MemHelpers::main_step_to_special_mem_step(
                    segment_initial_step + ((flush_index + 1) * flush_size) as u64 - 1,
                )
            })
            .collect();

        // To reduce memory used, only take memory for the maximum range of mem_step inside the
        // minimal trace. `chunk_size <= MAIN_STEPS_PER_SEGMENT` and `MEM_STEPS_BY_MAIN_STEP` is a
        // small constant, so `chunk_size * MEM_STEPS_BY_MAIN_STEP` fits in usize by construction.
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
                // register values are needed at the end of every flush window (to close it)
                // and at the last executed chunk (to close the remaining windows)
                let last_reg_values = (chunk_id + 1) % chunks_per_flush == 0
                    || chunk_id == (segment_min_traces.len() - 1);
                // Only the last chunk of a short segment needs the padding row, and a segment
                // is only short once the execution has ended — so the emulator is sitting on
                // the end instruction there, which is what makes the extra steps free of
                // minimal-trace data.
                let with_pad_row =
                    chunk_id == (segment_min_traces.len() - 1) && filled_rows < NUM_ROWS;
                let (next_pc, reg_values, pad_row) = Self::fill_partial_trace::<R>(
                    zisk_rom,
                    chunk,
                    &segment_min_traces[chunk_id],
                    &mut reg_trace,
                    &mut step_range_check,
                    last_reg_values,
                    with_pad_row,
                );
                ChunkFill { next_pc, reg_values, reg_trace, step_range_check, pad_row }
            })
            .collect::<Vec<ChunkFill<R>>>();
        let last_result = fill_trace_outputs.last().ok_or(MainSmError::EmptyFillTraceOutput)?;
        let next_pc = last_result.next_pc;
        let pad_row = last_result.pad_row;

        let mut step_range_check: Vec<u32> = (0..max_range)
            .into_par_iter()
            .map(|i| fill_trace_outputs.iter().map(|fill| fill.step_range_check[i]).sum())
            .collect();

        // In the range checks are values too large to store in steps_range_check, but there
        // are only a few values that exceed this limit, for this reason, are stored in a vector

        // Prepare main AIR values (filled below and by the flush-window closings)
        let mut air_values = MainAirValues::<F>::new();

        let mut reg_steps = [initial_step; REGS_IN_MAIN];
        let mut large_range_checks = Self::complete_trace_with_initial_reg_steps_per_chunk::<R>(
            &fill_trace_outputs,
            &mut main_trace,
            &mut step_range_check,
            &mut reg_steps,
            chunks_per_flush,
            &flush_steps,
            &mut air_values,
        )?;

        Self::update_reg_steps_with_last_chunk(&last_result.reg_trace, &mut reg_steps);

        // Close the remaining flush windows: the one where the execution ended and any later
        // (empty) window. Register values stay at their final state; each closing restarts
        // `reg_steps` at its flush mem-step, so empty windows chain flush to flush.
        let closed_windows = (fill_trace_outputs.len() - 1) / chunks_per_flush;
        for (flush_index, &flush_step) in flush_steps.iter().enumerate().skip(closed_windows) {
            Self::close_flush_window(
                &mut air_values,
                flush_index,
                flush_step,
                &last_result.reg_values,
                &mut reg_steps,
                &mut step_range_check,
                &mut large_range_checks,
            );
        }

        // Pad the segment's tail, and take the row that ends up last — the AIR values below
        // read the hand-over `c` off it.
        let last_row = match pad_row {
            Some(pad_row) => {
                Self::pad_trailing_rows(&mut main_trace.buffer, filled_rows, NUM_ROWS, pad_row)
            }
            None => main_trace.buffer[NUM_ROWS - 1],
        };

        // Determine the last row of the previous segment
        let prev_segment_last_c = match prev_chunk_last_c {
            Some(last_c) => Emu::intermediate_value(last_c),
            None => [F::ZERO, F::ZERO],
        };

        air_values.main_segment = F::from_usize(segment_id.into());
        air_values.main_last_segment = F::from_bool(is_last_segment);
        air_values.segment_initial_step = F::from_u64(segment_initial_step);
        // From the ROM, not the trace: row 0's `pc` column is instruction-derived, so
        // `main_trace[0].get_pc(0)` is 0 on the compact indexed row.
        let segment_initial_pc =
            zisk_rom.get_instruction(first_min_trace.start_state.pc).paddr as u32;
        air_values.segment_initial_pc = F::from_u32(segment_initial_pc);
        air_values.segment_next_pc = F::from_u64(next_pc);
        air_values.segment_previous_c = prev_segment_last_c;
        // The segment hands over the `c` of its LAST step, which is the last lane of the
        // last row (mirrors the `SEGMENT_LAST` constraint in main.pil).
        air_values.segment_last_c[0] = F::from_u32(last_row.get_c(MAIN_LANES - 1, 0));
        air_values.segment_last_c[1] = F::from_u32(last_row.get_c(MAIN_LANES - 1, 1));

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
    /// The next program counter value after processing the minimal trace, the register
    /// values when `last_reg_values`, and — when `with_pad_row` — one extra row holding
    /// `MAIN_LANES` further steps, for the caller to pad the segment's tail with.
    #[allow(clippy::too_many_arguments)]
    fn fill_partial_trace<R: MainTraceRowOps<F> + IndexedFill>(
        zisk_rom: &ZiskRom,
        main_trace: &mut [R],
        min_trace: &EmuTrace,
        reg_trace: &mut EmuRegTrace,
        step_range_check: &mut [u32],
        last_reg_values: bool,
        with_pad_row: bool,
    ) -> (u64, Vec<u64>, Option<R>) {
        // Initialize the emulator with the start state of the emu trace
        let mut emu = Emu::from_emu_trace_start(zisk_rom, &min_trace.start_state);
        let mut mem_reads_index: usize = 0;

        // Each row packs `MAIN_LANES` consecutive steps, so a row is written lane by lane
        // before moving on. The chunk always spans whole rows (`validate_chunk_size`).
        for trace in main_trace {
            for lane in 0..MAIN_LANES {
                emu.step_slice_full_trace::<R, F>(
                    trace,
                    lane,
                    &min_trace.mem_reads,
                    &mut mem_reads_index,
                    reg_trace,
                    Some(step_range_check),
                );
            }
        }

        // The padding row is asked for only past the end of the execution, where the emulator
        // is looping on the end instruction: it reads no minimal-trace data and touches no
        // register, so these steps neither consume `mem_reads` nor contribute range checks.
        // Building it from the emulator rather than copying a filled row is what keeps the
        // pc chain intact: a row that mixes real steps with end-instruction lanes cannot be
        // repeated, since its lane 0 would no longer follow the previous row's last lane.
        let pad_row = with_pad_row.then(|| {
            let mut pad_row = R::default();
            for lane in 0..MAIN_LANES {
                emu.step_slice_full_trace::<R, F>(
                    &mut pad_row,
                    lane,
                    &min_trace.mem_reads,
                    &mut mem_reads_index,
                    reg_trace,
                    None,
                );
            }
            pad_row
        });

        (
            emu.ctx.inst_ctx.pc,
            if last_reg_values {
                emu.ctx.inst_ctx.regs[REGS_IN_MAIN_FROM..=REGS_IN_MAIN_TO].to_vec()
            } else {
                vec![]
            },
            pad_row,
        )
    }

    /// Propagates per-register previous mem-step state across consecutive chunks of
    /// the segment, mutating `main_trace` and `step_range_check` in place. Whenever a
    /// chunk boundary is also a flush-window boundary, the corresponding flush is
    /// closed: its airvalues record the last access of the window and `reg_steps`
    /// restarts at the flush mem-step (the reload becomes the previous access of the
    /// next window). Returns the vector of out-of-range values (`large_range_checks`)
    /// for the caller to fold into the std range-check pipeline.
    ///
    /// # Errors
    /// Returns [`MainSmError::InvalidSlot`] if `MemHelpers::mem_step_to_slot`
    /// produces a value outside `0..=2`.
    #[allow(clippy::too_many_arguments)]
    fn complete_trace_with_initial_reg_steps_per_chunk<R: MainTraceRowOps<F> + IndexedFill>(
        fill_trace_outputs: &[ChunkFill<R>],
        main_trace: &mut MainTrace<R>,
        step_range_check: &mut [u32],
        reg_steps: &mut [u64; REGS_IN_MAIN],
        chunks_per_flush: usize,
        flush_steps: &[u64],
        air_values: &mut MainAirValues<'_, F>,
    ) -> Result<Vec<u32>, MainSmError> {
        let mut large_range_checks: Vec<u32> = vec![];
        let max_range = step_range_check.len() as u64;
        for (index, fill) in fill_trace_outputs.iter().enumerate().skip(1) {
            let reg_trace = &fill.reg_trace;
            // fold the previous chunk's last register steps into the carried steps
            Self::update_reg_steps_with_last_chunk(
                &fill_trace_outputs[index - 1].reg_trace,
                reg_steps,
            );

            // close the flush window ending at this chunk boundary, if any
            if index % chunks_per_flush == 0 {
                let flush_index = index / chunks_per_flush - 1;
                Self::close_flush_window(
                    air_values,
                    flush_index,
                    flush_steps[flush_index],
                    &fill_trace_outputs[index - 1].reg_values,
                    reg_steps,
                    step_range_check,
                    &mut large_range_checks,
                );
            }

            #[allow(clippy::needless_range_loop)]
            for reg_index in 0..REGS_IN_MAIN {
                let reg_prev_mem_step = reg_steps[reg_index];
                if let Some(mem_step) = reg_trace.first_step_uses[reg_index] {
                    let slot = MemHelpers::mem_step_to_slot(mem_step);
                    // `mem_step_to_row` yields the main step; the segment's steps are laid
                    // out `MAIN_LANES` per row, in lane order.
                    let segment_step =
                        MemHelpers::mem_step_to_row(mem_step) % MAIN_STEPS_PER_SEGMENT;
                    let row = segment_step / MAIN_LANES;
                    let lane = segment_step % MAIN_LANES;
                    let range = mem_step - reg_prev_mem_step - 1;
                    if range >= max_range {
                        large_range_checks.push(range as u32);
                    } else {
                        step_range_check[range as usize] += 1;
                    }
                    match slot {
                        0 => {
                            main_trace.buffer[row].set_a_reg_prev_mem_step(lane, reg_prev_mem_step);
                        }
                        1 => {
                            main_trace.buffer[row].set_b_reg_prev_mem_step(lane, reg_prev_mem_step);
                        }
                        2 => {
                            main_trace.buffer[row]
                                .set_store_reg_prev_mem_step(lane, reg_prev_mem_step);
                        }
                        _ => return Err(MainSmError::InvalidSlot { slot }),
                    }
                    // TODO: range_check mem_step - reg_prev_mem_step
                }
            }
        }
        Ok(large_range_checks)
    }

    /// Updates `reg_steps` with the last chunk's register steps, which are the initial
    /// steps for the next chunk.
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
    /// Closes a register flush window: records in the airvalues the last access
    /// (mem-step and value) of each register within the window, counts the range
    /// check between that access and the flush mem-step, and restarts `reg_steps`
    /// at the flush mem-step, since the reload becomes the previous access of the
    /// next window (or of the next segment, for the last window).
    fn close_flush_window(
        air_values: &mut MainAirValues<'_, F>,
        flush_index: usize,
        flush_step: u64,
        last_reg_values: &[u64],
        reg_steps: &mut [u64; REGS_IN_MAIN],
        step_range_check: &mut [u32],
        large_range_checks: &mut Vec<u32>,
    ) {
        let max_range = step_range_check.len() as u64;
        for ireg in 0..REGS_IN_MAIN {
            let reg_value = last_reg_values[ireg];
            let values = [F::from_u32(reg_value as u32), F::from_u32((reg_value >> 32) as u32)];
            air_values.last_reg_value[flush_index][ireg] = values;
            air_values.last_reg_mem_step[flush_index][ireg] = F::from_u64(reg_steps[ireg]);
            let range = (flush_step - reg_steps[ireg] - 1) as usize;
            if range >= max_range as usize {
                large_range_checks.push(range as u32);
            } else {
                step_range_check[range] += 1;
            }
            reg_steps[ireg] = flush_step;
        }
    }

    /// Updates the standard library range checks for the main instance
    /// based on the provided segment ID, step range checks, and large range checks.
    ///
    /// # Errors
    /// Returns [`MainSmError::Proofman`] if `pil2_std_lib::Std::get_range_id` fails
    /// to resolve the range IDs for the `mem_step` or `segment_id` range checks.
    /// This indicates a setup-time misconfiguration of the standard library.
    fn update_std_range_checks(
        &self,
        segment_id: SegmentId,
        step_range_check: Vec<u32>,
        large_range_checks: &[u32],
    ) -> Result<(), MainSmError> {
        let range_id = self.std.get_range_id(0, MEM_REGS_MAX_DIFF as i64, None)?;
        self.std.range_check_ranged(range_id, None, &step_range_check);

        for range in large_range_checks {
            self.std.range_check_one(range_id, *range);
        }

        let range_id = self.std.get_range_id(0, Self::MAX_SEGMENT_ID as i64, None)?;
        self.std.range_check_one(range_id, segment_id.as_usize());
        Ok(())
    }

    /// The mem-step a segment starting at main step `segment_initial_step` chains from: the one
    /// of the last step before it.
    ///
    /// The first segment has nothing before it, so it chains from step 0's own mem-step. Every
    /// other segment picks up exactly where the previous one left off, which is what makes the
    /// register range checks continuous across the boundary.
    fn previous_mem_step(segment_initial_step: u64) -> u64 {
        MemHelpers::main_step_to_special_mem_step(segment_initial_step.saturating_sub(1))
    }

    /// Rejects a non-final segment that was handed fewer minimal traces than it
    /// spans. Only the last segment may be partial; anywhere else a short slice
    /// means the caller's minimal-trace store did not hold this segment's whole
    /// chunk range, and padding the missing rows would quietly yield a truncated
    /// Main witness that only surfaces as a global-constraint failure.
    ///
    /// An empty *non-final* segment is short like any other and is rejected here.
    /// An empty *final* segment is the one case left to
    /// [`MainSmError::EmptyFillTraceOutput`], which already rejects it further
    /// down `compute_witness` and names the actual problem.
    ///
    /// # Errors
    /// - [`MainSmError::IncompleteSegment`] if a non-final segment is short.
    fn check_segment_complete(
        segment_id: SegmentId,
        got: usize,
        num_within: usize,
        is_last_segment: bool,
    ) -> Result<(), MainSmError> {
        if !is_last_segment && got != num_within {
            return Err(MainSmError::IncompleteSegment {
                segment_id: segment_id.as_usize(),
                got,
                expected: num_within,
            });
        }
        Ok(())
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

    /// Pads `buffer[filled_rows..num_rows]` with `pad_row`, in parallel, and returns the
    /// segment's final row — `pad_row` itself when anything was padded, and the last row the
    /// emulator wrote when the filled rows already reached `num_rows`.
    ///
    /// Caller must ensure `1 <= filled_rows <= num_rows <= buffer.len()`.
    fn pad_trailing_rows<R: Copy + Send + Sync>(
        buffer: &mut [R],
        filled_rows: usize,
        num_rows: usize,
        pad_row: R,
    ) -> R {
        buffer[filled_rows..num_rows].par_iter_mut().for_each(|row| *row = pad_row);
        buffer[num_rows - 1]
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
    use proofman_fields::Goldilocks;
    use std::any::Any;
    use zisk_common::{CheckPoint, ChunkId, InstanceType};

    // `mem_steps_for_segment` doesn't use `F`, but it's now an associated fn on
    // `MainInstance<F>`, so the call site has to pick some concrete `F`.
    type MI = MainInstance<Goldilocks>;

    fn make_plan(segment_id: Option<SegmentId>, meta: Option<Box<dyn Any + Send + Sync>>) -> Plan {
        Plan::new(0, 0, segment_id, InstanceType::Instance, CheckPoint::Single(ChunkId(0)), meta)
    }

    #[test]
    fn full_non_final_segment_is_accepted() {
        // The common case: a middle segment carrying exactly `num_within` chunks.
        MI::check_segment_complete(SegmentId(1), 16, 16, false).expect("complete segment");
    }

    #[test]
    fn short_non_final_segment_is_rejected() {
        // Only the last segment may be partial. A short middle segment means the
        // caller's store didn't hold the whole chunk range; computing it would pad
        // the missing rows and silently produce a truncated Main witness.
        let err = MI::check_segment_complete(SegmentId(2), 9, 16, false)
            .expect_err("truncated middle segment");
        assert!(matches!(
            err,
            MainSmError::IncompleteSegment { segment_id: 2, got: 9, expected: 16 }
        ));
    }

    #[test]
    fn empty_non_final_segment_is_rejected() {
        let err = MI::check_segment_complete(SegmentId(0), 0, 16, false)
            .expect_err("empty middle segment");
        assert!(matches!(err, MainSmError::IncompleteSegment { got: 0, expected: 16, .. }));
    }

    #[test]
    fn partial_final_segment_is_accepted() {
        // The execution's tail is legitimately partial — this must not false-fire.
        for got in 1..=16 {
            MI::check_segment_complete(SegmentId(3), got, 16, true)
                .unwrap_or_else(|e| panic!("final segment with {got} chunks rejected: {e}"));
        }
    }

    #[test]
    fn empty_final_segment_defers_to_empty_fill_trace_output() {
        // Not this check's job: `compute_witness` rejects an empty segment later
        // with `EmptyFillTraceOutput`, which names the actual problem.
        MI::check_segment_complete(SegmentId(3), 0, 16, true).expect("deferred");
    }

    #[test]
    fn the_first_segment_chains_from_step_zero() {
        // Nothing ran before it, so there is no earlier step to chain from and step 0's own
        // mem-step is what the register chain starts at.
        assert_eq!(MI::previous_mem_step(0), MemHelpers::main_step_to_special_mem_step(0));
    }

    #[test]
    fn a_later_segment_chains_from_the_step_before_it() {
        let initial = 5 * MAIN_STEPS_PER_SEGMENT as u64;
        assert_eq!(
            MI::previous_mem_step(initial),
            MemHelpers::main_step_to_special_mem_step(initial - 1)
        );
    }

    #[test]
    fn consecutive_segments_are_contiguous_in_mem_step_space() {
        // The invariant the register range checks rest on: a segment chains from the mem-step of
        // the last step of the one before it, leaving no gap at the boundary.
        let mut initial = 0u64;
        for _ in 0..4 {
            let next_initial = initial + MAIN_STEPS_PER_SEGMENT as u64;
            assert_eq!(
                MI::previous_mem_step(next_initial),
                MemHelpers::main_step_to_special_mem_step(next_initial - 1),
                "the next segment must chain from this segment's last step"
            );
            initial = next_initial;
        }
    }

    /// Segments of different sizes are the reason `segment_initial_step` is an air value rather
    /// than `segment_id * MAIN_STEPS_PER_SEGMENT`: the chain has to follow the real step counts.
    #[test]
    fn chaining_does_not_assume_uniform_segments() {
        for &initial in &[1u64, 7, 1_000, MAIN_STEPS_PER_SEGMENT as u64 + 3] {
            assert_eq!(
                MI::previous_mem_step(initial),
                MemHelpers::main_step_to_special_mem_step(initial - 1)
            );
        }
    }

    #[test]
    fn a_segment_spans_lanes_times_rows_steps() {
        // The whole lane change in one assertion: a segment still has NUM_ROWS rows, but
        // each of them carries MAIN_LANES steps.
        assert_eq!(MAIN_STEPS_PER_SEGMENT, MainTrace::<()>::NUM_ROWS * MAIN_LANES);
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
    fn pad_trailing_rows_fills_tail_with_the_pad_row() {
        let mut buf = [1u32, 2, 3, 4, 5, 0, 0, 0, 0, 0];
        let last = MI::pad_trailing_rows(&mut buf, 5, 10, 9);
        assert_eq!(last, 9);
        assert_eq!(buf, [1, 2, 3, 4, 5, 9, 9, 9, 9, 9]);
    }

    #[test]
    fn pad_trailing_rows_filled_equals_num_rows_keeps_the_emulated_last_row() {
        // Nothing to pad: the segment's final row is the one the emulator wrote, not the
        // pad row — which is why this returns `buffer[num_rows - 1]` rather than `pad_row`.
        let mut buf = [1u32, 2, 3, 4, 5];
        let before = buf;
        let last = MI::pad_trailing_rows(&mut buf, 5, 5, 9);
        assert_eq!(last, 5);
        assert_eq!(buf, before);
    }

    #[test]
    fn pad_trailing_rows_single_filled_row_pads_rest() {
        let mut buf = [42u32, 0, 0, 0];
        let last = MI::pad_trailing_rows(&mut buf, 1, 4, 7);
        assert_eq!(last, 7);
        assert_eq!(buf, [42, 7, 7, 7]);
    }
}
