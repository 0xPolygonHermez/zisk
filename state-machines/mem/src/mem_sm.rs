use std::sync::Arc;
use zisk_common::SegmentId;
use zisk_pil::{MemAirValues, MemTrace, MemTraceRow, MemTraceRowOps, MemTraceRowPacked};

#[cfg(any(feature = "debug_mem"))]
use std::{
    env,
    fs::File,
    io::{BufWriter, Write},
};

#[cfg(any(feature = "debug_mem", feature = "debug_mem_offsets"))]
use crate::mem_module::save_offsets_to_file;

use crate::{MemInput, MemModule};
use fields::PrimeField64;
use mem_common::{MemHelpers, MemModuleSegmentCheckPoint, RAM_W_ADDR_END, RAM_W_ADDR_INIT};
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use zisk_core::{RAM_ADDR, RAM_SIZE};

const OFFSET_DUAL_FLAG: u32 = 0x8000_0000;
const OFFSET_USE_FLAG: u32 = 0x4000_0000;
const OFFSET_VALUE_MASK: u32 = 0x3FFF_FFFF;
pub struct MemSM<F: PrimeField64> {
    /// PIL2 standard library
    std: Arc<Std<F>>,

    range_22bits_id: usize,
    range_16bits_id: usize,
}
#[derive(Debug, Default)]
pub struct MemPreviousSegment {
    pub addr: u32,
    pub step: u64,
    pub value: u64,
}

