use std::sync::Arc;

#[cfg(feature = "debug_mem")]
use num_bigint::ToBigInt;
#[cfg(feature = "debug_mem")]
use std::{
    fs::File,
    io::{BufWriter, Write},
};
use zisk_common::SegmentId;

use crate::{
    MemHelpers, MemInput, MemModule, MEMORY_MAX_DIFF, MEM_BYTES_BITS, STEP_MEMORY_LIMIT_TO_VERIFY,
    STEP_MEMORY_MAX_DIFF,
};
use p3_field::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace};

use zisk_core::{RAM_ADDR, RAM_SIZE};
use zisk_pil::{MemAirValues, MemTrace};

pub const RAM_W_ADDR_INIT: u32 = RAM_ADDR as u32 >> MEM_BYTES_BITS;
pub const RAM_W_ADDR_END: u32 = (RAM_ADDR + RAM_SIZE - 1) as u32 >> MEM_BYTES_BITS;

const _: () = {
    assert!(
        (RAM_ADDR + RAM_SIZE - 1) <= 0xFFFF_FFFF,
        "RAM memory exceeds the 32-bit addressable range"
    );
};

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
    pub fn save_to_file(&self, trace: &MemTrace<F>, file_name: &str) {
        println!("[MemDebug] writing information {} .....", file_name);
        let file = File::create(file_name).unwrap();
        let mut writer = BufWriter::new(file);
        let num_rows = MemTrace::<usize>::NUM_ROWS;

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
    ) -> AirInstance<F> {
        let mut trace = MemTrace::<F>::new();

        debug_assert!(
            !mem_ops.is_empty() && mem_ops.len() <= trace.num_rows,
            "MemSM Inputs too large segment_id:{} mem_ops:{} rows:{}  [0]{:?} [last]:{:?}",
            segment_id,
            mem_ops.len(),
            trace.num_rows,
            mem_ops[0],
            mem_ops[mem_ops.len() - 1],
        );

        let std = self.std.clone();

        let range_id = std.get_range(1, MEMORY_MAX_DIFF as i64, None);
        let mut range_check_data: Vec<u16> = vec![0; MEMORY_MAX_DIFF as usize];
        let f_range_check_max_value = 0xFFFF + 1;

        // use special counter for internal reads
        let mut range_check_data_max = 0u64;
        let mut range_check_data_min = 0u64;

        // index it's value - 1, for this reason no add +1
        range_check_data[(previous_segment.addr - RAM_W_ADDR_INIT) as usize] += 1;

        println!(
            "[MemSM] segment_id:{} mem_ops:{} rows:{}  [0]{:?} previous_segment:{:?}",
            segment_id,
            mem_ops.len(),
            trace.num_rows,
            mem_ops[0],
            previous_segment
        );
        let mut last_addr: u32 = previous_segment.addr;
        let mut last_step: u64 = previous_segment.step;
        let mut last_value: u64 = previous_segment.value;

        let mut i = 0;
        let mut increment;

        // f_max_increment it's plus 1 because on read operations we increment the step
        // difference in one, to allow read the same address with "same" step
        let f_max_increment = F::from_u64(STEP_MEMORY_MAX_DIFF + 1);

        #[cfg(feature = "debug_mem")]
        let mut _mem_op_done = 0;

        for mem_op in mem_ops {
            let mut step = mem_op.step;

            if i >= trace.num_rows {
                break;
            }

            // set the common values of trace between internal reads and regular memory operation
            trace[i].addr = F::from_u32(mem_op.addr);
            let addr_changes = last_addr != mem_op.addr;
            trace[i].addr_changes = if addr_changes { F::ONE } else { F::ZERO };

            if addr_changes {
                increment = (mem_op.addr - last_addr) as u64;
            } else {
                increment = step - last_step;
                if increment as usize >= STEP_MEMORY_LIMIT_TO_VERIFY {
                    // could be that no has internal reads, but need to check.
                    if let Some((mut full_rows, mut zero_row)) =
                        MemHelpers::get_intermediate_rows(last_step, step)
                    {
                        let internal_reads = full_rows + zero_row;
                        let incomplete = (i + internal_reads as usize) >= trace.num_rows;

                        // check if has enough rows to complete the internal reads + regular memory
                        if (i + internal_reads as usize) > trace.num_rows {
                            full_rows = (trace.num_rows - i) as u64;
                            zero_row = 0;
                        }

                        // without address changes, the internal reads before write must use the last
                        // value, in the case of reads value and the last value are the same
                        let (low_val, high_val) = (last_value as u32, (last_value >> 32) as u32);
                        trace[i].value = [F::from_u32(low_val), F::from_u32(high_val)];

                        // it's intenal
                        trace[i].sel = F::ZERO;

                        // in internal reads the increment is always the max increment
                        if full_rows > 0 {
                            trace[i].increment = f_max_increment;
                            // set step as max increment from last_step
                            step = last_step + STEP_MEMORY_MAX_DIFF;
                        } else {
                            // in case of zero_row = 1, full_rows = 0
                            trace[i].increment = F::ZERO;
                            step = last_step;
                        }

                        // internal reads always must be read
                        trace[i].wr = F::ZERO;

                        // setting step on trace
                        trace[i].step = F::from_u64(step);

                        // update last_step and increment step
                        last_step = step;

                        i += 1;

                        // the trace values of the rest of internal reads are equal to previous, only
                        // change the value of step
                        for _j in 1..full_rows {
                            trace[i] = trace[i - 1];
                            step += STEP_MEMORY_MAX_DIFF;
                            trace[i].step = F::from_u64(step);
                            last_step = step;
                            i += 1;
                        }
                        if zero_row > 0 {
                            // row with zero increment, step was the same, necessary extra row added
                            // because counters don't has information about previous or last step for
                            // each address
                            trace[i] = trace[i - 1];
                            // increment zero is allowed, because when operation is a read, increase
                            // in one the increment. With this feature a mem position can be read multiple
                            // times in same mem-step (timestamp)
                            trace[i].increment = F::ZERO;
                            range_check_data_min += 1;
                            i += 1;
                        }

                        range_check_data_max += full_rows;

                        // control the edge case when there aren't enough rows to complete the internal
                        // reads or regular memory operation
                        if incomplete {
                            last_addr = mem_op.addr;
                            break;
                        }
                        step = mem_op.step;
                        increment = step - last_step;

                        // copy last trace for the regular memory operation (addr, addr_changes)
                        trace[i] = trace[i - 1];
                    }
                }
            }

            if i >= trace.num_rows {
                break;
            }
            // set specific values of trace for regular memory operation
            let (low_val, high_val) = (mem_op.value as u32, (mem_op.value >> 32) as u32);
            trace[i].value = [F::from_u32(low_val), F::from_u32(high_val)];

            trace[i].step = F::from_u64(step);
            trace[i].sel = F::ONE;

            if !addr_changes && !mem_op.is_write {
                // in case of read operations of same address, add one to allow many reads
                // over same address and step
                increment += 1;
            }
            trace[i].increment = F::from_u64(increment);
            trace[i].wr = F::from_bool(mem_op.is_write);

            #[cfg(feature = "debug_mem")]
            {
                _mem_op_done += 1;
            }

            // Store the value of incremenet so it can be range checked
            let range_index = increment as usize - 1;
            if range_index < MEMORY_MAX_DIFF as usize {
                if range_check_data[range_index] == 0xFFFF {
                    range_check_data[range_index] = 0;
                    std.range_check(increment as i64, f_range_check_max_value, range_id);
                } else {
                    range_check_data[range_index] += 1;
                }
            } else {
                panic!("MemSM: increment's out of range: {} i:{} addr_changes:{} mem_op.addr:0x{:X} last_addr:0x{:X} mem_op.step:{} last_step:{}",
                    increment, i, addr_changes as u8, mem_op.addr, last_addr, mem_op.step, last_step);
            }

            last_addr = mem_op.addr;
            last_step = step;
            last_value = mem_op.value;
            i += 1;
        }
        let count = i;

        // STEP3. Add dummy rows to the output vector to fill the remaining rows
        // PADDING: At end of memory fill with same addr, incrementing step, same value, sel = 0, rd
        // = 1, wr = 0
        let last_row_idx = count - 1;
        let addr = trace[last_row_idx].addr;
        let value = trace[last_row_idx].value;

        // Two situations with padding, at end of all segments, where there aren't more operations,
        // in this case we increment step one-by-one. The second situation is in the middle of
        // padding between step with distance too large, in this case we increment with maximum
        // allowed distance.
        let padding_size = trace.num_rows - count;
        let padding_step = if is_last_segment { 1 } else { STEP_MEMORY_MAX_DIFF };
        let padding_increment = F::from_u64(padding_step + 1);
        for i in count..trace.num_rows {
            last_step += padding_step;
            trace[i].addr = addr;
            trace[i].step = F::from_u64(last_step);
            trace[i].sel = F::ZERO;
            trace[i].wr = F::ZERO;

            trace[i].value = value;

            trace[i].addr_changes = F::ZERO;
            trace[i].increment = padding_increment;
        }
        if padding_size > 0 {
            // Store the padding range checks
            self.std.range_check((padding_step + 1) as i64, padding_size as u64, range_id);
        }

        // no add extra +1 because index = value - 1
        // RAM_W_ADDR_END - last_addr + 1 - 1 = RAM_W_ADDR_END - last_addr
        range_check_data[(RAM_W_ADDR_END - last_addr) as usize] += 1; // TODO

        for (value, &multiplicity) in range_check_data.iter().enumerate() {
            if multiplicity == 0 {
                continue;
            }
            self.std.range_check((value + 1) as i64, multiplicity as u64, range_id);
        }
        // Add one in range_check_data_max because it's used by intermediate reads, and reads
        // add one to distance to allow same step on read operations.

        if range_check_data_max > 0 {
            self.std.range_check((STEP_MEMORY_MAX_DIFF + 1) as i64, range_check_data_max, range_id);
        }
        if range_check_data_min > 0 {
            self.std.range_check(1, range_check_data_min, range_id);
        }

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

        println!("MEM air_values: {:?}", air_values);

        #[cfg(feature = "debug_mem")]
        {
            self.save_to_file(&trace, &format!("/tmp/mem_trace_{}.txt", segment_id));
            println!(
                "[Mem:{}] mem_ops:{}/{} padding:{}",
                segment_id,
                _mem_op_done,
                mem_ops.len(),
                padding_size
            );
        }

        AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values))
    }
}
