use std::sync::Arc;

use crate::{MemInput, MemModule, MemPreviousSegment};
use mem_common::{MemModuleSegmentCheckPoint, MEM_BYTES_BITS, SEGMENT_ADDR_MAX_RANGE};

use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
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
const MAX_RANGE_CHECK_CACHE: usize = 2048;

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

pub struct InputDataSM<F: PrimeField64> {
    /// PIL2 standard library
    std: Arc<Std<F>>,

    /// Range check ID
    range_id: usize,

    /// Range check ID for the 16-bit chunks of the input values
    range_16bits_id: usize,
}

#[allow(unused, unused_variables)]
impl<F: PrimeField64> InputDataSM<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let range_id = std
            .get_range_id(0, SEGMENT_ADDR_MAX_RANGE as i64, None)
            .expect("Failed to get range ID");
        let range_16bits_id =
            std.get_range_id(0, (1 << 16) - 1, None).expect("Failed to get range ID");

        Arc::new(Self { range_16bits_id, std: std.clone(), range_id })
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
    /// Fills the witness trace from a **sorted** input slice (legacy path).
    ///
    /// `mem_ops` must be sorted by `(addr, step)` before this method is called.
    /// Rows are written sequentially: each operation is assigned the next
    /// available row in declaration order, so the trace is filled from top to
    /// bottom with no random-access indexing.
    ///
    /// Use this path when the GPU / planning stage is disabled
    /// (`legacy_mem_count_and_plan` feature flag) and the CPU planner provides
    /// pre-sorted inputs instead of offset tables.
    fn legacy_compute_witness(
        &self,
        mem_ops: &[MemInput],
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
        mem_ops: &[MemInput],
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = InputDataTrace::<R>::new_from_vec(trace_buffer)?;

        let num_rows = InputDataTrace::<R>::NUM_ROWS;
        debug_assert!(
            !mem_ops.is_empty() && mem_ops.len() <= num_rows,
            "InputDataSM: mem_ops.len()={} out of range {}",
            mem_ops.len(),
            num_rows
        );

        let mut range_16bits: Vec<u32> = vec![0; 1 << 16];

        let mut max_range_distance_count = 0;

        let distance_base = previous_segment.addr - INPUT_DATA_W_ADDR_INIT;
        let mut last_addr: u32 = previous_segment.addr;
        let mut last_step: u64 = previous_segment.step;
        let mut last_value: u64 = previous_segment.value;
        let mut i = 0;

        for mem_op in mem_ops.iter() {
            let distance = mem_op.addr - last_addr;

            if i >= num_rows {
                break;
            }

            if distance > SEGMENT_ADDR_MAX_RANGE as u32 {
                let mut internal_reads = (distance - 1) / SEGMENT_ADDR_MAX_RANGE as u32;

                let incomplete = (i + internal_reads as usize) >= num_rows;
                if incomplete {
                    internal_reads = (num_rows - i) as u32;
                }

                trace[i].set_addr_changes(true);
                last_addr += SEGMENT_ADDR_MAX_RANGE as u32;
                max_range_distance_count += 1;
                trace[i].set_addr(last_addr);

                // the step, value of internal reads isn't relevant
                last_step = 0;
                trace[i].set_step(0);
                trace[i].set_sel(false);

                // setting value to zero, is not relevant for internal reads
                last_value = 0;
                for j in 0..4 {
                    trace[i].set_value_word(j, 0);
                }
                i += 1;

                for _j in 1..internal_reads {
                    trace[i] = trace[i - 1];
                    last_addr += SEGMENT_ADDR_MAX_RANGE as u32;
                    max_range_distance_count += 1;
                    trace[i].set_addr(last_addr);

                    i += 1;
                }
                range_16bits[0] += 4 * internal_reads;
                if incomplete {
                    break;
                }
            }

            trace[i].set_addr(mem_op.addr);
            trace[i].set_step(mem_op.step);
            trace[i].set_sel(true);
            trace[i].set_is_free_read(mem_op.addr == INPUT_DATA_W_ADDR_INIT);

            let value = mem_op.value;
            let value_words = self.get_u16_values(value);
            for j in 0..4 {
                range_16bits[value_words[j] as usize] += 1;
                trace[i].set_value_word(j, value_words[j]);
            }

            let addr_changes = last_addr != mem_op.addr;
            if addr_changes {
                trace[i].set_addr_changes(true);
                self.std.range_check(self.range_id, (mem_op.addr - last_addr - 1) as i64, 1);
            } else {
                trace[i].set_addr_changes(false);
            }

            last_addr = mem_op.addr;
            last_step = mem_op.step;
            last_value = mem_op.value;
            i += 1;
        }
        let count = i;

        // STEP3. Add dummy rows to the output vector to fill the remaining rows
        //PADDING: At end of memory fill with same addr, incrementing step, same value, sel = 0
        let last_row_idx = count - 1;
        let addr = trace[last_row_idx].get_addr();
        let is_free_read = last_addr == INPUT_DATA_W_ADDR_INIT;

        let padding_size = num_rows - count;
        for i in count..num_rows {
            last_step += 1;

            trace[i].set_addr(addr);
            trace[i].set_step(last_step);
            trace[i].set_sel(false);
            for j in 0..4 {
                let value = trace[last_row_idx].get_value_word(j);
                trace[i].set_value_word(j, value);
            }
            trace[i].set_is_free_read(is_free_read);

            trace[i].set_addr_changes(false);

            // address doesn't change in padding rows, no range check is required
        }

        let distance_end = INPUT_DATA_W_ADDR_END - last_addr;

        self.std.range_check(
            self.range_id,
            SEGMENT_ADDR_MAX_RANGE as i64,
            max_range_distance_count,
        );

        // range of chunks
        for j in 0..4 {
            let value = trace[last_row_idx].get_value_word(j);
            range_16bits[value as usize] += padding_size as u32;
        }

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

        range_16bits[distance_base[0] as usize] += 1;
        range_16bits[distance_base[1] as usize] += 1;
        range_16bits[distance_end[0] as usize] += 1;
        range_16bits[distance_end[1] as usize] += 1;

        self.std.range_checks(self.range_16bits_id, range_16bits);

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values)))
    }

    fn compute_witness_with_offsets(
        &self,
        mem_ops: &[MemInput],
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
        packed: bool,
        seg: &MemModuleSegmentCheckPoint,
    ) -> ProofmanResult<AirInstance<F>> {
        if packed {
            self.compute_witness_with_offsets_inner::<InputDataTraceRowPacked<F>>(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
                seg,
            )
        } else {
            self.compute_witness_with_offsets_inner::<InputDataTraceRow<F>>(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
                seg,
            )
        }
    }
    /// Fills the witness trace using a precomputed **offset table** (GPU path).
    ///
    /// `mem_ops` does not need to be sorted. Each operation is placed directly
    /// into the trace row indicated by the `offsets` table, enabling
    /// random-access filling in a single pass.
    ///
    /// # Offset table layout
    ///
    /// `offset_base_addr` is the byte address of the first qword slot
    /// (i.e. the byte address of `offsets[0]`).  For every qword address
    /// `A = (offset_base_addr >> 3) + i` that falls inside this segment:
    ///
    /// * `offsets[i] = 0` — **halo slot**: address `A` belongs to the
    ///   previous segment (`previous_segment`).  Only slot 0 of a non-first
    ///   segment can be 0.
    /// * `offsets[i] = r + 1` — address `A` first appears at trace row `r`
    ///   (1-based so that 0 is unambiguously the halo).
    /// * Addresses **absent** from this instance are forward-filled: the slot
    ///   for a missing address inherits the value of the *next* present
    ///   address's slot.  Consequently, when traversing `offsets` in ascending
    ///   index order, the first absent address is the one where
    ///   `offsets[i] == offsets[i + 1]` (no increment between consecutive
    ///   slots).
    #[allow(clippy::too_many_arguments)]
    fn compute_witness_with_offsets_inner<R: InputDataTraceRowOps<F>>(
        &self,
        mem_ops: &[MemInput],
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
        seg: &MemModuleSegmentCheckPoint,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = InputDataTrace::<R>::new_from_vec(trace_buffer)?;

        let num_rows = InputDataTrace::<R>::NUM_ROWS;
        debug_assert!(
            !mem_ops.is_empty() && mem_ops.len() <= num_rows,
            "InputDataSM: mem_ops.len()={} out of range {}",
            mem_ops.len(),
            num_rows
        );

        let mut current_offsets = vec![0u32; seg.addr_range_slots as usize];
        let mut range_16bits: Vec<u32> = vec![0; 1 << 16];
        let mut range_check_cache = vec![0u32; MAX_RANGE_CHECK_CACHE];

        #[cfg(feature = "debug_mem")]
        let mut filled_rows = vec![false; trace.num_rows()];
        let offset_base_addr_w = seg.offsets_base_addr >> 3;

        // first address == halo
        // In input data, first special address not active flag change address.
        current_offsets[0] = OFFSET_USE_FLAG;

        for (index, mem_op) in mem_ops.iter().enumerate() {
            let addr_index = (mem_op.addr - offset_base_addr_w) as usize;
            let addr_changes = current_offsets[addr_index] == 0;

            let irow = if addr_changes {
                let offset = seg.offset_at(addr_index as u32);
                current_offsets[addr_index] = offset | OFFSET_USE_FLAG;
                offset as usize - 1
            } else {
                let offset = current_offsets[addr_index];
                current_offsets[addr_index] = offset + 1;
                (offset & OFFSET_VALUE_MASK) as usize
            };
            #[cfg(feature = "debug_mem")]
            {
                assert!(!filled_rows[irow],"InputDataSM: overriting non empty row {irow} at index {index} for mem_op with addr 0x{:X} => 0x{:X} step:{} => {}",
                    trace[irow].get_addr() * 8, mem_op.addr * 8, trace[irow].get_step(), mem_op.step);
                filled_rows[irow] = true;
            }

            trace[irow].set_addr(mem_op.addr);
            trace[irow].set_step(mem_op.step);
            trace[irow].set_sel(true);
            trace[irow].set_is_free_read(mem_op.addr == INPUT_DATA_W_ADDR_INIT);

            let value_words = self.get_u16_values(mem_op.value);

            range_16bits[value_words[0] as usize] += 1;
            range_16bits[value_words[1] as usize] += 1;
            range_16bits[value_words[2] as usize] += 1;
            range_16bits[value_words[3] as usize] += 1;
            trace[irow].set_value_word(0, value_words[0]);
            trace[irow].set_value_word(1, value_words[1]);
            trace[irow].set_value_word(2, value_words[2]);
            trace[irow].set_value_word(3, value_words[3]);

            if addr_changes {
                trace[irow].set_addr_changes(true);
                let previous_addr = seg.previous_change_addr_w(addr_index as u32)
                    .unwrap_or(previous_segment.addr as u64);
                let distance = mem_op.addr as i64 - previous_addr as i64 - 1;
                if distance < MAX_RANGE_CHECK_CACHE as i64 {
                    range_check_cache[distance as usize] += 1;
                } else {
                    self.std.range_check(self.range_id, distance, 1);
                }
            } else {
                trace[irow].set_addr_changes(false);
            }
        }

        // STEP3. Add dummy rows to the output vector to fill the remaining rows
        //PADDING: At end of memory fill with same addr, incrementing step, same value, sel = 0
        let count = mem_ops.len();
        let last_row = trace[count - 1];

        #[cfg(feature = "debug_mem")]
        {
            let mut prev_filled_row = filled_rows[0];
            let mut from_row = 0;
            let _count = if is_last_segment { count } else { trace.num_rows() };
            for i in 0.._count {
                debug_assert!(
                    filled_rows[i] == prev_filled_row,
                    "InputDataSM: not complete instance found [{}..{}] = {}",
                    from_row,
                    i - 1,
                    prev_filled_row
                );
            }
        }
        let last_addr = last_row.get_addr();
        let is_free_read = last_addr == INPUT_DATA_W_ADDR_INIT;

        let padding_size = num_rows - count;
        if padding_size > 0 {
            trace[count] = last_row;
            trace[count].set_sel(false);
            trace[count].set_is_free_read(is_free_read);
            trace[count].set_addr_changes(false);

            for i in count + 1..num_rows {
                trace[i] = trace[i - 1];
            }
            // address doesn't change in padding rows, no range check is required
        }

        let value_0 = last_row.get_value_word(0);
        let value_1 = last_row.get_value_word(1);
        let value_2 = last_row.get_value_word(2);
        let value_3 = last_row.get_value_word(3);

        range_16bits[value_0 as usize] += padding_size as u32;
        range_16bits[value_1 as usize] += padding_size as u32;
        range_16bits[value_2 as usize] += padding_size as u32;
        range_16bits[value_3 as usize] += padding_size as u32;

        self.std.range_checks(self.range_id, range_check_cache);

        let mut air_values = InputDataAirValues::<F>::new();
        air_values.segment_id = F::from_usize(segment_id.into());
        air_values.is_first_segment = F::from_bool(segment_id == 0);
        air_values.is_last_segment = F::from_bool(is_last_segment);
        air_values.previous_segment_step = F::from_u64(previous_segment.step);
        air_values.previous_segment_addr = F::from_u32(previous_segment.addr);
        air_values.segment_last_addr = F::from_u32(last_row.get_addr());
        air_values.segment_last_step = F::from_u64(last_row.get_step());

        air_values.previous_segment_value[0] = F::from_u32(previous_segment.value as u32);
        air_values.previous_segment_value[1] = F::from_u32((previous_segment.value >> 32) as u32);

        air_values.segment_last_value[0] = F::from_u32(value_0 as u32 + ((value_1 as u32) << 16));
        air_values.segment_last_value[1] = F::from_u32(value_2 as u32 + ((value_3 as u32) << 16));

        let distance_end = (INPUT_DATA_W_ADDR_END - last_addr) as i64;
        let distance_base = previous_segment.addr - INPUT_DATA_W_ADDR_INIT;

        let distance_base_chunks = [distance_base as u16, (distance_base >> 16) as u16];
        let distance_end_chunks = [distance_end as u16, (distance_end >> 16) as u16];

        air_values.distance_base[0] = F::from_u16(distance_base_chunks[0]);
        air_values.distance_base[1] = F::from_u16(distance_base_chunks[1]);

        air_values.distance_end[0] = F::from_u16(distance_end_chunks[0]);
        air_values.distance_end[1] = F::from_u16(distance_end_chunks[1]);

        range_16bits[distance_base_chunks[0] as usize] += 1;
        range_16bits[distance_base_chunks[1] as usize] += 1;
        range_16bits[distance_end_chunks[0] as usize] += 1;
        range_16bits[distance_end_chunks[1] as usize] += 1;
        self.std.range_checks(self.range_16bits_id, range_16bits);

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values)))
    }
}

impl<F: PrimeField64> MemModule<F> for InputDataSM<F> {
    fn get_addr_range(&self) -> (u32, u32) {
        (INPUT_DATA_W_ADDR_INIT, INPUT_DATA_W_ADDR_END)
    }
    fn is_dual(&self) -> bool {
        false
    }
    fn get_mem_name(&self) -> &str {
        "input"
    }

    /// Finalizes the witness accumulation process and triggers the proof generation.
    ///
    /// This method is invoked by the executor when no further witness data remains to be added.
    ///
    /// # Parameters
    ///
    /// - `mem_inputs`: A slice of all `ZiskRequiredMemory` inputs
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    fn compute_witness(
        &self,
        mem_ops: &[MemInput],
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
