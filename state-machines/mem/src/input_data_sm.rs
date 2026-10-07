use std::sync::Arc;

#[cfg(feature = "debug_mem")]
use std::{
    fs::File,
    io::{BufWriter, Write},
};

#[cfg(feature = "debug_mem")]
use zisk_sm_mem_common::MemHelpers;

use crate::mem_gpu_fill::{gpu_fill_mode, GpuFillMode};
use crate::{MemModule, MemOps, MemPreviousSegment};
use zisk_sm_mem_common::{
    MemLanes, MemModuleSegmentCheckPoint, MEM_BYTES_BITS, SEGMENT_ADDR_MAX_DISTANCE,
};

use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_fields::PrimeField64;
use rayon::prelude::*;
use zisk_common::SegmentId;
use zisk_core::{INPUT_ADDR, MAX_INPUT_SIZE};
use zisk_pil::{
    InputDataAirValues, InputDataTrace, InputDataTraceRow, InputDataTraceRowOps,
    InputDataTraceRowPacked,
};

pub const INPUT_DATA_W_ADDR_INIT: u32 = INPUT_ADDR as u32 >> MEM_BYTES_BITS;
pub const INPUT_DATA_W_ADDR_END: u32 = (INPUT_ADDR + MAX_INPUT_SIZE - 1) as u32 >> MEM_BYTES_BITS;

const OFFSET_USE_FLAG: u32 = 0x8000_0000;
const OFFSET_VALUE_MASK: u32 = 0x7FFF_FFFF;

#[allow(clippy::assertions_on_constants)]
const _: () = {
    assert!(
        INPUT_ADDR + MAX_INPUT_SIZE - 1 <= 0xFFFF_FFFF,
        "INPUT_DATA memory exceeds the 32-bit addressable range"
    );
    assert!(
        (MAX_INPUT_SIZE - 1) <= (1024 << 20),
        "INPUT_DATA is too large. Input size must be <= 1024MB"
    );
};

/// Lane layout of the `InputData` trace, read from the generated row so it always
/// follows `lanes_x_row` in `state-machines/mem/pil/mem.pil`.
#[inline]
fn lanes_of<F: PrimeField64, R: InputDataTraceRowOps<F>>() -> MemLanes {
    MemLanes::new(R::default().get_all_addr().len())
}

/// One padding lane: the last lane repeated, not selected and with no address change. Kept in one
/// place so the partial row and the whole rows cannot drift apart.
///
/// Every column of the row is written here, which is what lets the whole rows be filled by copying
/// one built row rather than by setting each column of each lane.
#[inline]
fn set_input_data_padding_lane<F: PrimeField64, R: InputDataTraceRowOps<F>>(
    row: &mut R,
    lane: usize,
    addr: u32,
    step: u64,
    is_free_read: bool,
    value_words: &[u16; 4],
) {
    row.set_addr(lane, addr);
    row.set_step(lane, step);
    row.set_sel(lane, false);
    row.set_is_free_read(lane, is_free_read);
    row.set_addr_changes(lane, false);
    for (index, &word) in value_words.iter().enumerate() {
        row.set_value_word(lane, index, word);
    }
}

/// The scalars an InputData instance's air values are built from: the last lane and the padding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InputDataFillOutput {
    last_addr: u32,
    last_step: u64,
    last_value: u64,
    padding_size: u32,
}

impl InputDataFillOutput {
    fn scalars(
        &self,
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
    ) -> [u64; 9] {
        [
            usize::from(segment_id) as u64,
            is_last_segment as u64,
            previous_segment.addr as u64,
            previous_segment.step,
            previous_segment.value,
            self.last_addr as u64,
            self.last_step,
            self.last_value,
            self.padding_size as u64,
        ]
    }

