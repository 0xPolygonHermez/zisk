use std::sync::Arc;
use zisk_common::SegmentId;
use zisk_pil::MemAirValues;
#[cfg(not(feature = "packed"))]
use zisk_pil::MemTrace;
#[cfg(feature = "packed")]
use zisk_pil::MemTracePacked;

#[cfg(feature = "packed")]
type MemTraceType<F> = MemTracePacked<F>;

#[cfg(not(feature = "packed"))]
type MemTraceType<F> = MemTrace<F>;
#[cfg(feature = "debug_mem")]
use {
    num_bigint::ToBigInt,
    std::{
        env,
        fs::File,
        io::{BufWriter, Write},
    },
};

use crate::{MemInput, MemModule};
use fields::PrimeField64;
use mem_common::{
    MemHelpers, MEM_INC_C_BITS, MEM_INC_C_MASK, MEM_INC_C_MAX_RANGE, MEM_INC_C_SIZE,
    RAM_W_ADDR_END, RAM_W_ADDR_INIT,
};
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace};
use zisk_core::{RAM_ADDR, RAM_SIZE};

const DUAL_RANGE_MAX: usize = (1 << 24) - 1;
const DUAL_PARTIAL_RANGE_MAX: usize = 1 << 20;

pub struct MemSM<F: PrimeField64> {
    /// PIL2 standard library
    std: Arc<Std<F>>,
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
        Arc::new(Self { std: std.clone() })
    }

    pub fn get_to_addr() -> u32 {
        (RAM_ADDR + RAM_SIZE - 1) as u32
    }
    #[cfg(feature = "debug_mem")]
    pub fn save_to_file(trace: &MemTrace<F>, file_name: &str) {
        println!("[MemDebug] writing information {} .....", file_name);
        let file = File::create(file_name).unwrap();
        let mut writer = BufWriter::new(file);
        let num_rows = MemTrace::NUM_ROWS;

        for i in 0..num_rows {
            let addr = trace[i].addr.as_canonical_biguint().to_bigint().unwrap() * 8;
            let step = trace[i].step.as_canonical_biguint().to_bigint().unwrap();
            writeln!(
                writer,
                "{:#010X} {} {} {:?}",
                addr, trace[i].step, trace[i].wr, trace[i].value
            )
            .unwrap();
        }
        println!("[MemDebug] done");
    }
}

impl<F: PrimeField64> MemModule<F> for MemSM<F> {
    fn get_addr_range(&self) -> (u32, u32) {
        (RAM_W_ADDR_INIT, RAM_W_ADDR_END)
    }
    fn is_dual(&self) -> bool {
        true
    }
    /// Finalizes the witness accumulation process and triggers the proof generation.
    ///
    /// This method is invoked by the executor when no further witness data remains to be added.
    ///
    /// # Parameters
    ///
    /// - `mem_inputs`: A slice of all `MemoryInput` inputs
    fn compute_witness(
        &self,
        mem_ops: &[MemInput],
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
    ) -> AirInstance<F> {
        let mut trace = MemTraceType::<F>::new_from_vec(trace_buffer);

        let std = self.std.clone();

        let range_id = std.get_range_id(0, MEM_INC_C_MAX_RANGE as i64, None);
        let mut range_check_data: Vec<u32> = vec![0; MEM_INC_C_SIZE];

        // 2^20 * 2 = 2^21 = 2MB
        let dual_range_id = std.get_range_id(0, DUAL_RANGE_MAX as i64, None);
        let mut dual_partial_range: Vec<u16> = vec![0; DUAL_PARTIAL_RANGE_MAX];

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
            // if step >= 28184622 && step <= 28184624 {
            //     println!(
            //         "@@@@@@@@@@ 0x{:08X} {step} 8 OP:{}",
            //         mem_op.addr * 8,
            //         if mem_op.is_write { 2 } else { 1 }
            //     );
            // }

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
                    if increment_step >= DUAL_PARTIAL_RANGE_MAX as u64 {
                        self.std.range_check(dual_range_id, increment_step as i64, 1);
                    } else if dual_partial_range[increment_step as usize] == u16::MAX {
                        dual_partial_range[increment_step as usize] = 0;
                        self.std.range_check(
                            dual_range_id,
                            increment_step as i64,
                            u16::MAX as u64 + 1,
                        );
                    } else {
                        dual_partial_range[increment_step as usize] += 1;
                    }

                    // TODO: add to range check
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
            trace[i].set_value(0, low_val);
            trace[i].set_value(1, high_val);

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
            let lsb_increment = increment & MEM_INC_C_MASK;
            let msb_increment = increment >> MEM_INC_C_BITS;
            trace[i].set_increment(0, lsb_increment as u32);
            trace[i].set_increment(1, msb_increment as u32);
            trace[i].set_wr(mem_op.is_write);

            #[cfg(feature = "debug_mem")]
            if (lsb_increment >= MEM_INC_C_SIZE) || (msb_increment > MEM_INC_C_SIZE) {
                panic!("MemSM: increment's out of range: {} i:{} addr_changes:{} mem_op.addr:0x{:X} last_addr:0x{:X} mem_op.step:{} last_step:{}",
                    increment, i, addr_changes as u8, mem_op.addr, last_addr, mem_op.step, last_step);
            }

            range_check_data[lsb_increment] += 1;
            range_check_data[msb_increment] += 1;

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

        let padding_size = trace.num_rows() - count;
        for i in count..trace.num_rows() {
            trace[i].set_previous_step(step);
            trace[i].set_addr(addr);
            trace[i].set_step(step);
            trace[i].set_sel(false);
            trace[i].set_wr(false);

            let low_value = last_row.get_value(0);
            trace[i].set_value(0, low_value);
            let high_value = last_row.get_value(1);
            trace[i].set_value(1, high_value);

            trace[i].set_addr_changes(false);
            trace[i].set_increment(0, 0);
            trace[i].set_increment(1, 0);
            trace[i].set_read_same_addr(true);
            trace[i].set_sel_dual(false);
            trace[i].set_step_dual(0);
        }

        if padding_size > 0 {
            // Store the padding range checks
            range_check_data[0] += (2 * padding_size) as u32;
        }

        // no add extra +1 because index = value - 1
        // RAM_W_ADDR_END - last_addr + 1 - 1 = RAM_W_ADDR_END - last_addr
        let distance_end = RAM_W_ADDR_END - last_addr;

        self.std.range_checks(range_id, range_check_data);

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

        let range_16bits_id = std.get_range_id(0, 0xFFFF, None);

        self.std.range_check(range_16bits_id, distance_base[0] as i64, 1);
        self.std.range_check(range_16bits_id, distance_base[1] as i64, 1);
        self.std.range_check(range_16bits_id, distance_end[0] as i64, 1);
        self.std.range_check(range_16bits_id, distance_end[1] as i64, 1);

        for (value, count) in dual_partial_range.iter().enumerate() {
            if *count == 0 {
                continue;
            }
            self.std.range_check(dual_range_id, value as i64, *count as u64);
        }

        #[cfg(feature = "debug_mem")]
        {
            let path = env::var("MEM_TRACE_DIR").unwrap_or("tmp/mem_trace".to_string());
            let filename = format!("{path}/mem_trace_{segment_id:04}.txt");
            println!("Saving {filename}");
            Self::save_to_file(&trace, &filename);
            println!("[Mem:{}] mem_ops:{} padding:{}", segment_id, mem_ops.len(), padding_size);
        }
        AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values))
    }
}
