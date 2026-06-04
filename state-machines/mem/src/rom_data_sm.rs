use std::sync::Arc;

use crate::{mem_sm::MemPreviousSegment, MemInput, MemModule};
use fields::PrimeField64;
use mem_common::{MemHelpers, MemModuleSegmentCheckPoint, MEM_BYTES_BITS, SEGMENT_ADDR_MAX_RANGE};
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use std::{
    fs::File,
    io::{BufWriter, Write},
};
use zisk_common::SegmentId;
use zisk_core::{ROM_ADDR, ROM_ADDR_MAX};
use zisk_pil::{
    RomDataAirValues, RomDataTrace, RomDataTraceRow, RomDataTraceRowOps, RomDataTraceRowPacked,
};

pub const ROM_DATA_W_ADDR_INIT: u32 = ROM_ADDR as u32 >> MEM_BYTES_BITS;
pub const ROM_DATA_W_ADDR_END: u32 = ROM_ADDR_MAX as u32 >> MEM_BYTES_BITS;

const _: () = {
    assert!(ROM_ADDR_MAX <= 0xFFFF_FFFF, "ROM_DATA memory exceeds the 32-bit addressable range");
    assert!(
        (ROM_ADDR_MAX - ROM_ADDR) <= (128 << 20),
        "ROM_DATA is too large. ROM size must be <= 128MB"
    );
};

pub struct RomDataSM<F: PrimeField64> {
    /// PIL2 standard library
    std: Arc<Std<F>>,

    range_id: usize,
}

const OFFSET_USE_FLAG: u32 = 0x8000_0000;
const OFFSET_VALUE_MASK: u32 = 0x7FFF_FFFF;
const MAX_RANGE_CHECK_CACHE: usize = 2048;