    fn air_values<F: PrimeField64>(
        &self,
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
    ) -> InputDataAirValues<'static, F> {
        let mut air_values = InputDataAirValues::<F>::new();
        air_values.segment_id = F::from_usize(segment_id.into());
        air_values.is_first_segment = F::from_bool(segment_id == 0);
        air_values.is_last_segment = F::from_bool(is_last_segment);
        air_values.previous_segment_step = F::from_u64(previous_segment.step);
        air_values.previous_segment_addr = F::from_u32(previous_segment.addr);
        air_values.segment_last_addr = F::from_u32(self.last_addr);
        air_values.segment_last_step = F::from_u64(self.last_step);
        air_values.previous_segment_value[0] = F::from_u32(previous_segment.value as u32);
        air_values.previous_segment_value[1] = F::from_u32((previous_segment.value >> 32) as u32);
        air_values.segment_last_value[0] = F::from_u32(self.last_value as u32);
        air_values.segment_last_value[1] = F::from_u32((self.last_value >> 32) as u32);
        let distance_base = previous_segment.addr - INPUT_DATA_W_ADDR_INIT;
        let distance_end = INPUT_DATA_W_ADDR_END - self.last_addr;
        air_values.distance_base[0] = F::from_u16(distance_base as u16);
        air_values.distance_base[1] = F::from_u16((distance_base >> 16) as u16);
        air_values.distance_end[0] = F::from_u16(distance_end as u16);
        air_values.distance_end[1] = F::from_u16((distance_end >> 16) as u16);
        air_values
    }
}

pub struct InputDataSM<F: PrimeField64> {
    _phantom: std::marker::PhantomData<F>,
}

#[allow(unused, unused_variables)]
impl<F: PrimeField64> InputDataSM<F> {
    /// Takes no `Std`: the only thing this machine used it for was range checks, which the prover
    /// now computes from the committed trace.
    pub fn new() -> Arc<Self> {
        Arc::new(Self { _phantom: std::marker::PhantomData })
    }
    fn get_u16_values(&self, value: u64) -> [u16; 4] {
        [value as u16, (value >> 16) as u16, (value >> 32) as u16, (value >> 48) as u16]
    }
    pub fn get_from_addr() -> u32 {
        INPUT_ADDR as u32
    }
    pub fn get_to_addr() -> u32 {
        (INPUT_ADDR + MAX_INPUT_SIZE - 1) as u32
    }

    #[cfg(feature = "debug_mem")]
    pub fn save_to_file<R: InputDataTraceRowOps<F>>(trace: &InputDataTrace<R>, file_name: &str) {
        println!("[MemDebug] writing information {} .....", file_name);
        let file = File::create(file_name).unwrap();
        let mut writer = BufWriter::new(file);
        let num_rows = InputDataTrace::<R>::NUM_ROWS;
        let lanes = lanes_of::<F, R>();

        for i in 0..num_rows {
            for lane in 0..lanes.lanes() {
                let addr = trace[i].get_addr(lane) as u64 * 8;
                let step = trace[i].get_step(lane);
                let main_step = MemHelpers::mem_step_to_main_step(step);
                let values = [
                    trace[i].get_value_word(lane, 0),
                    trace[i].get_value_word(lane, 1),
                    trace[i].get_value_word(lane, 2),
                    trace[i].get_value_word(lane, 3),
                ];
                let value = values[0] as u64
                    | ((values[1] as u64) << 16)
                    | ((values[2] as u64) << 32)
                    | ((values[3] as u64) << 48);
                let addr_changes = trace[i].get_addr_changes(lane);
                let is_free_read = trace[i].get_is_free_read(lane);
                let sel = trace[i].get_sel(lane);
                writeln!(
                    writer,
                    "{i:<8}.{lane} {addr:#010X} {step:>13} {main_step:>12} {values:?} 0x{value:016X} AC:{addr_changes} S:{sel} FR:{is_free_read}"
                )
                .unwrap();
            }
        }
        println!("[MemDebug] done");
    }

