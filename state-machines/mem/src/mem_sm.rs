use std::sync::Arc;

use crate::{MemInput, MemModule, MEMORY_MAX_DIFF, MEM_BYTES_BITS};
use num_bigint::BigInt;
use p3_field::PrimeField;
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace};

use zisk_core::{RAM_ADDR, RAM_SIZE};
use zisk_pil::{MemAirValues, MemTrace};

pub const RAM_W_ADDR_INIT: u32 = RAM_ADDR as u32 >> MEM_BYTES_BITS;
pub const RAM_W_ADDR_END: u32 = (RAM_ADDR + RAM_SIZE - 1) as u32 >> MEM_BYTES_BITS;

const _: () = {
    // assert!((RAM_SIZE - 1) >> MEM_BYTES_BITS <= MEMORY_MAX_DIFF, "RAM is too large");
    assert!(
        (RAM_ADDR + RAM_SIZE - 1) <= 0xFFFF_FFFF,
        "RAM memory exceeds the 32-bit addressable range"
    );
};

pub struct MemSM<F: PrimeField> {
    /// PIL2 standard library
    std: Arc<Std<F>>,
}

#[derive(Default)]
pub struct MemoryAirValues {
    pub segment_id: u32,
    pub is_first_segment: bool,
    pub is_last_segment: bool,
    pub previous_segment_addr: u32,
    pub previous_segment_step: u64,
    pub previous_segment_value: [u32; 2],
    pub segment_last_addr: u32,
    pub segment_last_step: u64,
    pub segment_last_value: [u32; 2],
}
#[derive(Debug, Default)]
pub struct MemPreviousSegment {
    pub addr: u32,
    pub step: u64,
    pub value: u64,
}

#[allow(unused, unused_variables)]
impl<F: PrimeField> MemSM<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self { std: std.clone() })
    }
    pub fn get_from_addr() -> u32 {
        RAM_ADDR as u32
    }
    pub fn get_to_addr() -> u32 {
        (RAM_ADDR + RAM_SIZE - 1) as u32
    }
}