#[allow(unused, unused_variables)]
impl<F: PrimeField64> MemSM<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let range_22bits_id =
            std.get_range_id(0, (1 << 22) - 1, None).expect("Failed to get 22 bits range ID");
        let range_16bits_id =
            std.get_range_id(0, (1 << 16) - 1, None).expect("Failed to get 16 bits range ID");

        Arc::new(Self { range_22bits_id, range_16bits_id, std: std.clone() })
    }

    pub fn get_to_addr() -> u32 {
        (RAM_ADDR + RAM_SIZE - 1) as u32
    }
    #[cfg(feature = "debug_mem")]
    pub fn save_to_file<R: MemTraceRowOps<F>>(trace: &MemTrace<R>, file_name: &str) {
        println!("[MemDebug] writing information {} .....", file_name);
        let file = File::create(file_name).unwrap();
        let mut writer = BufWriter::new(file);
        let num_rows = MemTrace::<R>::NUM_ROWS;

        for i in 0..num_rows {
            let addr = trace[i].get_addr() as u64 * 8;
            let step = trace[i].get_step();
            let main_step = MemHelpers::mem_step_to_main_step(step);
            let op = if trace[i].get_wr() { 'W' } else { 'R' };
            let values = [trace[i].get_value(0) as u64, trace[i].get_value(1) as u64];
            let value = values[0] | (values[1] << 32);
            writeln!(
                writer,
                "{i:<8} {addr:#010X} {step:>13} {main_step:>12} {op} {values:?} 0x{value:016X}"
            )
            .unwrap();
            let dual = trace[i].get_sel_dual();
            if dual {
                let step = trace[i].get_step_dual();
                writeln!(writer, "{i:<8} {addr:#010X} {step:>13} {main_step:>12} R {values:?} 0x{value:016X} DUAL")
                    .unwrap();
            }
        }
        println!("[MemDebug] done");
    }

    #[cfg(feature = "debug_mem")]
    pub fn dump_trace_to_file<R: MemTraceRowOps<F>>(trace: &MemTrace<R>, file_name: &str) {
        println!("[MemDebug] dumping trace to {} .....", file_name);
        let file = File::create(file_name).unwrap();
        let mut writer = BufWriter::new(file);
        let num_rows = MemTrace::<R>::NUM_ROWS;

        writeln!(writer, "row addr wr step chunk step_dual chunk_dual value sel_dual increment")
            .unwrap();
        for i in 0..num_rows {
            let addr = trace[i].get_addr() as u64 * 8;
            let step = trace[i].get_step();
            let step_dual = trace[i].get_step_dual();
            let chunk = if step == 0 { 0 } else { MemHelpers::mem_step_to_chunk(step).0 };
            let chunk_dual =
                if step_dual == 0 { 0 } else { MemHelpers::mem_step_to_chunk(step_dual).0 };
            let value = trace[i].get_value(0) as u64 | ((trace[i].get_value(1) as u64) << 32);
            let wr = trace[i].get_wr() as u8;
            let sel_dual = trace[i].get_sel_dual() as u8;
            let l_increment = trace[i].get_l_increment() as u64;
            let h_increment = trace[i].get_h_increment() as u64;

            let increment = l_increment + (h_increment << 22);
            writeln!(writer, "{i} {addr:#08X} {wr} {step} {chunk} {step_dual} {chunk_dual} 0x{value:X} {sel_dual} {increment}")
                .unwrap();
        }
        println!("[MemDebug] done");
    }

    #[cfg(any(feature = "debug_mem", feature = "debug_mem_offsets"))]
    pub fn save_addr_offsets_to_file<R: MemTraceRowOps<F>>(trace: &MemTrace<R>, file_name: &str) {
        use std::io::Write;

        println!("[MemDebug] saving address offsets to {} .....", file_name);
        let file = std::fs::File::create(file_name).unwrap();
        let mut writer = std::io::BufWriter::new(file);
        let num_rows = MemTrace::<R>::NUM_ROWS;

        let mut last_addr = u32::MAX;
        let mut first = true;
        for i in 0..num_rows {
            let addr = trace[i].get_addr();
            if addr != last_addr {
                writeln!(writer, "0x{:08X} {i}", addr * 8).unwrap();
                last_addr = addr;
            }
        }
        writeln!(writer).unwrap();
        println!("[MemDebug] done");
    }

    #[cfg(feature = "debug_mem")]
    pub fn save_range_to_file(range: &[u32], segment_id: SegmentId, tag: &str) {
        let file_name = format!("tmp/mem_range_{segment_id:04}_{tag}.txt");
        println!("[MemDebug] saving range {tag} to {file_name} .....");
        let file = File::create(&file_name).unwrap();
        let mut writer = BufWriter::new(file);
        for (index, &value) in range.iter().enumerate() {
            if value != 0 {
                writeln!(writer, "{index} {value}").unwrap();
            }
        }
        println!("[MemDebug] done");
    }

    #[cfg(feature = "debug_mem")]
    pub fn save_mem_inputs_to_file(mem_ops: &[MemInput], segment_id: SegmentId) {
        let file_name = format!("tmp/mem_inputs_{segment_id}.txt");
        println!("[MemDebug] saving mem_inputs to {} .....", file_name);
        let file = File::create(&file_name).unwrap();
        let mut writer = BufWriter::new(file);
        for op in mem_ops {
            let is_write = if op.is_write { 1u8 } else { 0u8 };
            let chunk = if op.step == 0 { 0 } else { MemHelpers::mem_step_to_chunk(op.step).0 };
            writeln!(
                writer,
                "0x{:08X} {} {} {} 0x{:016X}",
                op.addr * 8,
                is_write,
                op.step,
                chunk,
                op.value
            )
            .unwrap();
        }
        println!("[MemDebug] done");
    }

    /// Saves a dense binary offsets table for a segment instance.
    ///
    /// File format: raw array of u32 little-endian values, one per qword slot.
    /// The number of entries is implicit (file_size / 4).
    /// The base address is stored externally (segment->offsets_base_addr).
    ///
    /// offsets[i] = 1-based row index of the first trace row whose qword address equals
    ///              (from_addr + i).  If that address is absent, the slot inherits the value
    ///              of the preceding slot (forward propagation); position 0 stays 0 when the
    ///              halo address does not appear in the trace.
    ///
    /// from_addr: first qword address in the trace for segment 0; previous_segment.addr for later
    ///            segments so the halo slot (index 0) is always present.
    #[cfg(feature = "debug_mem_bin_offsets")]
    pub fn save_bin_offsets_to_file<R: MemTraceRowOps<F>>(
        trace: &MemTrace<R>,
        segment_id: SegmentId,
        previous_segment: &MemPreviousSegment,
        count: usize,
        file_name: &str,
    ) {
        use std::io::Write;

        if count == 0 {
            println!("[MemDebug] save_bin_offsets_to_file: count is 0, skipping {}", file_name);
            return;
        }
        println!("[MemDebug] saving binary offsets to {} .....", file_name);

        let first_trace_addr = trace[0].get_addr();
        let last_trace_addr = trace[count - 1].get_addr();

        // For segment 0 the range starts at the first trace address; for later segments it starts
        // at previous_segment.addr so the halo slot (position 0) is always present.
        let from_addr: u32 = if segment_id == 0 { first_trace_addr } else { previous_segment.addr };

        let num_entries = (last_trace_addr - from_addr + 1) as usize;
        let mut offsets: Vec<u32> = vec![0u32; num_entries];

        // Walk every row; on each address change fill the current slot and any gap since the
        // previous address with the current offset (i + 1).
        // Slots before the first address stay 0 (halo position stays 0 when not in trace).
        let mut last_seen = from_addr;
        if segment_id == 0 {
            offsets[0] = 1;
        } else {
            offsets[0] = 0;
        }
        for i in 0..count {
            let addr = trace[i].get_addr();
            if addr != last_seen {
                let idx = (addr - from_addr) as usize;
                let fill_from = (last_seen - from_addr) as usize + 1;
                for offset in offsets.iter_mut().take(idx + 1).skip(fill_from) {
                    *offset = (i + 1) as u32;
                }
                last_seen = addr;
            }
        }

        // Write only the raw offset array – no header.
        let file = std::fs::File::create(file_name).unwrap();
        let mut writer = std::io::BufWriter::new(file);
        for &offset in &offsets {
            writer.write_all(&offset.to_le_bytes()).unwrap();
        }
        println!("[MemDebug] done ({} entries, from_addr qword: 0x{:08X})", num_entries, from_addr);
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
            self.legacy_compute_witness_inner::<MemTraceRowPacked<F>>(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
            )
        } else {
            self.legacy_compute_witness_inner::<MemTraceRow<F>>(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
            )
        }
    }
    fn legacy_compute_witness_inner<R: MemTraceRowOps<F>>(
        &self,
        mem_ops: &[MemInput],
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = MemTrace::<R>::new_from_vec(trace_buffer)?;

        let mut range_22bits: Vec<u32> = vec![0; 1 << 22];
        let mut range_16bits: Vec<u32> = vec![0; 1 << 16];

        // use special counter for internal reads
        let distance_base = previous_segment.addr - RAM_W_ADDR_INIT;
        let mut last_addr = previous_segment.addr;
        let mut last_value = previous_segment.value;
        let mut dual_candidate = false;
        // the last_step of previous_row
        let mut last_step = previous_segment.step;

        let mut i = 0;
        let mut step = 0;
        let mem_op_count = mem_ops.len();
        for index in 0..mem_op_count {
            let mem_op = &mem_ops[index];
            step = mem_op.step;

            let addr_changes = last_addr != mem_op.addr;
            if dual_candidate {
                dual_candidate = false;
                trace[i].set_previous_step(last_step);
                // println!("trace[{i}].previous_step = {last_step} (last_step)");
                let previous_step = mem_ops[index - 1].step;
                let previous_chunk_id = MemHelpers::mem_step_to_chunk(previous_step);
                let chunk_id = MemHelpers::mem_step_to_chunk(step);
                if mem_op.is_write || addr_changes || previous_chunk_id != chunk_id {
                    // not dual, because write or addr changes
                    trace[i].set_sel_dual(false);
                    trace[i].set_step_dual(0);
                    // last step is previous_step (step)
                    last_step = previous_step;
                    i += 1;
                } else {
                    trace[i].set_sel_dual(true);
                    trace[i].set_step_dual(step);
                    // last step is step_dual (step)
                    last_step = step;
                    let increment_step =
                        step - previous_step - if mem_ops[index - 1].is_write { 1 } else { 0 };
                    assert_eq!(
                        (trace[i].get_step_dual() - trace[i].get_step() - trace[i].get_wr() as u64),
                        increment_step
                    );
                    range_22bits[increment_step as usize] += 1;

                    i += 1;
                    continue;
                }
            }

            if i >= trace.num_rows() {
                break;
            }

            dual_candidate = true;

            // set the common values of trace between internal reads and regular memory operation
            trace[i].set_addr(mem_op.addr);
            trace[i].set_addr_changes(addr_changes);

            let mut increment = if addr_changes {
                (mem_op.addr - last_addr) as usize
            } else {
                if step < last_step {
                    panic!(
                        "MemSM: step < last_step {} < {} addr_changes:{} mem_op.addr:0x{:X} last_addr:0x{:X} mem_op.step:{} last_step:{} row:{} previous:{:?}",
                        step, last_step, addr_changes as u8, mem_op.addr * 8, last_addr * 8, mem_op.step, last_step, i, previous_segment
                    );
                }
                (step - last_step) as usize
            };

            // set specific values of trace for regular memory operation
            let (low_val, high_val) = (mem_op.value as u32, (mem_op.value >> 32) as u32);
            trace[i].set_all_value(&[low_val, high_val]);

            trace[i].set_step(step);
            trace[i].set_sel(true);

            if addr_changes || mem_op.is_write {
                // in case of read operations of same address, add one to allow many reads
                // over same address and step
                trace[i].set_read_same_addr(false);
                increment -= 1;
            } else {
                trace[i].set_read_same_addr(true);
            }
            let l_increment = increment & ((1 << 22) - 1);
            let h_increment = increment >> 22;
            trace[i].set_l_increment(l_increment as u32);
            trace[i].set_h_increment(h_increment as u16);
            trace[i].set_wr(mem_op.is_write);

            #[cfg(feature = "debug_mem")]
            if (l_increment >= (1 << 22)) || (h_increment >= (1 << 16)) {
                panic!("MemSM: increment's out of range: {} i:{} addr_changes:{} mem_op.addr:0x{:X} last_addr:0x{:X} mem_op.step:{} last_step:{}",
                    increment, i, addr_changes as u8, mem_op.addr, last_addr, mem_op.step, last_step);
            }

            range_22bits[l_increment] += 1;
            range_16bits[h_increment] += 1;

            last_addr = mem_op.addr;
            last_value = mem_op.value;
        }
        if dual_candidate {
            // if dual, need to "close" not dual row
            trace[i].set_sel_dual(false);
            trace[i].set_step_dual(0);
            trace[i].set_previous_step(last_step);
            last_step = step;
            i += 1;
        }
        let count = i;

        // STEP3. Add dummy rows to the output vector to fill the remaining rows
        // PADDING: At end of memory fill with same addr, incrementing step, same value, sel = 0, rd
        // = 1, wr = 0
        let last_row_idx = count - 1;
        let last_row = trace[last_row_idx];
        let addr = last_row.get_addr();
        let step =
            if !last_row.get_sel_dual() { last_row.get_step() } else { last_row.get_step_dual() };

        let value = last_row.get_all_value();
        let padding_size = trace.num_rows() - count;
        assert!(
            is_last_segment || padding_size == 0,
            "MemSM: padding_size must be 0 for non last segment, but got {padding_size}"
        );
        for i in count..trace.num_rows() {
            trace[i].set_previous_step(step);
            trace[i].set_addr(addr);
            trace[i].set_step(step);
            trace[i].set_sel(false);
            trace[i].set_wr(false);

            trace[i].set_all_value(&value);

            trace[i].set_addr_changes(false);
            trace[i].set_h_increment(0);
            trace[i].set_l_increment(0);
            trace[i].set_read_same_addr(true);
            trace[i].set_sel_dual(false);
            trace[i].set_step_dual(0);
        }

        if padding_size > 0 {
            // Store the padding range checks
            range_16bits[0] += padding_size as u32;
            range_22bits[0] += padding_size as u32;
        }

        // no add extra +1 because index = value - 1
        // RAM_W_ADDR_END - last_addr + 1 - 1 = RAM_W_ADDR_END - last_addr
        let distance_end = RAM_W_ADDR_END - last_addr;

        // Add one in range_check_data_max because it's used by intermediate reads, and reads
        // add one to distance to allow same step on read operations.

        let mut air_values = MemAirValues::<F>::new();
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

        self.std.range_check_ranged(self.range_22bits_id, None, &range_22bits);
        self.std.range_check_ranged(self.range_16bits_id, None, &range_16bits);

        #[cfg(feature = "debug_mem")]
        {
            let path = env::var("MEM_TRACE_DIR").unwrap_or("tmp/mem_trace".to_string());
            let filename = format!("{path}/mem_trace_{segment_id:04}.txt");
            println!("Saving {filename}");
            Self::save_to_file(&trace, &filename);
            println!("[Mem:{}] mem_ops:{} padding:{}", segment_id, mem_ops.len(), padding_size);
        }

        #[cfg(feature = "debug_mem_bin_offsets")]
        Self::save_bin_offsets_to_file(
            &trace,
            segment_id,
            previous_segment,
            count,
            &format!("tmp/mem_trace_{segment_id:04}_bin_offsets.bin"),
        );
        #[cfg(any(feature = "debug_mem", feature = "debug_mem_offsets"))]
        Self::save_addr_offsets_to_file(
            &trace,
            &format!("tmp/mem_trace_{segment_id:04}_offsets.txt"),
        );
        #[cfg(feature = "debug_mem")]
        Self::dump_trace_to_file(&trace, &format!("tmp/mem_trace_{segment_id:04}_dump.txt"));
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
            self.compute_witness_with_offsets_inner::<MemTraceRowPacked<F>>(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
                seg,
            )
        } else {
            self.compute_witness_with_offsets_inner::<MemTraceRow<F>>(
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
    fn compute_witness_with_offsets_inner<R: MemTraceRowOps<F>>(
        &self,
        mem_ops: &[MemInput],
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
        seg: &MemModuleSegmentCheckPoint,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = MemTrace::<R>::new_from_vec_zeroes(trace_buffer)?;
        #[cfg(feature = "debug_mem")]
        Self::save_mem_inputs_to_file(mem_ops, segment_id);
        #[cfg(any(feature = "debug_mem", feature = "debug_mem_offsets"))]
        save_offsets_to_file(seg, &format!("tmp/mem_trace_gpu_{segment_id:04}_offsets.txt"));

        let mut range_22bits: Vec<u32> = vec![0; 1 << 22];
        let mut range_16bits: Vec<u32> = vec![0; 1 << 16];

        // use special counter for internal reads
        let distance_base = previous_segment.addr - RAM_W_ADDR_INIT;
        // the last_step of previous_row
        let mut current_offsets = vec![0u32; seg.addr_range_slots as usize];

        #[cfg(feature = "debug_mem")]
        let mut filled_rows = vec![false; trace.num_rows()];
        let offset_base_addr_w = seg.offsets_base_addr >> 3;

        let mut i = 0;
        let mut step = 0;
        let mem_op_count = mem_ops.len();
        let mut last_row_idx = 0;

        // The address with offset 0 is the halo address, but point of view of continuations halo doesn't
        // implies a addr_changes, for this reason init current_offsets[0] with OFFSET_USE_FLAG
        if seg.offset_at(0) == 0 {
            current_offsets[0] = OFFSET_USE_FLAG;
        }
        for (index, mem_op) in mem_ops.iter().enumerate().take(mem_op_count) {
            step = mem_op.step;

            let addr_index = (mem_op.addr - offset_base_addr_w) as usize;
            // The most significant bit of current_offsets is used to indicate whether the dual row is available for this address
            let mut dual_available = current_offsets[addr_index] & OFFSET_DUAL_FLAG != 0;
            let addr_changes = current_offsets[addr_index] == 0;
            let mut irow = if addr_changes {
                let off_val = seg.offset_at(addr_index as u32);
                debug_assert!(off_val > 0, "MemSM: Address 0x{:X} at index {index} is out of offsets range, offset_base_addr_w: 0x{:X}",
                        mem_op.addr * 8, offset_base_addr_w * 8);
                off_val as usize - 1
            } else {
                (current_offsets[addr_index] & OFFSET_VALUE_MASK) as usize
            };

            let mut init_row = false;
            let increment = if mem_op.is_write {
                let _increment = if dual_available {
                    // A write goes to a new row; if dual_available is true it means that
                    // the current row is already occupied and therefore the write goes to a new row.
                    // dual_available also means that addr_changes must be false
                    debug_assert!(
                        !addr_changes,
                        "MemSM: dual_available && addr_changes (addr: 0x{:X} index: {index})",
                        mem_op.addr * 8
                    );
                    // fill sel_dual and step_dual for not dual row
                    dual_available = false;
                    irow += 1;
                    step - if irow == 0 {
                        previous_segment.step
                    } else {
                        trace[irow - 1].get_step()
                    } - 1
                } else if addr_changes {
                    // First access to this address → new row. The previous
                    // distinct address comes from the sparse change-point
                    // table (was a backward linear scan on the dense
                    // offsets array before the SoA refactor).
                    let previous_addr_w = seg
                        .previous_change_addr_w(addr_index as u32)
                        .unwrap_or(previous_segment.addr as u64);
                    mem_op.addr as u64 - previous_addr_w - 1
                } else {
                    // How addr_changes is false, means that the previous row belongs to same address,
                    // in this case, how the inputs are in natural time order, we could read from
                    // previous trace the "last step", if dual the dual_step, else the step.

                    let previous_step = if irow == 0 {
                        previous_segment.step
                    } else if trace[irow - 1].get_sel_dual() {
                        trace[irow - 1].get_step_dual()
                    } else {
                        trace[irow - 1].get_step()
                    };
                    if step <= previous_step {
                        #[cfg(feature = "debug_mem")]
                        Self::dump_trace_to_file(
                            &trace,
                            &format!("tmp/mem_trace_gpu_{segment_id:04}_dump.txt"),
                        );
                        panic!("MemSM: Warning: step {step} is not greater than previous_step {previous_step} \
                                for write operation at index {index} and irow {irow} with addr 0x{:X} \
                                (addr_index: {addr_index} previous_segment.addr: 0x{:X} offset_base_addr_w: 0x{:X})",
                            mem_op.addr * 8, previous_segment.addr * 8, offset_base_addr_w * 8);
                    }
                    step - previous_step - 1
                };
                current_offsets[addr_index] = (irow as u32) | OFFSET_DUAL_FLAG;
                init_row = true;
                _increment
            } else if addr_changes {
                // It's the first address access, it's a read, means no dual.
                debug_assert!(
                    !dual_available,
                    "MemSM: asddr_changes && dual_available (addr: 0x{:X} index: {index})",
                    mem_op.addr * 8
                );
                current_offsets[addr_index] = (irow as u32) | OFFSET_DUAL_FLAG;
                // dual available
                init_row = true;

                let previous_addr = seg
                    .previous_change_addr_w(addr_index as u32)
                    .unwrap_or(previous_segment.addr as u64);
                debug_assert!(previous_addr <= mem_op.addr as u64, "MemSM: Warning: address goes back \
                              from 0x{:X} to 0x{previous_addr:X} at irow {irow} (offset_base_addr_w: \
                              0x{offset_base_addr_w:X})",
                    mem_op.addr);
                mem_op.addr as u64 - previous_addr - 1
            } else if dual_available {
                // It's dual read, not addr_changes.
                let prev_step = trace[irow].get_step();
                // But duals must be in the same chunk, otherwise I can't use
                if MemHelpers::mem_steps_belongs_to_same_chunk(prev_step, step) {
                    trace[irow].set_sel_dual(true);
                    trace[irow].set_step_dual(step);
                    current_offsets[addr_index] = irow as u32 + 1;
                    step - prev_step - if trace[irow].get_wr() { 1 } else { 0 }
                } else {
                    dual_available = false;
                    irow += 1;
                    init_row = true;
                    current_offsets[addr_index] = (irow as u32) | OFFSET_DUAL_FLAG;
                    step - prev_step
                }
            } else {
                current_offsets[addr_index] = (irow as u32) | OFFSET_DUAL_FLAG;
                // dual available
                init_row = true;
                if irow >= trace.num_rows() {
                    println!(
                        "MemSM: Warning: irow {irow} goes beyond trace num_rows {} for mem_op at \
                        index {index} with addr 0x{:X} step {} {index}/{mem_op_count}",
                        trace.num_rows(),
                        mem_op.addr,
                        mem_op.step
                    );
                    break;
                }
                // set specific values of trace for regular memory operation
                step - if irow == 0 {
                    previous_segment.step
                } else if trace[irow - 1].get_sel_dual() {
                    trace[irow - 1].get_step_dual()
                } else {
                    trace[irow - 1].get_step()
                }
            };
            if init_row {
                if irow >= trace.num_rows() {
                    println!("MemSM: Warning: irow {irow} goes beyond trace num_rows {} for mem_op at index {index} with addr 0x{:X} step {} {index}/{mem_op_count}",
                        trace.num_rows(),mem_op.addr, mem_op.step);
                    break;
                }
                #[cfg(feature = "debug_mem")]
                {
                    if filled_rows[irow] {
                        Self::dump_trace_to_file(
                            &trace,
                            &format!("tmp/mem_trace_gpu_{segment_id:04}_dump.txt"),
                        );
                        println!("MemSM: overriting non empty row {irow} for mem_op at index {index} with addr 0x{:X} => 0x{:X} step ({},{}) => {} {index}/{mem_op_count}",
                    trace[irow].get_addr() * 8, mem_op.addr * 8, trace[irow].get_step(),trace[irow].get_step_dual(), mem_op.step);
                        break;
                    }
                    filled_rows[irow] = true;
                }
                // always set dual to false because we don't know if there will dual reads, maybe
                // this is the last access to this address in this segment.
                trace[irow].set_sel_dual(false);
                trace[irow].set_step_dual(0);
                trace[irow].set_addr(mem_op.addr);
                trace[irow].set_step(step);
                trace[irow].set_sel(true);
                trace[irow].set_addr_changes(addr_changes);
                trace[irow].set_wr(mem_op.is_write);
                let (low_val, high_val) = (mem_op.value as u32, (mem_op.value >> 32) as u32);
                trace[irow].set_value(0, low_val);
                trace[irow].set_value(1, high_val);
                // trace[irow].set_read_same_addr((addr_changes || mem_op.is_write) == false);
                // range check between rows
                let increment = increment as usize;
                let l_increment = increment & ((1 << 22) - 1);
                let h_increment = increment >> 22;
                trace[irow].set_l_increment(l_increment as u32);
                trace[irow].set_h_increment(h_increment as u16);

                range_22bits[l_increment] += 1;
                range_16bits[h_increment] += 1;
            }
            // trace[irow].set_previous_step
            if dual_available {
                // range check dual
                range_22bits[increment as usize] += 1;
            }
            if irow > last_row_idx {
                last_row_idx = irow;
            }
        }

        #[cfg(feature = "debug_mem")]
        {
            let mut prev_filled_row = filled_rows[0];
            let mut from_row = 0;
            let count = if is_last_segment { last_row_idx } else { trace.num_rows() };
            for (i, filled) in filled_rows.iter().enumerate().take(count) {
                debug_assert!(
                    *filled == prev_filled_row,
                    "MemSM: not complete instance found [{}..{}] = {}",
                    from_row,
                    i - 1,
                    prev_filled_row
                );
            }
        }
        // STEP3. Add dummy rows to the output vector to fill the remaining rows
        // PADDING: At end of memory fill with same addr, incrementing step, same value, sel = 0, rd
        // = 1, wr = 0
        let last_row = trace[last_row_idx];
        let addr = last_row.get_addr();
        let step =
            if !last_row.get_sel_dual() { last_row.get_step() } else { last_row.get_step_dual() };

        let low_value = last_row.get_value(0);
        let high_value = last_row.get_value(1);
        let padding_size = trace.num_rows() - last_row_idx - 1;
        for i in (last_row_idx + 1)..trace.num_rows() {
            // trace[i].set_previous_step(step);
            trace[i].set_addr(addr);
            trace[i].set_step(step);
            trace[i].set_sel(false);
            trace[i].set_wr(false);

            trace[i].set_value(0, low_value);
            trace[i].set_value(1, high_value);

            trace[i].set_addr_changes(false);
            trace[i].set_h_increment(0);
            trace[i].set_l_increment(0);
            // trace[i].set_read_same_addr(true);
            trace[i].set_sel_dual(false);
            trace[i].set_step_dual(0);
        }

        if padding_size > 0 {
            // Store the padding range checks
            range_16bits[0] += padding_size as u32;
            range_22bits[0] += padding_size as u32;
        }

        // no add extra +1 because index = value - 1
        // RAM_W_ADDR_END - last_addr + 1 - 1 = RAM_W_ADDR_END - last_addr
        let distance_end = RAM_W_ADDR_END - last_row.get_addr();

        // Add one in range_check_data_max because it's used by intermediate reads, and reads
        // add one to distance to allow same step on read operations.

        let mut air_values = MemAirValues::<F>::new();
        air_values.segment_id = F::from_usize(segment_id.into());
        air_values.is_first_segment = F::from_bool(segment_id == 0);
        air_values.is_last_segment = F::from_bool(is_last_segment);
        air_values.previous_segment_step = F::from_u64(previous_segment.step);
        air_values.previous_segment_addr = F::from_u32(previous_segment.addr);
        air_values.segment_last_addr = F::from_u32(last_row.get_addr());
        let last_step =
            if !last_row.get_sel_dual() { last_row.get_step() } else { last_row.get_step_dual() };
        air_values.segment_last_step = F::from_u64(last_step);

        air_values.previous_segment_value[0] = F::from_u32(previous_segment.value as u32);
        air_values.previous_segment_value[1] = F::from_u32((previous_segment.value >> 32) as u32);

        air_values.segment_last_value[0] = F::from_u32(last_row.get_value(0));
        air_values.segment_last_value[1] = F::from_u32(last_row.get_value(1));

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

        self.std.range_check_ranged(self.range_22bits_id, None, &range_22bits);
        self.std.range_check_ranged(self.range_16bits_id, None, &range_16bits);

        #[cfg(feature = "debug_mem")]
        {
            let path = env::var("MEM_TRACE_DIR").unwrap_or("tmp/mem_trace".to_string());
            let filename = format!("{path}/mem_trace_{segment_id:04}.txt");
            println!("Saving {filename}");
            Self::save_to_file(&trace, &filename);
            println!("[Mem:{}] mem_ops:{} padding:{}", segment_id, mem_ops.len(), padding_size);
        }
        #[cfg(feature = "debug_mem")]
        Self::dump_trace_to_file(&trace, &format!("tmp/mem_trace_gpu_{segment_id:04}_dump.txt"));
        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values)))
    }
}

impl<F: PrimeField64> MemModule<F> for MemSM<F> {
    fn get_addr_range(&self) -> (u32, u32) {
        (RAM_W_ADDR_INIT, RAM_W_ADDR_END)
    }
    fn is_dual(&self) -> bool {
        true
    }
    fn get_mem_name(&self) -> &str {
        "ram"
    }

    /// Finalizes the witness accumulation process and triggers the proof generation.
    ///
    /// This method is invoked by the executor when no further witness data remains to be added.
    ///
    /// # Parameters
    ///
    /// - `mem_inputs`: A slice of all `MemoryInput` inputs
    #[cfg_attr(feature = "legacy_mem_count_and_plan", allow(unused_variables))]
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