    #[cfg(feature = "debug_mem")]
    pub fn save_addr_offsets_to_file<R: InputDataTraceRowOps<F>>(
        trace: &InputDataTrace<R>,
        file_name: &str,
    ) {
        println!("[InputDataDebug] saving address offsets to {} .....", file_name);
        let file = std::fs::File::create(file_name).unwrap();
        let mut writer = std::io::BufWriter::new(file);
        let num_rows = InputDataTrace::<R>::NUM_ROWS;
        let lanes = lanes_of::<F, R>();

        let mut last_addr = u32::MAX;
        // `islot` is the virtual row: the offsets table is expressed in these units.
        for islot in 0..lanes.slots(num_rows) {
            let (row, lane) = lanes.split(islot);
            let addr = trace[row].get_addr(lane);
            if addr != last_addr {
                writeln!(writer, "0x{:08X} {islot}", addr * 8).unwrap();
                last_addr = addr;
            }
        }
        writeln!(writer).unwrap();
        println!("[InputDataDebug] done");
    }
    /// Fills the witness trace from a **sorted** input slice (legacy path).
    ///
    /// `mem_ops` must be sorted by `(addr, step)` before this method is called.
    /// Virtual rows are written sequentially: each operation is assigned the next
    /// available lane in declaration order, so the trace is filled from top to
    /// bottom (lane 0 .. lanes_x_row - 1 of each row) with no random-access
    /// indexing.
    ///
    /// Use this path when the GPU / planning stage is disabled
    /// (`legacy_mem_count_and_plan` feature flag) and the CPU planner provides
    /// pre-sorted inputs instead of offset tables.
    fn legacy_compute_witness(
        &self,
        mem_ops: MemOps<'_>,
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<AirInstance<F>> {
        if packed {
            self.legacy_compute_witness_inner::<InputDataTraceRowPacked<F>>(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
            )
        } else {
            self.legacy_compute_witness_inner::<InputDataTraceRow<F>>(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
            )
        }
    }
    fn legacy_compute_witness_inner<R: InputDataTraceRowOps<F>>(
        &self,
        mem_ops: MemOps<'_>,
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = InputDataTrace::<R>::new_from_vec(trace_buffer)?;

        let lanes = lanes_of::<F, R>();
        let num_slots = lanes.slots(InputDataTrace::<R>::NUM_ROWS);
        debug_assert!(
            !mem_ops.is_empty() && mem_ops.len() <= num_slots,
            "InputDataSM: mem_ops.len()={} out of range {}",
            mem_ops.len(),
            num_slots
        );

        let distance_base = previous_segment.addr - INPUT_DATA_W_ADDR_INIT;
        let mut last_addr: u32 = previous_segment.addr;
        let mut last_step: u64 = previous_segment.step;
        let mut last_value: u64 = previous_segment.value;
        // `i` is a virtual row: `lanes.split(i)` gives the physical row and the lane inside it.
        let mut i = 0;

        for mem_op in mem_ops.iter() {
            let distance = mem_op.addr - last_addr;

            if i >= num_slots {
                break;
            }

            if distance > SEGMENT_ADDR_MAX_DISTANCE as u32 {
                let mut internal_reads = (distance - 1) / SEGMENT_ADDR_MAX_DISTANCE as u32;

                let incomplete = (i + internal_reads as usize) >= num_slots;
                if incomplete {
                    internal_reads = (num_slots - i) as u32;
                }

                let (row, lane) = lanes.split(i);
                trace[row].set_addr_changes(lane, true);
                last_addr += SEGMENT_ADDR_MAX_DISTANCE as u32;
                trace[row].set_addr(lane, last_addr);

                // the step, value of internal reads isn't relevant
                last_step = 0;
                trace[row].set_step(lane, 0);
                trace[row].set_sel(lane, false);
                trace[row].set_is_free_read(lane, false);

                // setting value to zero, is not relevant for internal reads
                last_value = 0;
                for j in 0..4 {
                    trace[row].set_value_word(lane, j, 0);
                }
                i += 1;

                for _j in 1..internal_reads {
                    // same lane content as the previous internal read, only the address moves on
                    let (row, lane) = lanes.split(i);
                    last_addr += SEGMENT_ADDR_MAX_DISTANCE as u32;
                    trace[row].set_addr(lane, last_addr);
                    trace[row].set_addr_changes(lane, true);
                    trace[row].set_step(lane, 0);
                    trace[row].set_sel(lane, false);
                    trace[row].set_is_free_read(lane, false);
                    for j in 0..4 {
                        trace[row].set_value_word(lane, j, 0);
                    }

                    i += 1;
                }
                if incomplete {
                    break;
                }
            }

            let (row, lane) = lanes.split(i);
            trace[row].set_addr(lane, mem_op.addr);
            trace[row].set_step(lane, mem_op.step);
            trace[row].set_sel(lane, true);
            trace[row].set_is_free_read(lane, mem_op.addr == INPUT_DATA_W_ADDR_INIT);

            let value = mem_op.value;
            let value_words = self.get_u16_values(value);
            for (j, value) in value_words.iter().enumerate() {
                trace[row].set_value_word(lane, j, *value);
            }

            let addr_changes = last_addr != mem_op.addr;
            if addr_changes {
                trace[row].set_addr_changes(lane, true);
            } else {
                trace[row].set_addr_changes(lane, false);
            }

            last_addr = mem_op.addr;
            last_step = mem_op.step;
            last_value = mem_op.value;
            i += 1;
        }
        let count = i;

        // STEP3. Add dummy lanes to the output vector to fill the remaining virtual rows
        //PADDING: At end of memory fill with same addr, incrementing step, same value, sel = 0
        let (last_row, last_lane) = lanes.split(count - 1);
        let addr = trace[last_row].get_addr(last_lane);
        let is_free_read = last_addr == INPUT_DATA_W_ADDR_INIT;

        let padding_size = num_slots - count;
        let last_value_word = [
            trace[last_row].get_value_word(last_lane, 0),
            trace[last_row].get_value_word(last_lane, 1),
            trace[last_row].get_value_word(last_lane, 2),
            trace[last_row].get_value_word(last_lane, 3),
        ];
        for islot in count..num_slots {
            last_step += 1;

            let (row, lane) = lanes.split(islot);
            trace[row].set_addr(lane, addr);
            trace[row].set_step(lane, last_step);
            trace[row].set_sel(lane, false);
            for (j, &word) in last_value_word.iter().enumerate() {
                trace[row].set_value_word(lane, j, word);
            }
            trace[row].set_is_free_read(lane, is_free_read);

            trace[row].set_addr_changes(lane, false);

            // address doesn't change in padding lanes, no range check is required
        }

        let distance_end = INPUT_DATA_W_ADDR_END - last_addr;

        let mut air_values = InputDataAirValues::<F>::new();
        air_values.segment_id = F::from_usize(segment_id.into());
        air_values.is_first_segment = F::from_bool(segment_id == 0);
        air_values.is_last_segment = F::from_bool(is_last_segment);
        air_values.previous_segment_step = F::from_u64(previous_segment.step);
        air_values.previous_segment_addr = F::from_u32(previous_segment.addr);
        air_values.segment_last_addr = F::from_u32(last_addr);
        air_values.segment_last_step = F::from_u64(last_step);

        air_values.previous_segment_value[0] = F::from_u32(previous_segment.value as u32);
        air_values.previous_segment_value[1] = F::from_u32((previous_segment.value >> 32) as u32);

        air_values.segment_last_value[0] = F::from_u32(last_value as u32);
        air_values.segment_last_value[1] = F::from_u32((last_value >> 32) as u32);

        let distance_base = [distance_base as u16, (distance_base >> 16) as u16];
        let distance_end = [distance_end as u16, (distance_end >> 16) as u16];

        air_values.distance_base[0] = F::from_u16(distance_base[0]);
        air_values.distance_base[1] = F::from_u16(distance_base[1]);

        air_values.distance_end[0] = F::from_u16(distance_end[0]);
        air_values.distance_end[1] = F::from_u16(distance_end[1]);

        #[cfg(feature = "debug_mem")]
        {
            let path = env::var("MEM_TRACE_DIR").unwrap_or("tmp/mem_trace".to_string());
            let filename = format!("{path}/input_data_legacy_trace_{segment_id:04}.txt");
            println!("Saving {filename}");
            Self::save_to_file(&trace, &filename);
            println!("[Mem:{}] mem_ops:{} padding:{}", segment_id, mem_ops.len(), padding_size);
        }

        #[cfg(feature = "debug_mem")]
        Self::save_addr_offsets_to_file(
            &trace,
            &format!("tmp/input_data_trace_{segment_id:04}_offsets.txt"),
        );

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values)))
    }

    #[allow(clippy::too_many_arguments)]
    fn compute_witness_with_offsets(
        &self,
        mem_ops: MemOps<'_>,
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
        packed: bool,
        seg: &MemModuleSegmentCheckPoint,
    ) -> ProofmanResult<AirInstance<F>> {
        if packed {
            // `ZISK_MEM_GPU_FILL=arena-check`: the device rows against the CPU rows, every word
            // and scalar; the CPU rows are proved.
            let seg_idx = usize::from(segment_id);
            let check = gpu_fill_mode() == GpuFillMode::ArenaCheck;
            let used_bits = Self::packed_used_bits();
            let mut hook = move |cpu: &[u64], out: &InputDataFillOutput| {
                let n_rows = InputDataTrace::<InputDataTraceRowPacked<F>>::NUM_ROWS;
                let words_per_row = InputDataTraceRowPacked::<F>::PACKED_WORDS;
                let mut gpu = vec![0u64; n_rows * words_per_row];
                match Self::device_rows(&mut gpu, n_rows, segment_id) {
                    Ok((report, prev)) => {
                        let (count, first) = crate::mem_gpu_fill::compare_rows_masked(
                            cpu,
                            &gpu,
                            words_per_row,
                            used_bits,
                        );
                        let scalars_ok = report == *out
                            && prev.addr == previous_segment.addr
                            && prev.step == previous_segment.step
                            && prev.value == previous_segment.value;
                        tracing::info!(
                            "InputData[{seg_idx}] arena CHECK: {} words differ{} | scalars {}{}",
                            count,
                            first
                                .map(|(row, w, c, g)| format!(
                                    ", first row {row} word {w}: cpu {c:#x} gpu {g:#x}"
                                ))
                                .unwrap_or_default(),
                            if scalars_ok { "match" } else { "DIFFER" },
                            if scalars_ok {
                                String::new()
                            } else {
                                format!(
                                    " (cpu prev {previous_segment:?} {out:?}; gpu prev {prev:?} {report:?})"
                                )
                            }
                        );
                    }
                    Err(e) => tracing::warn!("InputData[{seg_idx}] arena CHECK unavailable: {e}"),
                }
            };
            self.compute_witness_with_offsets_inner::<InputDataTraceRowPacked<F>>(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
                seg,
                if check { Some(&mut hook) } else { None },
                used_bits,
            )
        } else {
            self.compute_witness_with_offsets_inner::<InputDataTraceRow<F>>(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
                seg,
                None,
                std::mem::size_of::<InputDataTraceRow<F>>() * 8,
            )
        }
    }

    /// The bits a packed InputData row uses.
    fn packed_used_bits() -> usize {
        crate::mem_gpu_fill::packed_used_bits(
            InputDataTrace::<()>::AIRGROUP_ID,
            InputDataTrace::<()>::AIR_ID,
        )
        .unwrap_or(InputDataTraceRowPacked::<F>::PACKED_WORDS * 64)
    }

    /// The rows of InputData instance `segment_id` from the GPU planner, with the fill's scalars
    /// and the lane before the instance.
    fn device_rows(
        rows: &mut [u64],
        n_rows: usize,
        segment_id: SegmentId,
    ) -> Result<(InputDataFillOutput, MemPreviousSegment), String> {
        let res = zisk_sm_mem_planner::gpu_input_witness_fill(
            usize::from(segment_id) as u32,
            rows,
            n_rows as u32,
        )?;
        let num_slots = n_rows * zisk_sm_mem_common::input_data_lanes_x_row();
        let out = InputDataFillOutput {
            last_addr: res.last_addr_w,
            last_step: res.last_step,
            last_value: res.last_value,
            padding_size: (num_slots - res.n_lanes as usize) as u32,
        };
        let prev = MemPreviousSegment {
            addr: res.prev_addr_w,
            step: res.prev_step,
            value: res.prev_value,
        };
        Ok((out, prev))
    }
    /// Fills the witness trace using a precomputed **offset table** (GPU path).
    ///
    /// `mem_ops` does not need to be sorted. Each operation is placed directly
    /// into the virtual row indicated by the `offsets` table, enabling
    /// random-access filling in a single pass.
    ///
    /// # Offset table layout
    ///
    /// The table is expressed in **virtual rows**: with `lanes_x_row` lanes per
    /// physical row, the virtual row `v` is the lane `v % lanes_x_row` of the
    /// physical row `v / lanes_x_row` (see [`MemLanes`]).
    ///
    /// `offset_base_addr` is the byte address of the first qword slot
    /// (i.e. the byte address of `offsets[0]`).  For every qword address
    /// `A = (offset_base_addr >> 3) + i` that falls inside this segment:
    ///
    /// * `offsets[i] = 0` — **halo slot**: address `A` belongs to the
    ///   previous segment (`previous_segment`).  Only slot 0 of a non-first
    ///   segment can be 0.
    /// * `offsets[i] = v + 1` — address `A` first appears at virtual row `v`
    ///   (1-based so that 0 is unambiguously the halo).
    /// * Addresses **absent** from this instance are forward-filled: the slot
    ///   for a missing address inherits the value of the *next* present
    ///   address's slot.  Consequently, when traversing `offsets` in ascending
    ///   index order, the first absent address is the one where
    ///   `offsets[i] == offsets[i + 1]` (no increment between consecutive
    ///   slots).
    ///
    /// `on_filled` sees the filled rows (as words) and the fill's scalars before the air instance
    /// is built: the arena check compares them with the device rows.
    #[allow(clippy::too_many_arguments)]
    fn compute_witness_with_offsets_inner<R: InputDataTraceRowOps<F>>(
        &self,
        mem_ops: MemOps<'_>,
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
        seg: &MemModuleSegmentCheckPoint,
        on_filled: crate::mem_gpu_fill::OnFilled<'_, InputDataFillOutput>,
        used_bits: usize,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = InputDataTrace::<R>::new_from_vec(trace_buffer)?;

        let lanes = lanes_of::<F, R>();
        let num_slots = lanes.slots(InputDataTrace::<R>::NUM_ROWS);
        debug_assert!(
            !mem_ops.is_empty() && mem_ops.len() <= num_slots,
            "InputDataSM: mem_ops.len()={} out of range {}",
            mem_ops.len(),
            num_slots
        );

        // `current_offsets` packs a 1-based virtual row plus a flag bit in a u32.
        debug_assert!(
            num_slots < OFFSET_USE_FLAG as usize,
            "InputDataSM: {num_slots} virtual rows do not fit in OFFSET_VALUE_MASK"
        );

        let mut current_offsets = vec![0u32; seg.addr_range_slots as usize];

        #[cfg(feature = "debug_mem")]
        let mut filled_slots = vec![false; num_slots];
        let offset_base_addr_w = seg.offsets_base_addr >> 3;

        // first address == halo
        // In input data, first special address not active flag change address if has free inputs
        // on special address INPUT_ADDR, first position of input data.
        if seg.offset_at(0) == 0 || seg.offsets_base_addr == INPUT_ADDR as u32 {
            current_offsets[0] = OFFSET_USE_FLAG;
        }

        for (index, mem_op) in mem_ops.iter().enumerate() {
            let addr_index = (mem_op.addr - offset_base_addr_w) as usize;
            let addr_changes = current_offsets[addr_index] == 0;

            // `islot` is a virtual row: the offsets table is expressed in these units, so the
            // physical row and the lane inside it come from `lanes.split(islot)`.
            let islot = if addr_changes {
                let offset = seg.offset_at(addr_index as u32);
                current_offsets[addr_index] = offset | OFFSET_USE_FLAG;
                offset as usize - 1
            } else {
                let offset = current_offsets[addr_index];
                current_offsets[addr_index] = offset + 1;
                (offset & OFFSET_VALUE_MASK) as usize
            };
            let (row, lane) = lanes.split(islot);
            #[cfg(feature = "debug_mem")]
            {
                assert!(!filled_slots[islot],"InputDataSM: overwriting non empty slot {islot} at index {index} for mem_op with addr 0x{:X} => 0x{:X} step:{} => {}",
                    trace[row].get_addr(lane) * 8, mem_op.addr * 8, trace[row].get_step(lane), mem_op.step);
                filled_slots[islot] = true;
            }

            trace[row].set_addr(lane, mem_op.addr);
            trace[row].set_step(lane, mem_op.step);
            trace[row].set_sel(lane, true);
            trace[row].set_is_free_read(lane, mem_op.addr == INPUT_DATA_W_ADDR_INIT);

            let value_words = self.get_u16_values(mem_op.value);

            trace[row].set_value_word(lane, 0, value_words[0]);
            trace[row].set_value_word(lane, 1, value_words[1]);
            trace[row].set_value_word(lane, 2, value_words[2]);
            trace[row].set_value_word(lane, 3, value_words[3]);

            if addr_changes {
                trace[row].set_addr_changes(lane, true);
                let previous_addr = seg
                    .previous_change_addr_w(addr_index as u32)
                    .unwrap_or(previous_segment.addr as u64);
                let distance = mem_op.addr as i64 - previous_addr as i64 - 1;
            } else {
                trace[row].set_addr_changes(lane, false);
            }
        }

        // STEP3. Add dummy lanes to the output vector to fill the remaining virtual rows
        //PADDING: At end of memory fill with same addr, incrementing step, same value, sel = 0
        let count = mem_ops.len();
        let (last_row, last_lane) = lanes.split(count - 1);

        #[cfg(feature = "debug_mem")]
        {
            let mut prev_filled_slot = filled_slots[0];
            let mut from_slot = 0;
            let _count = if is_last_segment { count } else { num_slots };
            for (i, &filled_slot) in filled_slots.iter().enumerate().take(_count) {
                debug_assert!(
                    filled_slot == prev_filled_slot,
                    "InputDataSM: not complete instance found [{}..{}] = {}",
                    from_slot,
                    i - 1,
                    prev_filled_slot
                );
            }
        }
        let last_addr = trace[last_row].get_addr(last_lane);
        let last_step = trace[last_row].get_step(last_lane);
        let is_free_read = last_addr == INPUT_DATA_W_ADDR_INIT;

        let value_0 = trace[last_row].get_value_word(last_lane, 0);
        let value_1 = trace[last_row].get_value_word(last_lane, 1);
        let value_2 = trace[last_row].get_value_word(last_lane, 2);
        let value_3 = trace[last_row].get_value_word(last_lane, 3);

        let padding_size = num_slots - count;
        // Every padding lane holds the same values (the address does not change in them, so no
        // range check is required either), so only the row the last operation shares with the
        // padding is written lane by lane; the whole rows after it are one built row copied over
        // them in parallel, which is a row-sized move instead of a setter per column.
        if count < num_slots {
            let value_words = [value_0, value_1, value_2, value_3];
            let lanes_x_row = lanes.lanes();
            let partial_end = count.next_multiple_of(lanes_x_row).min(num_slots);
            for islot in count..partial_end {
                let (row, lane) = lanes.split(islot);
                set_input_data_padding_lane::<F, R>(
                    &mut trace[row],
                    lane,
                    last_addr,
                    last_step,
                    is_free_read,
                    &value_words,
                );
            }
            let from_row = partial_end / lanes_x_row;
            if from_row < InputDataTrace::<R>::NUM_ROWS {
                let mut pad_row = R::default();
                for lane in 0..lanes_x_row {
                    set_input_data_padding_lane::<F, R>(
                        &mut pad_row,
                        lane,
                        last_addr,
                        last_step,
                        is_free_read,
                        &value_words,
                    );
                }
                trace.buffer[from_row..].par_iter_mut().for_each(|row| *row = pad_row);
            }
        }

        let out = InputDataFillOutput {
            last_addr,
            last_step,
            last_value: value_0 as u64
                | ((value_1 as u64) << 16)
                | ((value_2 as u64) << 32)
                | ((value_3 as u64) << 48),
            padding_size: padding_size as u32,
        };
        {
            let words = crate::mem_trace_hash::rows_as_words(&trace.buffer);
            if let Some(hook) = on_filled {
                hook(words, &out);
            }
            crate::mem_trace_hash::dump_scalars(
                self.get_mem_name(),
                usize::from(segment_id),
                words,
                &out.scalars(segment_id, is_last_segment, previous_segment),
                std::mem::size_of::<R>() / 8,
                used_bits,
            );
        }
        let mut air_values = out.air_values::<F>(segment_id, is_last_segment, previous_segment);

        #[cfg(feature = "debug_mem")]
        {
            let path = env::var("MEM_TRACE_DIR").unwrap_or("tmp/mem_trace".to_string());
            let filename = format!("{path}/input_data_trace_{segment_id:04}.txt");
            println!("Saving {filename}");
            Self::save_to_file(&trace, &filename);
            println!("[Mem:{}] mem_ops:{} padding:{}", segment_id, mem_ops.len(), padding_size);
        }

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values)))
    }
}