impl<F: PrimeField> MemModule<F> for MemSM<F> {
    /// Finalizes the witness accumulation process and triggers the proof generation.
    ///
    /// This method is invoked by the executor when no further witness data remains to be added.
    ///
    /// # Parameters
    ///
    /// - `mem_inputs`: A slice of all `MemoryInput` inputs
    fn prove_instance(
        &self,
        mem_ops: &[MemInput],
        segment_id: usize,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
    ) -> AirInstance<F> {
        let mut trace = MemTrace::<F>::new();

        assert!(
            !mem_ops.is_empty() && mem_ops.len() <= trace.num_rows,
            "MemSM: mem_ops.len()={} out of range {}",
            mem_ops.len(),
            trace.num_rows,
        );

        let std = self.std.clone();
        let range_id = std.get_range(BigInt::from(1), BigInt::from(MEMORY_MAX_DIFF), None);
        let mut range_check_data = Box::new([0u16; MEMORY_MAX_DIFF as usize]);
        let f_range_check_max_value = F::from_canonical_u64(0xFFFF + 1);

        // use special counter for internal reads
        let mut range_check_data_max = 0u64;

        let mut air_values_mem = MemoryAirValues {
            segment_id: segment_id as u32,
            is_first_segment: segment_id == 0,
            is_last_segment,
            previous_segment_addr: previous_segment.addr,
            previous_segment_step: previous_segment.step,
            previous_segment_value: [
                previous_segment.value as u32,
                (previous_segment.value >> 32) as u32,
            ],
            ..MemoryAirValues::default()
        };

        // index it's value - 1, for this reason no add +1
        range_check_data[(previous_segment.addr - RAM_W_ADDR_INIT) as usize] += 1; // TODO

        let mut last_addr: u32 = previous_segment.addr;
        let mut last_step: u64 = previous_segment.step;
        let mut last_value: u64 = previous_segment.value;

        let mut i = 0;
        let mut increment;
        let f_max_increment = F::from_canonical_u64(MEMORY_MAX_DIFF);
        for mem_op in mem_ops.iter() {
            let mut step = mem_op.step;

            // set the common values of trace between internal reads and regular memory operation
            trace[i].addr = F::from_canonical_u32(mem_op.addr);
            let addr_changes = last_addr != mem_op.addr;
            trace[i].addr_changes = if addr_changes { F::one() } else { F::zero() };

            if addr_changes {
                increment = (mem_op.addr - last_addr) as u64;
            } else {
                increment = step - last_step;
                if increment > MEMORY_MAX_DIFF {
                    // calculate the number of internal reads
                    let mut internal_reads = (increment - 1) / MEMORY_MAX_DIFF;

                    // check if has enough rows to complete the internal reads + regular memory
                    let incomplete = (i + internal_reads as usize) >= trace.num_rows;
                    if incomplete {
                        internal_reads = (trace.num_rows - i) as u64;
                    }

                    // without address changes, the internal reads before write must use the last
                    // value, in the case of reads value and the last value are the same
                    let (low_val, high_val) = (last_value as u32, (last_value >> 32) as u32);
                    trace[i].value =
                        [F::from_canonical_u32(low_val), F::from_canonical_u32(high_val)];

                    // it's intenal
                    trace[i].sel = F::zero();

                    // in internal reads the increment is always the max increment
                    trace[i].increment = f_max_increment;

                    // internal reads always must be read
                    trace[i].wr = F::zero();

                    // setting step
                    trace[i].step = F::from_canonical_u64(step);
                    last_step = step;
                    step += MEMORY_MAX_DIFF;

                    i += 1;

                    // the trace values of the rest of internal reads are equal to previous, only
                    // change the value of step
                    for _j in 1..internal_reads {
                        trace[i] = trace[i - 1];
                        trace[i].step = F::from_canonical_u64(step);
                        last_step = step;
                        step += MEMORY_MAX_DIFF;
                        i += 1;
                    }

                    range_check_data_max += internal_reads;

                    // control the edge case when there aren't enough rows to complete the internal
                    // reads or regular memory operation
                    if incomplete {
                        last_addr = mem_op.addr;
                        break;
                    }
                    // copy last trace for the regular memory operation (addr, addr_changes)
                    trace[i] = trace[i - 1];
                    increment -= internal_reads * MEMORY_MAX_DIFF;
                }
            }

            // set specific values of trace for regular memory operation
            let (low_val, high_val) = (mem_op.value as u32, (mem_op.value >> 32) as u32);
            trace[i].value = [F::from_canonical_u32(low_val), F::from_canonical_u32(high_val)];

            trace[i].step = F::from_canonical_u64(step);
            trace[i].sel = F::one();
            trace[i].increment = F::from_canonical_u64(increment);
            trace[i].wr = F::from_bool(mem_op.is_write);
            i += 1;

            // Store the value of incremenet so it can be range checked
            let range_index = increment as usize - 1;
            if range_index < MEMORY_MAX_DIFF as usize {
                if range_check_data[range_index] == 0xFFFF {
                    range_check_data[range_index] = 0;
                    std.range_check(
                        F::from_canonical_u64(increment),
                        f_range_check_max_value,
                        range_id,
                    );
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

        let padding_size = trace.num_rows - count;
        for i in count..trace.num_rows {
            last_step += 1;
            trace[i].addr = addr;
            trace[i].step = F::from_canonical_u64(last_step);
            trace[i].sel = F::zero();
            trace[i].wr = F::zero();

            trace[i].value = value;

            trace[i].addr_changes = F::zero();
            trace[i].increment = F::one();
        }

        air_values_mem.segment_last_addr = last_addr;
        air_values_mem.segment_last_step = last_step;
        air_values_mem.segment_last_value[0] = last_value as u32;
        air_values_mem.segment_last_value[1] = (last_value >> 32) as u32;

        // Store the value of trivial increment so that they can be range checked
        // value = 1 => index = 0
        self.std.range_check(F::zero(), F::from_canonical_usize(padding_size), range_id);

        // no add extra +1 because index = value - 1
        // RAM_W_ADDR_END - last_addr + 1 - 1 = RAM_W_ADDR_END - last_addr
        range_check_data[(RAM_W_ADDR_END - last_addr) as usize] += 1; // TODO

        // TODO: Perform the range checks
        let range_id = self.std.get_range(BigInt::from(1), BigInt::from(MEMORY_MAX_DIFF), None);
        for (value, &multiplicity) in range_check_data.iter().enumerate() {
            if multiplicity == 0 {
                continue;
            }
            self.std.range_check(
                F::from_canonical_usize(value + 1),
                F::from_canonical_u16(multiplicity),
                range_id,
            );
        }
        self.std.range_check(
            f_range_check_max_value,
            F::from_canonical_u64(range_check_data_max),
            range_id,
        );

        let mut air_values = MemAirValues::<F>::new();
        air_values.segment_id = F::from_canonical_u32(air_values_mem.segment_id);
        air_values.is_first_segment = F::from_bool(air_values_mem.is_first_segment);
        air_values.is_last_segment = F::from_bool(air_values_mem.is_last_segment);
        air_values.previous_segment_step =
            F::from_canonical_u64(air_values_mem.previous_segment_step);
        air_values.previous_segment_addr =
            F::from_canonical_u32(air_values_mem.previous_segment_addr);
        air_values.segment_last_addr = F::from_canonical_u32(air_values_mem.segment_last_addr);
        air_values.segment_last_step = F::from_canonical_u64(air_values_mem.segment_last_step);
        let count = air_values_mem.previous_segment_value.len();
        for i in 0..count {
            air_values.previous_segment_value[i] =
                F::from_canonical_u32(air_values_mem.previous_segment_value[i]);
            air_values.segment_last_value[i] =
                F::from_canonical_u32(air_values_mem.segment_last_value[i]);
        }

        AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values))
    }
    fn get_addr_ranges(&self) -> Vec<(u32, u32)> {
        vec![(RAM_ADDR as u32, (RAM_ADDR + RAM_SIZE - 1) as u32)]
    }
}
