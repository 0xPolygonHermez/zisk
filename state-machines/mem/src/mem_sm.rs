use std::sync::Arc;
use zisk_common::SegmentId;
use zisk_pil::{MemAirValues, MemTrace, MemTraceRow, MemTraceRowOps, MemTraceRowPacked};

#[cfg(feature = "debug_mem")]
use std::{
    env,
    fs::File,
    io::{BufWriter, Write},
};

#[cfg(any(feature = "debug_mem", feature = "debug_mem_offsets"))]
use crate::mem_module::save_offsets_to_file;

use crate::{MemInput, MemModule, MemOps};
use pil2_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_fields::PrimeField64;
// `phase_ms` is only ever read inside a `phase_log!`, which vanishes without the
// `witness_timers` feature -- and takes the only use of the import with it.
use rayon::prelude::*;
#[allow(unused_imports)]
use zisk_common::{phase_end, phase_log, phase_ms, phase_start};
use zisk_core::{RAM_ADDR, RAM_SIZE};

use crate::mem_witness_split::{split_mem_slots, MemFillRange};
use zisk_sm_mem_common::{
    MemHelpers, MemLanes, MemModuleSegmentCheckPoint, RAM_W_ADDR_END, RAM_W_ADDR_INIT,
};

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

/// Lane layout of the `Mem` trace, read from the generated row so it always
/// follows `lanes_x_row` in `state-machines/mem/pil/mem.pil`.
#[inline]
fn lanes_of<F: PrimeField64, R: MemTraceRowOps<F>>() -> MemLanes {
    MemLanes::new(R::default().get_all_addr().len())
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
        let lanes = lanes_of::<F, R>();

        for i in 0..num_rows {
            for lane in 0..lanes.lanes() {
                let addr = trace[i].get_addr(lane) as u64 * 8;
                let step = trace[i].get_step(lane);
                let main_step = if step == 0 { 0 } else { MemHelpers::mem_step_to_main_step(step) };
                let op = if trace[i].get_wr(lane) { 'W' } else { 'R' };
                let values =
                    [trace[i].get_value(lane, 0) as u64, trace[i].get_value(lane, 1) as u64];
                let value = values[0] | (values[1] << 32);
                writeln!(
                    writer,
                    "{i:<8}.{lane} {addr:#010X} {step:>13} {main_step:>12} {op} {values:?} 0x{value:016X}"
                )
                .unwrap();
                let dual = trace[i].get_sel_dual(lane);
                if dual {
                    let step = trace[i].get_step_dual(lane);
                    writeln!(writer, "{i:<8}.{lane} {addr:#010X} {step:>13} {main_step:>12} R {values:?} 0x{value:016X} DUAL")
                        .unwrap();
                }
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
        let lanes = lanes_of::<F, R>();

        writeln!(
            writer,
            "row lane addr wr step chunk step_dual chunk_dual value sel_dual increment"
        )
        .unwrap();
        for i in 0..num_rows {
            for lane in 0..lanes.lanes() {
                let addr = trace[i].get_addr(lane) as u64 * 8;
                let step = trace[i].get_step(lane);
                let step_dual = trace[i].get_step_dual(lane);
                let chunk = if step == 0 { 0 } else { MemHelpers::mem_step_to_chunk(step).0 };
                let chunk_dual =
                    if step_dual == 0 { 0 } else { MemHelpers::mem_step_to_chunk(step_dual).0 };
                let value = trace[i].get_value(lane, 0) as u64
                    | ((trace[i].get_value(lane, 1) as u64) << 32);
                let wr = trace[i].get_wr(lane) as u8;
                let sel_dual = trace[i].get_sel_dual(lane) as u8;
                let l_increment = trace[i].get_l_increment(lane) as u64;
                let h_increment = trace[i].get_h_increment(lane) as u64;

                let increment = l_increment + (h_increment << 22);
                writeln!(writer, "{i} {lane} {addr:#08X} {wr} {step} {chunk} {step_dual} {chunk_dual} 0x{value:X} {sel_dual} {increment}")
                .unwrap();
            }
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
    fn is_initializable(&self) -> bool {
        true
    }
    /// Finalizes the witness accumulation process and triggers the proof generation.
    ///
    /// File format: raw array of u32 little-endian values, one per qword slot.
    /// The number of entries is implicit (file_size / 4).
    /// The base address is stored externally (segment->offsets_base_addr).
    ///
    /// offsets[i] = 1-based virtual row (row * lanes_x_row + lane) of the first
    ///              memory lane whose qword address equals (from_addr + i).  If
    ///              that address is absent, the slot inherits the value of the
    ///              preceding slot (forward propagation); position 0 stays 0 when
    ///              the halo address does not appear in the trace.
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

        let lanes = lanes_of::<F, R>();
        let (last_row, last_lane) = lanes.split(count - 1);
        let first_trace_addr = trace[0].get_addr(0);
        let last_trace_addr = trace[last_row].get_addr(last_lane);

        // For segment 0 the range starts at the first trace address; for later segments it starts
        // at previous_segment.addr so the halo slot (position 0) is always present.
        let from_addr: u32 = if segment_id == 0 { first_trace_addr } else { previous_segment.addr };

        let num_entries = (last_trace_addr - from_addr + 1) as usize;
        let mut offsets: Vec<u32> = vec![0u32; num_entries];

        // Walk every virtual row; on each address change fill the current slot and any gap since
        // the previous address with the current offset (islot + 1).
        // Slots before the first address stay 0 (halo position stays 0 when not in trace).
        let mut last_seen = from_addr;
        if segment_id == 0 {
            offsets[0] = 1;
        } else {
            offsets[0] = 0;
        }
        for islot in 0..count {
            let (row, lane) = lanes.split(islot);
            let addr = trace[row].get_addr(lane);
            if addr != last_seen {
                let idx = (addr - from_addr) as usize;
                let fill_from = (last_seen - from_addr) as usize + 1;
                for offset in offsets.iter_mut().take(idx + 1).skip(fill_from) {
                    *offset = (islot + 1) as u32;
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
        // The legacy fill reads `mem_ops[index - 1]`, so it is the one path that needs the
        // operations contiguous. It also sorts them, so it needs ownership regardless.
        let mem_ops = &mem_ops.to_flat_vec()[..];
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

        let lanes = lanes_of::<F, R>();
        let num_slots = lanes.slots(trace.num_rows());

        let mut range_22bits: Vec<u32> = vec![0; 1 << 22];
        let mut range_16bits: Vec<u32> = vec![0; 1 << 16];

        // use special counter for internal reads
        let distance_base = previous_segment.addr - RAM_W_ADDR_INIT;
        let mut last_addr = previous_segment.addr;
        let mut last_value = previous_segment.value;
        let mut dual_candidate = false;
        // the last_step of previous lane
        let mut last_step = previous_segment.step;

        // `i` is a virtual row: `lanes.split(i)` gives the physical row and the lane inside it.
        let mut i = 0;
        let mut step = 0;
        let mem_op_count = mem_ops.len();
        for index in 0..mem_op_count {
            let mem_op = &mem_ops[index];
            step = mem_op.step;

            let addr_changes = last_addr != mem_op.addr;
            if dual_candidate {
                dual_candidate = false;
                let (row, lane) = lanes.split(i);
                trace[row].set_previous_step(lane, last_step);
                // println!("trace[{i}].previous_step = {last_step} (last_step)");
                let previous_step = mem_ops[index - 1].step;
                let previous_chunk_id = MemHelpers::mem_step_to_chunk(previous_step);
                let chunk_id = MemHelpers::mem_step_to_chunk(step);
                if mem_op.is_write || addr_changes || previous_chunk_id != chunk_id {
                    // not dual, because write or addr changes
                    trace[row].set_sel_dual(lane, false);
                    trace[row].set_step_dual(lane, 0);
                    // last step is previous_step (step)
                    last_step = previous_step;
                    i += 1;
                } else {
                    trace[row].set_sel_dual(lane, true);
                    trace[row].set_step_dual(lane, step);
                    // last step is step_dual (step)
                    last_step = step;
                    let increment_step =
                        step - previous_step - if mem_ops[index - 1].is_write { 1 } else { 0 };
                    assert_eq!(
                        (trace[row].get_step_dual(lane)
                            - trace[row].get_step(lane)
                            - trace[row].get_wr(lane) as u64),
                        increment_step
                    );
                    range_22bits[increment_step as usize] += 1;

                    i += 1;
                    continue;
                }
            }

            if i >= num_slots {
                break;
            }

            dual_candidate = true;

            let (row, lane) = lanes.split(i);

            // set the common values of trace between internal reads and regular memory operation
            trace[row].set_addr(lane, mem_op.addr);
            trace[row].set_addr_changes(lane, addr_changes);

            let mut increment = if addr_changes {
                (mem_op.addr - last_addr) as usize
            } else {
                if step < last_step {
                    panic!(
                        "MemSM: step < last_step {} < {} addr_changes:{} mem_op.addr:0x{:X} last_addr:0x{:X} mem_op.step:{} last_step:{} slot:{} previous:{:?}",
                        step, last_step, addr_changes as u8, mem_op.addr * 8, last_addr * 8, mem_op.step, last_step, i, previous_segment
                    );
                }
                (step - last_step) as usize
            };

            // set specific values of trace for regular memory operation
            let (low_val, high_val) = (mem_op.value as u32, (mem_op.value >> 32) as u32);
            trace[row].set_value(lane, 0, low_val);
            trace[row].set_value(lane, 1, high_val);

            trace[row].set_step(lane, step);
            trace[row].set_sel(lane, true);

            if addr_changes || mem_op.is_write {
                // in case of read operations of same address, add one to allow many reads
                // over same address and step
                trace[row].set_read_same_addr(lane, false);
                increment -= 1;
            } else {
                trace[row].set_read_same_addr(lane, true);
            }
            let l_increment = increment & ((1 << 22) - 1);
            let h_increment = increment >> 22;
            trace[row].set_l_increment(lane, l_increment as u32);
            trace[row].set_h_increment(lane, h_increment as u16);
            trace[row].set_wr(lane, mem_op.is_write);

            #[cfg(feature = "debug_mem")]
            if (l_increment >= (1 << 22)) || (h_increment >= (1 << 16)) {
                panic!("MemSM: increment's out of range: {} i:{} addr_changes:{} mem_op.addr:0x{:X} last_addr:0x{:X} mem_op.step:{} last_step:{}",
                    increment, i, addr_changes as u8, mem_op.addr, last_addr, mem_op.step, last_step);
            }

            #[cfg(feature = "debug_mem")]
            if h_increment >= 0x1_0000 {
                panic!("MemSM: h_increment out of range: {h_increment} increment:{increment} slot:{i} addr_changes:{addr_changes} mem_op.addr:0x{:X} last_addr:0x{:X} mem_op.step:{} last_step:{}",
                     mem_op.addr, last_addr, mem_op.step, last_step);
            }
            range_22bits[l_increment] += 1;
            range_16bits[h_increment] += 1;

            last_addr = mem_op.addr;
            last_value = mem_op.value;
        }
        if dual_candidate {
            // if dual, need to "close" not dual lane
            let (row, lane) = lanes.split(i);
            trace[row].set_sel_dual(lane, false);
            trace[row].set_step_dual(lane, 0);
            trace[row].set_previous_step(lane, last_step);
            last_step = step;
            i += 1;
        }
        let count = i;

        // STEP3. Add dummy lanes to the output vector to fill the remaining virtual rows
        // PADDING: At end of memory fill with same addr, incrementing step, same value, sel = 0, rd
        // = 1, wr = 0
        let (last_row, last_lane) = lanes.split(count - 1);
        let addr = trace[last_row].get_addr(last_lane);
        let step = if !trace[last_row].get_sel_dual(last_lane) {
            trace[last_row].get_step(last_lane)
        } else {
            trace[last_row].get_step_dual(last_lane)
        };

        let value =
            [trace[last_row].get_value(last_lane, 0), trace[last_row].get_value(last_lane, 1)];
        let padding_size = num_slots - count;
        assert!(
            is_last_segment || padding_size == 0,
            "MemSM: padding_size must be 0 for non last segment, but got {padding_size}"
        );
        for islot in count..num_slots {
            let (row, lane) = lanes.split(islot);
            trace[row].set_previous_step(lane, step);
            trace[row].set_addr(lane, addr);
            trace[row].set_step(lane, step);
            trace[row].set_sel(lane, false);
            trace[row].set_wr(lane, false);

            trace[row].set_value(lane, 0, value[0]);
            trace[row].set_value(lane, 1, value[1]);

            trace[row].set_addr_changes(lane, false);
            trace[row].set_h_increment(lane, 0);
            trace[row].set_l_increment(lane, 0);
            trace[row].set_read_same_addr(lane, true);
            trace[row].set_sel_dual(lane, false);
            trace[row].set_step_dual(lane, 0);
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
        mem_ops: MemOps<'_>,
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
    #[allow(clippy::too_many_arguments)]
    fn compute_witness_with_offsets_inner<R: MemTraceRowOps<F>>(
        &self,
        mem_ops: MemOps<'_>,
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
        seg: &MemModuleSegmentCheckPoint,
    ) -> ProofmanResult<AirInstance<F>> {
        // Taken as-is, NOT zeroed: the buffer comes from the recycled basic-trace pool and the
        // trace is `NUM_ROWS * lanes_x_row` slots wide, so zeroing it moved gigabytes before a
        // single operation had been placed -- and then the padding wrote most of them again.
        //
        // Safe because every lane of every slot is written exactly once: `fill_mem_range` writes
        // slots `0..=last_slot` (the offsets table is dense, so it leaves no hole -- a hole would
        // break the `l_increment + 2**22 * h_increment + 1 === ...` constraint whatever the buffer
        // held, zeroed or not), and the padding below writes every slot after it, up to
        // `num_slots`. The three writers -- `fill_mem_range`'s `init_row`, `copy_mem_lane` and
        // `set_mem_padding_lane` -- each set the same 11 columns, which is every column of the row
        // except `previous_step` and `read_same_addr`. Those two are the air's `witness_calc`
        // hints: the prover evaluates them into cm1 before it commits (`calculateWitnessExpr` in
        // STARK_STEP_1, and again on the contribution path in `commit_witness` / the GPU
        // `commit_witness_gpu`), so whatever this buffer holds for them is overwritten before it
        // is ever read. `mem_sm_fill_tests` pins both halves of that.
        phase_start!(t_zero);
        let mut trace = MemTrace::<R>::new_from_vec(trace_buffer)?;
        phase_end!(d_zero, t_zero);
        #[cfg(feature = "debug_mem")]
        Self::save_mem_inputs_to_file(mem_ops, segment_id);
        #[cfg(any(feature = "debug_mem", feature = "debug_mem_offsets"))]
        save_offsets_to_file(seg, &format!("tmp/mem_trace_gpu_{segment_id:04}_offsets.txt"));

        // One range per thread of the pool this witness computation is ALREADY running in: the
        // executor wraps the whole dispatch in `lease_pool(n_cores).install(...)`, so
        // `current_num_threads` is exactly the `n_cores` proofman handed over, and there is no
        // second knob to keep in step with it.
        let n_ranges = rayon::current_num_threads();
        let out = fill_mem_trace::<F, R>(
            &mut trace.buffer,
            mem_ops,
            seg,
            previous_segment,
            segment_id,
            is_last_segment,
            n_ranges,
        );

        let mut air_values = MemAirValues::<F>::new();
        air_values.segment_id = F::from_usize(segment_id.into());
        air_values.is_first_segment = F::from_bool(segment_id == 0);
        air_values.is_last_segment = F::from_bool(is_last_segment);
        air_values.previous_segment_step = F::from_u64(previous_segment.step);
        air_values.previous_segment_addr = F::from_u32(previous_segment.addr);
        air_values.segment_last_addr = F::from_u32(out.last_addr);
        air_values.segment_last_step = F::from_u64(out.last_step);

        air_values.previous_segment_value[0] = F::from_u32(previous_segment.value as u32);
        air_values.previous_segment_value[1] = F::from_u32((previous_segment.value >> 32) as u32);

        air_values.segment_last_value[0] = F::from_u32(out.last_value[0]);
        air_values.segment_last_value[1] = F::from_u32(out.last_value[1]);

        air_values.distance_base[0] = F::from_u16(out.distance_base[0]);
        air_values.distance_base[1] = F::from_u16(out.distance_base[1]);

        air_values.distance_end[0] = F::from_u16(out.distance_end[0]);
        air_values.distance_end[1] = F::from_u16(out.distance_end[1]);

        // Timed apart from the fill because it is not free: `range_check_ranged` widens the whole
        // 2^22-entry histogram into a fresh `Vec<u64>` (32 MiB) before `assign_values_ranged` walks
        // every bucket, so it costs the same whether the instance was full or nearly empty.
        phase_start!(t_rc);
        self.std.range_check_ranged(self.range_22bits_id, None, &out.range_22bits);
        self.std.range_check_ranged(self.range_16bits_id, None, &out.range_16bits);
        phase_end!(d_rc, t_rc);

        phase_start!(t_air);
        let air_instance = AirInstance::new_from_trace(
            FromTrace::new(&mut trace).with_air_values(&mut air_values),
        );
        phase_end!(d_air, t_air);
        phase_log!(
            "Mem[{}] witness: take trace {:.0}ms range checks {:.0}ms air instance {:.0}ms",
            usize::from(segment_id),
            phase_ms!(d_zero),
            phase_ms!(d_rc),
            phase_ms!(d_air)
        );

        #[cfg(feature = "debug_mem")]
        {
            let path = env::var("MEM_TRACE_DIR").unwrap_or("tmp/mem_trace".to_string());
            let filename = format!("{path}/mem_trace_{segment_id:04}.txt");
            println!("Saving {filename}");
            Self::save_to_file(&trace, &filename);
        }
        #[cfg(feature = "debug_mem")]
        Self::dump_trace_to_file(&trace, &format!("tmp/mem_trace_gpu_{segment_id:04}_dump.txt"));
        Ok(air_instance)
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

    fn is_initializable(&self) -> bool {
        true
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

/// The rows one fill range may write: the rows it owns outright, plus a scratch stand-in for the
/// row it shares with the previous range.
///
/// Ranges are cut at address boundaries, so a cut lands where that address's slots begin -- almost
/// never on a physical row boundary. The row straddling a cut therefore holds lanes of two ranges.
/// Rather than hand both a `&mut` to it, the later range writes its lanes into `head` and the
/// caller merges them back once every range has finished. That is a complete view for the range:
/// it only ever reads slots it wrote itself, because every backward read the fill makes stays
/// within one address, and an address belongs to a single range.
struct RowView<'a, R> {
    /// Stand-in for row `first_row`, merged into the trace by the caller.
    head: R,
    /// Rows `first_row + 1 ..` of the trace, this range's alone.
    owned: &'a mut [R],
    /// Absolute index of the shared leading row.
    first_row: usize,
}

impl<R> RowView<'_, R> {
    /// The row at absolute index `abs`. The branch picks the scratch row, so it is taken for at
    /// most `lanes_x_row - 1` of the millions of slots a range fills, and predicts perfectly.
    #[inline(always)]
    fn at(&mut self, abs: usize) -> &mut R {
        if abs == self.first_row {
            &mut self.head
        } else {
            &mut self.owned[abs - self.first_row - 1]
        }
    }
}

/// What one range hands back: its shared leading row, how far it got, and its multiplicities.
struct RangeFill<R> {
    head: R,
    /// Highest slot this range wrote, or `None` when its addresses carried no operation.
    last_slot: Option<usize>,
    range_22bits: Vec<u32>,
    range_16bits: Vec<u32>,
    /// Operations this range actually filled. The ranges are balanced by *slots*, so comparing
    /// these across ranges is what says whether slots are a good proxy for the work.
    #[cfg(feature = "witness_timers")]
    ops_filled: usize,
    /// Wall time this range took. The spread across ranges is the parallel speedup's ceiling.
    #[cfg(feature = "witness_timers")]
    elapsed: std::time::Duration,
}

/// Copies one lane of every column the fill writes. `previous_step` and `read_same_addr` are left
/// out on purpose: they are `<==` columns in `mem.pil`, so the prover derives them and the witness
/// never sets them.
#[inline]
fn copy_mem_lane<F: PrimeField64, R: MemTraceRowOps<F>>(dst: &mut R, src: &R, lane: usize) {
    dst.set_addr(lane, src.get_addr(lane));
    dst.set_step(lane, src.get_step(lane));
    dst.set_sel(lane, src.get_sel(lane));
    dst.set_addr_changes(lane, src.get_addr_changes(lane));
    dst.set_wr(lane, src.get_wr(lane));
    dst.set_sel_dual(lane, src.get_sel_dual(lane));
    dst.set_step_dual(lane, src.get_step_dual(lane));
    dst.set_value(lane, 0, src.get_value(lane, 0));
    dst.set_value(lane, 1, src.get_value(lane, 1));
    dst.set_l_increment(lane, src.get_l_increment(lane));
    dst.set_h_increment(lane, src.get_h_increment(lane));
}

/// One padding lane: same address, same step, same value, not selected. Kept in one place so the
/// partial row and the whole rows cannot drift apart.
#[inline]
fn set_mem_padding_lane<F: PrimeField64, R: MemTraceRowOps<F>>(
    row: &mut R,
    lane: usize,
    addr: u32,
    step: u64,
    low_value: u32,
    high_value: u32,
) {
    row.set_addr(lane, addr);
    row.set_step(lane, step);
    row.set_sel(lane, false);
    row.set_wr(lane, false);
    row.set_value(lane, 0, low_value);
    row.set_value(lane, 1, high_value);
    row.set_addr_changes(lane, false);
    row.set_h_increment(lane, 0);
    row.set_l_increment(lane, 0);
    row.set_sel_dual(lane, false);
    row.set_step_dual(lane, 0);
}

/// What the fill produces besides the rows themselves: the multiplicities, and the scalars the air
/// values are built from. Deliberately not `MemAirValues` -- that type carries a lifetime and a
/// self-referential buffer, and building it is the caller's business anyway.
struct MemFillOutput {
    range_22bits: Vec<u32>,
    range_16bits: Vec<u32>,
    /// Address, step and value of the last filled slot: what the padding repeats and what the
    /// segment hands to the next one.
    last_addr: u32,
    last_step: u64,
    last_value: [u32; 2],
    /// Distance from the segment's base / to the memory end, split in 16-bit halves.
    distance_base: [u16; 2],
    distance_end: [u16; 2],
}

/// Fills a `Mem` segment's rows, splitting the work into at most `n_ranges` parallel ranges.
///
/// Takes `rows` rather than a `MemTrace` so the equivalence test can run it over a handful of rows:
/// the generated `MemTrace` fixes 2^22 rows, which at 104 columns is 3.25 GB and unusable from a
/// test. `rows.len()` is the segment's row count, exactly as `trace.num_rows()` was.
fn fill_mem_trace<F: PrimeField64, R: MemTraceRowOps<F>>(
    rows: &mut [R],
    mem_ops: MemOps<'_>,
    seg: &MemModuleSegmentCheckPoint,
    previous_segment: &MemPreviousSegment,
    segment_id: SegmentId,
    is_last_segment: bool,
    n_ranges: usize,
) -> MemFillOutput {
    let lanes = lanes_of::<F, R>();
    let num_slots = lanes.slots(rows.len());
    let lanes_x_row = lanes.lanes();
    // `current_offsets` packs a virtual row plus two flag bits in a u32.
    debug_assert!(
        num_slots < OFFSET_USE_FLAG as usize,
        "MemSM: {num_slots} virtual rows do not fit in OFFSET_VALUE_MASK"
    );

    // use special counter for internal reads
    let distance_base = previous_segment.addr - RAM_W_ADDR_INIT;
    let previous_segment_addr = previous_segment.addr - (segment_id == 0) as u32;

    // One range per thread of the pool this witness computation is ALREADY running in: the
    // executor wraps the whole dispatch in `lease_pool(n_cores).install(...)`, so
    // `current_num_threads` is exactly the `n_cores` proofman handed over, and there is no
    // second knob to keep in step with it.
    phase_start!(t_split);
    let ranges = split_mem_slots(seg, num_slots, n_ranges);
    phase_end!(d_split, t_split);

    // Each range fills its own slots into the rows it owns outright, plus a scratch stand-in
    // for the row it shares with the previous range (see `RowView`). Nothing else is shared:
    // the offsets table fixes every address's slots before the fill starts, and a range owns
    // whole addresses, so each backward read the fill makes lands on a slot it wrote itself.
    phase_start!(t_fill);
    let fills: Vec<RangeFill<R>> = {
        // Rows `first_row + 1 ..= last_row` of each range, carved in one forward pass.
        // `split_mem_ops` guarantees range `i`'s `last_row` is at most range `i+1`'s
        // `first_row`, which is what makes these slices disjoint.
        let mut owned_rows: Vec<&mut [R]> = Vec::with_capacity(ranges.len());
        let mut rest: &mut [R] = rows;
        let mut base = 0usize;
        for range in &ranges {
            let first_row = range.slot_from / lanes_x_row;
            let last_row = (range.slot_to - 1) / lanes_x_row;
            let skip = (first_row + 1).saturating_sub(base).min(rest.len());
            let (_, tail) = rest.split_at_mut(skip);
            base += skip;
            let take = (last_row + 1).saturating_sub(base).min(tail.len());
            let (mine, tail) = tail.split_at_mut(take);
            base += take;
            owned_rows.push(mine);
            rest = tail;
        }
        ranges
            .par_iter()
            .zip(owned_rows.into_par_iter())
            .map(|(range, owned)| {
                fill_mem_range::<F, R>(
                    range,
                    owned,
                    mem_ops,
                    seg,
                    lanes,
                    num_slots,
                    previous_segment,
                    previous_segment_addr,
                    is_last_segment,
                )
            })
            .collect()
    };

    phase_start!(t_merge);
    phase_end!(d_fill, t_fill);

    // The shared leading rows, merged lane by lane: only the lanes the range actually filled,
    // so the lanes of that row belonging to its neighbours -- written straight into the trace
    // as their own rows -- are left alone. This is the only point where two ranges meet.
    for (range, fill) in ranges.iter().zip(fills.iter()) {
        let first_row = range.slot_from / lanes_x_row;
        let lane_from = range.slot_from - first_row * lanes_x_row;
        let lane_to = (range.slot_to - first_row * lanes_x_row).min(lanes_x_row);
        for lane in lane_from..lane_to {
            copy_mem_lane::<F, R>(&mut rows[first_row], &fill.head, lane);
        }
    }

    phase_end!(d_merge, t_merge);

    let last_slot = fills.iter().filter_map(|f| f.last_slot).max();

    // Per-range cost, gathered before the histograms are consumed by the reduction below.
    #[cfg(feature = "witness_timers")]
    let n_threads = rayon::current_num_threads();
    #[cfg(feature = "witness_timers")]
    let range_report: Vec<(usize, usize, u128)> = fills
        .iter()
        .zip(ranges.iter())
        .map(|(f, r)| (r.slots_len_any(), f.ops_filled, f.elapsed.as_micros()))
        .collect();

    phase_start!(t_reduce);

    // Multiplicities summed across ranges: one parallel pass per bucket, the shape `main_sm`
    // uses for its per-chunk range checks. With a single range its histograms already are the
    // answer and are moved out rather than summed.
    let (mut range_22bits, mut range_16bits) = match fills.len() {
        0 => (vec![0u32; 1 << 22], vec![0u32; 1 << 16]),
        1 => {
            let f = fills.into_iter().next().unwrap();
            (f.range_22bits, f.range_16bits)
        }
        _ => {
            let h22: Vec<u32> = (0..1usize << 22)
                .into_par_iter()
                .map(|i| fills.iter().map(|f| f.range_22bits[i]).sum())
                .collect();
            let h16: Vec<u32> = (0..1usize << 16)
                .into_par_iter()
                .map(|i| fills.iter().map(|f| f.range_16bits[i]).sum())
                .collect();
            (h22, h16)
        }
    };

    phase_start!(t_pad);
    phase_end!(d_reduce, t_reduce);

    // STEP3. Add dummy lanes to the output vector to fill the remaining virtual rows
    // PADDING: At end of memory fill with same addr, incrementing step, same value, sel = 0, rd
    // = 1, wr = 0
    //
    // The padding repeats the last slot the fill wrote. When no range wrote one -- a segment with
    // no operation at all, which the planner does not produce -- it repeats the hand-over from the
    // previous segment instead, and pads from slot 0. Reading slot 0 there would read a slot
    // nobody wrote, which on the recycled (non-zeroed) buffer is the previous instance's data.
    let (addr, step, low_value, high_value, first_pad_slot) = match last_slot {
        Some(last_slot_idx) => {
            let (last_row, last_lane) = lanes.split(last_slot_idx);
            let step = if !rows[last_row].get_sel_dual(last_lane) {
                rows[last_row].get_step(last_lane)
            } else {
                rows[last_row].get_step_dual(last_lane)
            };
            (
                rows[last_row].get_addr(last_lane),
                step,
                rows[last_row].get_value(last_lane, 0),
                rows[last_row].get_value(last_lane, 1),
                last_slot_idx + 1,
            )
        }
        None => (
            previous_segment.addr,
            previous_segment.step,
            previous_segment.value as u32,
            (previous_segment.value >> 32) as u32,
            0,
        ),
    };
    let padding_size = num_slots - first_pad_slot;
    if padding_size > 0 {
        // Every padding slot repeats the same values, so the row holding them is built once and
        // the whole rows are overwritten in parallel; only the row the last operation shares
        // with the padding has its lanes set one at a time.
        let partial_end = first_pad_slot.next_multiple_of(lanes_x_row).min(num_slots);
        for islot in first_pad_slot..partial_end {
            let (row, lane) = lanes.split(islot);
            set_mem_padding_lane::<F, R>(&mut rows[row], lane, addr, step, low_value, high_value);
        }
        let from_row = partial_end / lanes_x_row;
        if from_row < rows.len() {
            let mut pad_row = R::default();
            for lane in 0..lanes_x_row {
                set_mem_padding_lane::<F, R>(&mut pad_row, lane, addr, step, low_value, high_value);
            }
            rows[from_row..].par_iter_mut().for_each(|row| *row = pad_row);
        }
    }

    phase_end!(d_pad, t_pad);

    if padding_size > 0 {
        // Store the padding range checks
        range_16bits[0] += padding_size as u32;
        range_22bits[0] += padding_size as u32;
    }

    // One line per instance. `ops` is what each range actually filled out of the whole list every
    // range has to walk (`mem_ops` is unsorted, so a range cannot skip ahead); the gap between the
    // slowest and the fastest range is what caps the speedup.
    #[cfg(feature = "witness_timers")]
    {
        let slowest = range_report.iter().map(|r| r.2).max().unwrap_or(0) as f64 / 1e3;
        let fastest = range_report.iter().map(|r| r.2).min().unwrap_or(0) as f64 / 1e3;
        let ops_filled: usize = range_report.iter().map(|r| r.1).sum();
        phase_log!(
            "Mem[{}] fill: {} pool threads -> {} ranges | {} of {} ops | \
             split {:.2}ms fill {:.0}ms (slowest range {:.0}ms, fastest {:.0}ms) \
             merge {:.2}ms reduce {:.0}ms pad {:.0}ms | padding {} of {} slots",
            usize::from(segment_id),
            n_threads,
            range_report.len(),
            ops_filled,
            mem_ops.len(),
            phase_ms!(d_split),
            phase_ms!(d_fill),
            slowest,
            fastest,
            phase_ms!(d_merge),
            phase_ms!(d_reduce),
            phase_ms!(d_pad),
            padding_size,
            num_slots,
        );
        phase_log!(
            "Mem[{}] ranges (slots, ops, ms): {}",
            usize::from(segment_id),
            range_report
                .iter()
                .map(|(s, o, us)| format!("({}, {}, {:.0})", s, o, *us as f64 / 1e3))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }

    // no add extra +1 because index = value - 1
    // RAM_W_ADDR_END - last_addr + 1 - 1 = RAM_W_ADDR_END - last_addr
    let distance_end = RAM_W_ADDR_END - addr;

    // Add one in range_check_data_max because it's used by intermediate reads, and reads
    // add one to distance to allow same step on read operations.

    let distance_base = [distance_base as u16, (distance_base >> 16) as u16];
    let distance_end = [distance_end as u16, (distance_end >> 16) as u16];

    range_16bits[distance_base[0] as usize] += 1;
    range_16bits[distance_base[1] as usize] += 1;
    range_16bits[distance_end[0] as usize] += 1;
    range_16bits[distance_end[1] as usize] += 1;

    MemFillOutput {
        range_22bits,
        range_16bits,
        last_addr: addr,
        last_step: step,
        last_value: [low_value, high_value],
        distance_base,
        distance_end,
    }
}

/// Fills one range's slots. Pure by design -- no `&self`, no `MemSM` state -- which is what lets
/// the equivalence test run the same fill with one range and with many and compare the traces.
#[allow(clippy::too_many_arguments)]
fn fill_mem_range<F: PrimeField64, R: MemTraceRowOps<F>>(
    range: &MemFillRange,
    owned: &mut [R],
    mem_ops: MemOps<'_>,
    seg: &MemModuleSegmentCheckPoint,
    lanes: MemLanes,
    num_slots: usize,
    previous_segment: &MemPreviousSegment,
    previous_segment_addr: u32,
    is_last_segment: bool,
) -> RangeFill<R> {
    phase_start!(started);
    #[cfg(feature = "witness_timers")]
    let mut ops_filled = 0usize;
    let lanes_x_row = lanes.lanes();
    let first_row = range.slot_from / lanes_x_row;
    let mut rows = RowView { head: R::default(), owned, first_row };

    let mut range_22bits: Vec<u32> = vec![0; 1 << 22];
    let mut range_16bits: Vec<u32> = vec![0; 1 << 16];

    // Address cursors, this range's addresses only: `current_offsets[addr_index - addr_base]`.
    let addr_base = range.addr_from as usize;
    let addr_limit = range.addr_to as usize;
    let mut current_offsets = vec![0u32; (range.addr_to - range.addr_from) as usize];

    // A range must not write past its own slots. With a single range this is `num_slots`, so the
    // guard below is the one the sequential fill had.
    let slot_limit = range.slot_to;
    let offset_base_addr_w = seg.offsets_base_addr >> 3;

    #[cfg(feature = "debug_mem")]
    let mut filled_slots = vec![false; num_slots];
    #[cfg(not(feature = "debug_mem"))]
    let _ = (num_slots, is_last_segment);

    // `None` until this range writes a slot: a range whose addresses carry no operation must
    // not raise the segment's last slot.
    let mut last_slot_idx: Option<usize> = None;

    // The address with offset 0 is the halo address, but point of view of continuations halo doesn't
    // implies a addr_changes, for this reason init current_offsets[0] with OFFSET_USE_FLAG
    if addr_base == 0 && seg.offset_at(0) == 0 {
        current_offsets[0] = OFFSET_USE_FLAG;
    }
    for (index, mem_op) in mem_ops.iter().enumerate() {
        let step = mem_op.step;

        let addr_index = (mem_op.addr - offset_base_addr_w) as usize;
        // Operations outside this range's addresses belong to another range. Every range walks the
        // whole list because `mem_ops` is NOT sorted: the offsets table is what lets the fill place
        // an operation straight into its slot, in any order.
        if addr_index < addr_base || addr_index >= addr_limit {
            continue;
        }
        #[cfg(feature = "witness_timers")]
        {
            ops_filled += 1;
        }
        // The most significant bit of current_offsets is used to indicate whether the dual slot is available for this address
        let mut dual_available = current_offsets[addr_index - addr_base] & OFFSET_DUAL_FLAG != 0;
        let addr_changes = current_offsets[addr_index - addr_base] == 0;
        // `islot` is a virtual row: the offsets table is expressed in these units, so the
        // physical row and the lane inside it come from `lanes.split(islot)`.
        let mut islot = if addr_changes {
            let off_val = seg.offset_at(addr_index as u32);
            debug_assert!(off_val > 0, "MemSM: Address 0x{:X} at index {index} is out of offsets range, offset_base_addr_w: 0x{:X}",
                    mem_op.addr * 8, offset_base_addr_w * 8);
            off_val as usize - 1
        } else {
            (current_offsets[addr_index - addr_base] & OFFSET_VALUE_MASK) as usize
        };

        let mut init_row = false;
        let increment = if mem_op.is_write {
            let _increment = if dual_available {
                // A write goes to a new lane; if dual_available is true it means that
                // the current lane is already occupied and therefore the write goes to a new lane.
                // dual_available also means that addr_changes must be false
                debug_assert!(
                    !addr_changes,
                    "MemSM: dual_available && addr_changes (addr: 0x{:X} index: {index})",
                    mem_op.addr * 8
                );
                // fill sel_dual and step_dual for not dual lane
                dual_available = false;
                islot += 1;
                step - if islot == 0 {
                    previous_segment.step
                } else {
                    let (prow, plane) = lanes.split(islot - 1);
                    rows.at(prow).get_step(plane)
                } - 1
            } else if addr_changes {
                // First access to this address → new lane. The previous
                // distinct address comes from the sparse change-point
                // table (was a backward linear scan on the dense
                // offsets array before the SoA refactor).
                let previous_addr_w = seg
                    .previous_change_addr_w(addr_index as u32)
                    .unwrap_or(previous_segment_addr as u64);

                debug_assert!(
                    previous_addr_w < mem_op.addr as u64,
                    "MemSM: Warning: address goes back \
                          or no change (on addr_changes path) from 0x{:X} to 0x{:X} \
                          at slot {islot} (offset_base_addr_w: 0x{:X})",
                    mem_op.addr * 8,
                    previous_addr_w * 8,
                    offset_base_addr_w * 8
                );

                mem_op.addr as u64 - previous_addr_w - 1
            } else {
                // How addr_changes is false, means that the previous lane belongs to same address,
                // in this case, how the inputs are in natural time order, we could read from
                // previous lane the "last step", if dual the dual_step, else the step.

                let previous_step = if islot == 0 {
                    previous_segment.step
                } else {
                    let (prow, plane) = lanes.split(islot - 1);
                    if rows.at(prow).get_sel_dual(plane) {
                        rows.at(prow).get_step_dual(plane)
                    } else {
                        rows.at(prow).get_step(plane)
                    }
                };
                if step <= previous_step {
                    panic!("MemSM: Warning: step {step} is not greater than previous_step {previous_step} \
                            for write operation at index {index} and slot {islot} with addr 0x{:X} \
                            (addr_index: {addr_index} previous_segment.addr: 0x{:X} offset_base_addr_w: 0x{:X})",
                        mem_op.addr * 8, previous_segment_addr * 8, offset_base_addr_w * 8);
                }
                step - previous_step - 1
            };
            current_offsets[addr_index - addr_base] = (islot as u32) | OFFSET_DUAL_FLAG;
            init_row = true;
            _increment
        } else if addr_changes {
            // It's the first address access, it's a read, means no dual.
            debug_assert!(
                !dual_available,
                "MemSM: addr_changes && dual_available (addr: 0x{:X} index: {index})",
                mem_op.addr * 8
            );
            current_offsets[addr_index - addr_base] = (islot as u32) | OFFSET_DUAL_FLAG;
            // dual available
            init_row = true;

            let previous_addr: u64 = seg
                .previous_change_addr_w(addr_index as u32)
                .unwrap_or(previous_segment_addr as u64);
            debug_assert!(
                previous_addr < mem_op.addr as u64,
                "MemSM: Warning: address goes back \
                          or no change (on addr_changes path) from 0x{:X} to 0x{:X} \
                          at slot {islot} (offset_base_addr_w: 0x{:X})",
                mem_op.addr * 8,
                previous_addr * 8,
                offset_base_addr_w * 8
            );
            mem_op.addr as u64 - previous_addr - 1
        } else if dual_available {
            // It's dual read, not addr_changes.
            let (row, lane) = lanes.split(islot);
            let prev_step = rows.at(row).get_step(lane);
            // But duals must be in the same chunk, otherwise I can't use
            if MemHelpers::mem_steps_belongs_to_same_chunk(prev_step, step) {
                rows.at(row).set_sel_dual(lane, true);
                rows.at(row).set_step_dual(lane, step);
                current_offsets[addr_index - addr_base] = islot as u32 + 1;
                step - prev_step - if rows.at(row).get_wr(lane) { 1 } else { 0 }
            } else {
                dual_available = false;
                islot += 1;
                init_row = true;
                current_offsets[addr_index - addr_base] = (islot as u32) | OFFSET_DUAL_FLAG;
                step - prev_step
            }
        } else {
            current_offsets[addr_index - addr_base] = (islot as u32) | OFFSET_DUAL_FLAG;
            // dual available
            init_row = true;
            if islot >= slot_limit {
                break;
            }
            // set specific values of trace for regular memory operation
            step - if islot == 0 {
                previous_segment.step
            } else {
                let (prow, plane) = lanes.split(islot - 1);
                if rows.at(prow).get_sel_dual(plane) {
                    rows.at(prow).get_step_dual(plane)
                } else {
                    rows.at(prow).get_step(plane)
                }
            }
        };
        if init_row {
            if islot >= slot_limit {
                break;
            }
            #[cfg(feature = "debug_mem")]
            {
                if filled_slots[islot] {
                    break;
                }
                filled_slots[islot] = true;
            }
            let (row, lane) = lanes.split(islot);
            // always set dual to false because we don't know if there will dual reads, maybe
            // this is the last access to this address in this segment.
            rows.at(row).set_sel_dual(lane, false);
            rows.at(row).set_step_dual(lane, 0);
            rows.at(row).set_addr(lane, mem_op.addr);
            rows.at(row).set_step(lane, step);
            rows.at(row).set_sel(lane, true);
            rows.at(row).set_addr_changes(lane, addr_changes);
            rows.at(row).set_wr(lane, mem_op.is_write);
            let (low_val, high_val) = (mem_op.value as u32, (mem_op.value >> 32) as u32);
            rows.at(row).set_value(lane, 0, low_val);
            rows.at(row).set_value(lane, 1, high_val);
            // rows.at(row).set_read_same_addr(lane, (addr_changes || mem_op.is_write) == false);
            // range check between lanes
            let increment = increment as usize;
            let l_increment = increment & ((1 << 22) - 1);
            let h_increment = increment >> 22;
            rows.at(row).set_l_increment(lane, l_increment as u32);
            rows.at(row).set_h_increment(lane, h_increment as u16);

            range_22bits[l_increment] += 1;
            range_16bits[h_increment] += 1;
        }
        // rows.at(row).set_previous_step(lane, ...)
        if dual_available {
            // range check dual
            range_22bits[increment as usize] += 1;
        }
        if last_slot_idx.map_or(true, |last| islot > last) {
            last_slot_idx = Some(islot);
        }
    }
    #[cfg(feature = "debug_mem")]
    {
        // Per range now, not per segment. NOTE: as written this can only ever compare against
        // `filled_slots[0]` -- neither `prev_filled_slot` nor `from_slot` is updated in the loop --
        // so it is far weaker than it looks; kept as it was rather than changed here.
        let prev_filled_slot = filled_slots[range.slot_from];
        let from_slot = range.slot_from;
        let count =
            if is_last_segment { last_slot_idx.unwrap_or(range.slot_from) } else { range.slot_to };
        for (i, filled) in filled_slots.iter().enumerate().take(count).skip(range.slot_from) {
            debug_assert!(
                *filled == prev_filled_slot,
                "MemSM: not complete instance found [{}..{}] = {}",
                from_slot,
                i - 1,
                prev_filled_slot
            );
        }
    }
    RangeFill {
        head: rows.head,
        last_slot: last_slot_idx,
        range_22bits,
        range_16bits,
        #[cfg(feature = "witness_timers")]
        ops_filled,
        #[cfg(feature = "witness_timers")]
        elapsed: started.elapsed(),
    }
}

#[cfg(test)]
#[path = "mem_sm_fill_tests.rs"]
mod fill_tests;