impl<F: PrimeField64> MemModule<F> for InputDataSM<F> {
    fn compute_witness_gpu_arena(
        &self,
        segment_id: SegmentId,
        is_last_segment: bool,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        if !packed {
            return Err(proofman_common::ProofmanError::InvalidParameters(
                "ZISK_MEM_GPU_FILL=arena needs the packed InputData trace".to_string(),
            ));
        }
        let seg_idx = usize::from(segment_id);
        // The device rows cover every row of the instance, padding included.
        let mut trace = InputDataTrace::<InputDataTraceRowPacked<F>>::new_from_vec(trace_buffer)?;
        let n_rows = trace.num_rows();
        let (out, previous_segment) = {
            let words = crate::mem_trace_hash::rows_as_words_mut(&mut trace.buffer);
            Self::device_rows(words, n_rows, segment_id)
        }
        .map_err(|e| {
            proofman_common::ProofmanError::InvalidParameters(format!(
                "InputData[{seg_idx}] witness from the GPU planner failed: {e}"
            ))
        })?;
        crate::mem_trace_hash::dump_scalars(
            self.get_mem_name(),
            seg_idx,
            crate::mem_trace_hash::rows_as_words(&trace.buffer),
            &out.scalars(segment_id, is_last_segment, &previous_segment),
            InputDataTraceRowPacked::<F>::PACKED_WORDS,
            Self::packed_used_bits(),
        );
        let mut air_values = out.air_values::<F>(segment_id, is_last_segment, &previous_segment);
        Ok(Some(AirInstance::new_from_trace(
            FromTrace::new(&mut trace).with_air_values(&mut air_values),
        )))
    }