#[allow(unused, unused_variables)]
impl<F: PrimeField64> RomDataSM<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let range_id = std
            .get_range_id(0, SEGMENT_ADDR_MAX_RANGE as i64, None)
            .expect("Failed to get range ID");

        Arc::new(Self { std: std.clone(), range_id })
    }
    pub fn get_from_addr() -> u32 {
        ROM_DATA_W_ADDR_INIT
    }
    fn get_u32_values(&self, value: u64) -> (u32, u32) {
        (value as u32, (value >> 32) as u32)
    }
    pub fn get_to_addr() -> u32 {
        ROM_DATA_W_ADDR_END
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
            self.legacy_compute_witness_inner::<RomDataTraceRowPacked<F>>(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
            )
        } else {
            self.legacy_compute_witness_inner::<RomDataTraceRow<F>>(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
            )
        }
    }
    fn legacy_compute_witness_inner<R: RomDataTraceRowOps<F>>(
        &self,
        mem_ops: &[MemInput],
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = RomDataTrace::<R>::new_from_vec(trace_buffer)?;
        let num_rows = RomDataTrace::<R>::NUM_ROWS;
        assert!(
            !mem_ops.is_empty() && mem_ops.len() <= num_rows,
            "RomDataSM: mem_ops.len()={} out of range {}",
            mem_ops.len(),
            num_rows
        );

        // range of instance
        self.std.range_check_one(self.range_id, previous_segment.addr - ROM_DATA_W_ADDR_INIT);

        let mut max_range_distance_count = 0;

        // Fill the remaining rows
        let mut last_addr: u32 = previous_segment.addr;
        let mut last_step: u64 = previous_segment.step;
        let mut last_value: u64 = previous_segment.value;

        if segment_id == 0 {
            // In the pil, in first row of first segment, we use previous_segment less 1, to
            // allow to use ROM_DATA_W_ADDR_INIT as address, and active address change flag
            // to free the value, if not
            last_addr = ROM_DATA_W_ADDR_INIT - 1;
        }
        let mut i = 0;

        for mem_op in mem_ops.iter() {
            let distance = mem_op.addr - last_addr;
            if i >= num_rows {
                break;
            }
            if distance > SEGMENT_ADDR_MAX_RANGE as u32 {
                let mut internal_reads = (distance - 1) / SEGMENT_ADDR_MAX_RANGE as u32;

                #[cfg(feature = "debug_mem")]
                println!(
                    "INTERNAL_READS[{},{}] {} 0x{:X},{} LAST:0x{:X}",
                    segment_id,
                    i,
                    internal_reads,
                    mem_op.addr * 8,
                    mem_op.step,
                    last_addr * 8
                );

                // check if has enough rows to complete the internal reads + regular memory
                let incomplete = (i + internal_reads as usize) >= num_rows;
                if incomplete {
                    internal_reads = (num_rows - i) as u32;
                }

                trace[i].set_addr_changes(true);
                last_addr += SEGMENT_ADDR_MAX_RANGE as u32;
                max_range_distance_count += 1;
                trace[i].set_addr(last_addr);
                trace[i].set_all_value(&[0; 2]);
                trace[i].set_sel(false);
                // the step, value of internal reads isn't relevant
                trace[i].set_step(0);
                i += 1;

                for _j in 1..internal_reads {
                    trace[i] = trace[i - 1];
                    last_addr += SEGMENT_ADDR_MAX_RANGE as u32;
                    max_range_distance_count += 1;
                    trace[i].set_addr(last_addr);
                    i += 1;
                }
                if incomplete {
                    break;
                }
            }
            trace[i].set_addr(mem_op.addr);
            trace[i].set_step(mem_op.step);
            trace[i].set_sel(true);

            let (low_val, high_val) = self.get_u32_values(mem_op.value);
            trace[i].set_all_value(&[low_val, high_val]);

            let addr_changes = last_addr != mem_op.addr;
            if addr_changes || (i == 0 && segment_id == 0) {
                trace[i].set_addr_changes(true);
                self.std.range_check_one(self.range_id, mem_op.addr - last_addr - 1);
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
        // PADDING: At end of memory fill with same addr, incrementing step, same value, sel = 0, rd
        // = 1, wr = 0
        let last_row_idx = count - 1;
        if count < num_rows {
            trace[count] = trace[last_row_idx];
            trace[count].set_addr_changes(false);
            trace[count].set_sel(false);

            for i in count + 1..num_rows {
                trace[i] = trace[i - 1];
            }
            // address doesn't change in padding rows, no range check is required
        }

        self.std.range_check(
            self.range_id,
            SEGMENT_ADDR_MAX_RANGE,
            max_range_distance_count as u32,
        );
        self.std.range_check_one(self.range_id, ROM_DATA_W_ADDR_END - last_addr);

        let mut air_values = RomDataAirValues::<F>::new();
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

        #[cfg(feature = "debug_mem")]
        {
            let path = std::env::var("MEM_TRACE_DIR").unwrap_or("tmp/mem_trace".to_string());
            let filename = format!("{path}/rom_trace_{segment_id:04}.txt");
            Self::save_to_file(&trace, &filename);
        }

        #[cfg(feature = "debug_mem")]
        Self::save_addr_offsets_to_file(
            &trace,
            &format!("tmp/rom_data_trace_{segment_id:04}_offsets.txt"),
        );

        Self::dump_trace_to_file(
            &trace,
            &format!("tmp/rom_data_trace_legacy_{segment_id:04}_dump.txt"),
        );
        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values)))
    }

    #[allow(clippy::too_many_arguments)]
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
            self.compute_witness_with_offsets_inner::<RomDataTraceRowPacked<F>>(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
                seg,
            )
        } else {
            self.compute_witness_with_offsets_inner::<RomDataTraceRow<F>>(
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
    fn compute_witness_with_offsets_inner<R: RomDataTraceRowOps<F>>(
        &self,
        mem_ops: &[MemInput],
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
        seg: &MemModuleSegmentCheckPoint,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = RomDataTrace::<R>::new_from_vec(trace_buffer)?;
        let num_rows = RomDataTrace::<R>::NUM_ROWS;
        assert!(
            !mem_ops.is_empty() && mem_ops.len() <= num_rows,
            "RomDataSM: mem_ops.len()={} out of range {}",
            mem_ops.len(),
            num_rows
        );
        // save_offsets_to_file(
        //     seg,
        //     &format!("tmp/rom_data_trace_gpu_{segment_id:04}_offsets.txt"),
        // );

        let mut current_offsets = vec![0u32; seg.addr_range_slots as usize];
        let mut range_check_cache = vec![0u32; MAX_RANGE_CHECK_CACHE];

        #[cfg(debug_assertions)]
        let mut filled_rows = vec![false; trace.num_rows()];
        let offset_base_addr_w = seg.offsets_base_addr >> 3;

        let distance_from_prev = previous_segment.addr - ROM_DATA_W_ADDR_INIT;
        if distance_from_prev < MAX_RANGE_CHECK_CACHE as u32 {
            range_check_cache[distance_from_prev as usize] += 1;
        } else {
            self.std.range_check_one(self.range_id, distance_from_prev as i64);
        }

        if seg.offset_at(0) == 0 {
            current_offsets[0] = OFFSET_USE_FLAG;
            // first address == halo
        }

        for mem_op in mem_ops.iter() {
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
            #[cfg(debug_assertions)]
            {
                assert!(!filled_rows[irow],"RomDataSM: overriting non empty row {irow} for mem_op with addr 0x{:X} => 0x{:X} step:{} => {}",
                    trace[irow].get_addr() * 8, mem_op.addr * 8, trace[irow].get_step(), mem_op.step);
                filled_rows[irow] = true;
            }

            trace[irow].set_addr(mem_op.addr);
            trace[irow].set_step(mem_op.step);
            trace[irow].set_sel(true);

            let (low_val, high_val) = self.get_u32_values(mem_op.value);
            trace[irow].set_value(0, low_val);
            trace[irow].set_value(1, high_val);

            if addr_changes || (irow == 0 && segment_id == 0) {
                trace[irow].set_addr_changes(true);
                let previous_addr = seg
                    .previous_change_addr_w(addr_index as u32)
                    .unwrap_or(previous_segment.addr as u64);
                let distance = mem_op.addr as i64
                    - previous_addr as i64
                    - !(irow == 0 && segment_id == 0) as i64;
                if distance < MAX_RANGE_CHECK_CACHE as i64 {
                    range_check_cache[distance as usize] += 1;
                } else {
                    self.std.range_check_one(self.range_id, distance);
                }
            } else {
                trace[irow].set_addr_changes(false);
            }
        }
        // STEP3. Add dummy rows to the output vector to fill the remaining rows
        // PADDING: At end of memory fill with same addr, incrementing step, same value, sel = 0, rd
        // = 1, wr = 0
        let count = mem_ops.len();
        let last_row = trace[count - 1];

        #[cfg(debug_assertions)]
        {
            let mut prev_filled_row = filled_rows[0];
            let mut from_row = 0;
            let _count = if is_last_segment { count } else { trace.num_rows() };
            for (i, filled) in filled_rows.iter().enumerate().take(_count) {
                debug_assert!(
                    filled_rows[i] == prev_filled_row,
                    "RomDataSM: not complete instance found [{}..{}] = {}",
                    from_row,
                    i - 1,
                    prev_filled_row
                );
            }
        }

        if count < num_rows {
            trace[count] = last_row;
            trace[count].set_addr_changes(false);
            trace[count].set_sel(false);

            for i in count + 1..num_rows {
                trace[i] = trace[i - 1];
            }
            // address doesn't change in padding rows, no range check is required
        }

        let distance_to_end = (ROM_DATA_W_ADDR_END - last_row.get_addr()) as i64;
        if distance_to_end < MAX_RANGE_CHECK_CACHE as i64 {
            range_check_cache[distance_to_end as usize] += 1;
        } else {
            self.std.range_check_one(self.range_id, distance_to_end);
        }
        self.std.range_check_ranged(self.range_id, None, &range_check_cache);

        let mut air_values = RomDataAirValues::<F>::new();
        air_values.segment_id = F::from_usize(segment_id.into());
        air_values.is_first_segment = F::from_bool(segment_id == 0);
        air_values.is_last_segment = F::from_bool(is_last_segment);
        air_values.previous_segment_step = F::from_u64(previous_segment.step);
        air_values.previous_segment_addr = F::from_u32(previous_segment.addr);
        air_values.segment_last_addr = F::from_u32(last_row.get_addr());
        air_values.segment_last_step = F::from_u64(last_row.get_step());

        air_values.previous_segment_value[0] = F::from_u32(previous_segment.value as u32);
        air_values.previous_segment_value[1] = F::from_u32((previous_segment.value >> 32) as u32);

        air_values.segment_last_value[0] = F::from_u32(last_row.get_value(0));
        air_values.segment_last_value[1] = F::from_u32(last_row.get_value(1));

        #[cfg(feature = "debug_mem")]
        {
            let path = std::env::var("MEM_TRACE_DIR").unwrap_or("tmp/mem_trace".to_string());
            let filename = format!("{path}/rom_trace_{segment_id:04}.txt");
            Self::save_to_file(&trace, &filename);
        }

        #[cfg(feature = "debug_mem")]
        Self::dump_trace_to_file(
            &trace,
            &format!("tmp/rom_data_trace_gpu_{segment_id:04}_dump.txt"),
        );
        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values)))
    }

    pub fn dump_trace_to_file<R: RomDataTraceRowOps<F>>(trace: &RomDataTrace<R>, file_name: &str) {
        println!("[RomDataDebug] dumping trace to {} .....", file_name);
        let file = File::create(file_name).unwrap();
        let mut writer = BufWriter::new(file);
        let num_rows = RomDataTrace::<R>::NUM_ROWS;

        writeln!(writer, "row addr wr step chunk step_dual chunk_dual value sel_dual increment")
            .unwrap();
        for i in 0..num_rows {
            let addr = trace[i].get_addr() as u64 * 8;
            let step = trace[i].get_step();
            let sel = trace[i].get_sel();
            let chunk = if step == 0 { 0 } else { MemHelpers::mem_step_to_chunk(step).0 };
            let value = trace[i].get_value(0) as u64 | ((trace[i].get_value(1) as u64) << 32);
            let wr = trace[i].get_sel() as u8;

            writeln!(writer, "{i} {addr:#08X} {wr} {step} {chunk} 0x{value:X} {sel}").unwrap();
        }
        println!("[RomDataDebug] done");
    }

    #[cfg(feature = "debug_mem")]
    pub fn save_to_file<R: RomDataTraceRowOps<F>>(trace: &RomDataTrace<R>, file_name: &str) {
        let file = File::create(file_name).unwrap();
        let mut writer = BufWriter::new(file);
        let num_rows = RomDataTrace::<R>::NUM_ROWS;

        for i in 0..num_rows {
            let addr = trace[i].get_addr() * 8;
            let step = trace[i].get_step();
            let sel = trace[i].get_sel();
            // TODO: chunk_size * 4 = 20
            writeln!(
                writer,
                "{:#010X} {} {:?} S:{sel} @{}",
                addr,
                step,
                trace[i].get_value(0) as u64 + ((trace[i].get_value(1) as u64) << 32),
                (step - 1) >> 20
            )
            .unwrap();
        }
    }

    #[cfg(feature = "debug_mem")]
    pub fn save_addr_offsets_to_file<R: RomDataTraceRowOps<F>>(
        trace: &RomDataTrace<R>,
        file_name: &str,
    ) {
        println!("[RomDataDebug] saving address offsets to {} .....", file_name);
        let file = std::fs::File::create(file_name).unwrap();
        let mut writer = std::io::BufWriter::new(file);
        let num_rows = RomDataTrace::<R>::NUM_ROWS;

        let mut last_addr = u32::MAX;
        for i in 0..num_rows {
            let addr = trace[i].get_addr();
            if addr != last_addr {
                writeln!(writer, "0x{:08X} {i}", addr * 8).unwrap();
                last_addr = addr;
            }
        }
        writeln!(writer).unwrap();
        println!("[RomDataDebug] done");
    }
}

impl<F: PrimeField64> MemModule<F> for RomDataSM<F> {
    fn get_addr_range(&self) -> (u32, u32) {
        (ROM_DATA_W_ADDR_INIT, ROM_DATA_W_ADDR_END)
    }
    fn is_dual(&self) -> bool {
        false
    }
    fn get_mem_name(&self) -> &str {
        "rom"
    }
    /// Finalizes the witness accumulation process and triggers the proof generation.
    ///
    /// This method is invoked by the executor when no further witness data remains to be added.
    ///
    /// # Parameters
    ///
    /// - `mem_inputs`: A slice of all `MemoryInput` inputs
    #[allow(clippy::too_many_arguments)]
    #[cfg_attr(feature = "legacy_mem_count_and_plan", allow(unused_variables))]
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