    fn compute_witness_gpu_slot(
        &self,
        decl: &proofman_common::GpuWitnessAir,
        segment_id: SegmentId,
        is_last_segment: bool,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        use proofman_common::trace::TraceRow;
        if !packed {
            return Err(proofman_common::ProofmanError::InvalidParameters(
                "ZISK_MEM_GPU_FILL=slot needs the packed InputData trace".to_string(),
            ));
        }
        let seg_idx = usize::from(segment_id);
        let n_rows = InputDataTrace::<InputDataTraceRowPacked<F>>::NUM_ROWS;
        let res = zisk_sm_mem_planner::gpu_mem_witness_scalars(
            crate::mem_gpu_fill::SLOT_FAMILY_INPUT,
            seg_idx as u32,
        )
        .map_err(|e| {
            proofman_common::ProofmanError::InvalidParameters(format!(
                "InputData[{seg_idx}] scalars from the GPU planner failed: {e}"
            ))
        })?;
        let num_slots = n_rows * zisk_sm_mem_common::input_data_lanes_x_row();
        let out = InputDataFillOutput {
            last_addr: res.last_addr_w,
            last_step: res.last_step,
            last_value: res.last_value,
            padding_size: (num_slots - res.n_lanes as usize) as u32,
        };
        let previous_segment = MemPreviousSegment {
            addr: res.prev_addr_w,
            step: res.prev_step,
            value: res.prev_value,
        };
        let mut air_values = out.air_values::<F>(segment_id, is_last_segment, &previous_segment);
        crate::mem_gpu_fill::slot_instance(
            decl,
            crate::mem_gpu_fill::SLOT_FAMILY_INPUT,
            InputDataTrace::<()>::AIR_ID,
            seg_idx,
            n_rows,
            InputDataTraceRow::<F>::ROW_SIZE,
            trace_buffer,
            proofman_common::trace::Values::get_buffer(&mut air_values),
        )
        .map(Some)
    }

    fn get_addr_range(&self) -> (u32, u32) {
        (INPUT_DATA_W_ADDR_INIT, INPUT_DATA_W_ADDR_END)
    }
    fn is_dual(&self) -> bool {
        false
    }
    fn get_mem_name(&self) -> &str {
        "input"
    }
    fn is_initializable(&self) -> bool {
        false
    }

    /// Finalizes the witness accumulation process and triggers the proof generation.
    ///
    /// This method is invoked by the executor when no further witness data remains to be added.
    ///
    /// # Parameters
    ///
    /// - `mem_inputs`: A slice of all `ZiskRequiredMemory` inputs
    #[allow(clippy::too_many_arguments)]
    #[cfg_attr(feature = "legacy_mem_count_and_plan", allow(unused_variables))]
    #[inline(always)]
    fn compute_witness(
        &self,
        mem_ops: MemOps<'_>,
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
        packed: bool,
        seg: &MemModuleSegmentCheckPoint,
    ) -> ProofmanResult<AirInstance<F>> {
        #[cfg(not(feature = "legacy_mem_count_and_plan"))]
        {
            self.compute_witness_with_offsets(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
                packed,
                seg,
            )
        }
        #[cfg(feature = "legacy_mem_count_and_plan")]
        {
            self.legacy_compute_witness(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
                packed,
            )
        }
    }
}
