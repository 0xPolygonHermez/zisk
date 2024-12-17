use std::sync::Arc;

use crate::{MemInput, MemModule, MEMORY_MAX_DIFF, MEM_BYTES_BITS};
use num_bigint::BigInt;
use p3_field::PrimeField;
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace};

use zisk_core::{RAM_ADDR, RAM_SIZE};
use zisk_pil::{MemAirValues, MemTrace, MEM_AIR_IDS, ZISK_AIRGROUP_ID};

const RAM_W_ADDR_INIT: u32 = RAM_ADDR as u32 >> MEM_BYTES_BITS;
const RAM_W_ADDR_END: u32 = (RAM_ADDR + RAM_SIZE - 1) as u32 >> MEM_BYTES_BITS;

const _: () = {
    // assert!((RAM_SIZE - 1) >> MEM_BYTES_BITS <= MEMORY_MAX_DIFF, "RAM is too large");
    assert!(
        (RAM_ADDR + RAM_SIZE - 1) <= 0xFFFF_FFFF,
        "RAM memory exceeds the 32-bit addressable range"
    );
};

pub struct MemSM<F: PrimeField> {
    // STD
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
#[derive(Debug)]
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

    pub fn prove(&self, inputs: &[MemInput]) {
        // PRE: proxy calculate if exists jmp on step out-of-range, adding internal inputs
        // memory only need to process these special inputs, but inputs no change. At end of
        // inputs proxy add an extra internal input to jump to last address

        let air_rows = MemTrace::<F>::NUM_ROWS;

        // at least one row to go
        let count = inputs.len();
        let count_rem = count % air_rows;
        let num_segments = (count / air_rows) + if count_rem > 0 { 1 } else { 0 };

        let mut global_idxs = vec![0; num_segments];

        #[allow(clippy::needless_range_loop)]
        for i in 0..num_segments {
            // TODO: Review
            if let (true, global_idx) = self.std.pctx.dctx.write().unwrap().add_instance(
                ZISK_AIRGROUP_ID,
                MEM_AIR_IDS[0],
                1,
            ) {
                global_idxs[i] = global_idx;
            }
        }

        #[allow(clippy::needless_range_loop)]
        for segment_id in 0..num_segments {
            let is_last_segment = segment_id == num_segments - 1;
            let input_offset = segment_id * air_rows;
            let previous_segment = if (segment_id == 0) {
                MemPreviousSegment { addr: RAM_W_ADDR_INIT, step: 0, value: 0 }
            } else {
                MemPreviousSegment {
                    addr: inputs[input_offset - 1].addr,
                    step: inputs[input_offset - 1].step,
                    value: inputs[input_offset - 1].value,
                }
            };
            let input_end =
                if (input_offset + air_rows) > count { count } else { input_offset + air_rows };
            let mem_ops = &inputs[input_offset..input_end];

            let air_instance =
                self.prove_instance(mem_ops, segment_id, is_last_segment, &previous_segment);

            self.std
                .pctx
                .air_instance_repo
                .add_air_instance(air_instance, Some(global_idxs[segment_id]));
        }
    }

    /// Finalizes the witness accumulation process and triggers the proof generation.
    ///
    /// This method is invoked by the executor when no further witness data remains to be added.
    ///
    /// # Parameters
    ///
    /// - `mem_inputs`: A slice of all `MemoryInput` inputs
    pub fn prove_instance(
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

        // In a Mem AIR instance the first row is a dummy row used for the continuations between AIR
        // segments In a Memory AIR instance, the first row is reserved as a dummy row.
        // This dummy row is used to facilitate the continuation state between different AIR
        // segments. It ensures seamless transitions when multiple AIR segments are
        // processed consecutively. This design avoids discontinuities in memory access
        // patterns and ensures that the memory trace is continuous, For this reason we use
        // AIR num_rows - 1 as the number of rows in each memory AIR instance

        // Create a vector of Mem0Row instances, one for each memory operation
        // Recall that first row is a dummy row used for the continuations between AIR segments
        // The length of the vector is the number of input memory operations plus one because
        // in the prove_witnesses method we drain the memory operations in chunks of n - 1 rows

        let mut range_check_data: Vec<u64> = vec![0; MEMORY_MAX_DIFF as usize];

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

        // Fill the remaining rows
        let mut last_addr: u32 = previous_segment.addr;
        let mut last_step: u64 = previous_segment.step;
        let mut last_value: u64 = previous_segment.value;

        for (i, mem_op) in mem_ops.iter().enumerate() {
            trace[i].addr = F::from_canonical_u32(mem_op.addr);
            trace[i].step = F::from_canonical_u64(mem_op.step);
            trace[i].sel = F::from_bool(!mem_op.is_internal);
            trace[i].wr = F::from_bool(mem_op.is_write);

            let (low_val, high_val) = (mem_op.value as u32, (mem_op.value >> 32) as u32);
            trace[i].value = [F::from_canonical_u32(low_val), F::from_canonical_u32(high_val)];

            let addr_changes = last_addr != mem_op.addr;
            trace[i].addr_changes = if addr_changes { F::one() } else { F::zero() };

            let increment = if addr_changes {
                // (mem_op.addr - last_addr + if i == 0 && segment_id == 0 { 1 } else { 0 }) as u64
                (mem_op.addr - last_addr) as u64
            } else {
                mem_op.step - last_step
            };
            trace[i].increment = F::from_canonical_u64(increment);

            // Store the value of incremenet so it can be range checked
            if increment <= MEMORY_MAX_DIFF || increment == 0 {
                range_check_data[(increment - 1) as usize] += 1;
            } else {
                panic!("MemSM: increment's out of range: {} i:{} addr_changes:{} mem_op.addr:0x{:X} last_addr:0x{:X} mem_op.step:{} last_step:{}",
                    increment, i, addr_changes as u8, mem_op.addr, last_addr, mem_op.step, last_step);
            }

            last_addr = mem_op.addr;
            last_step = mem_op.step;
            last_value = mem_op.value;
        }

        // STEP3. Add dummy rows to the output vector to fill the remaining rows
        // PADDING: At end of memory fill with same addr, incrementing step, same value, sel = 0, rd
        // = 1, wr = 0
        let last_row_idx = mem_ops.len() - 1;
        let addr = trace[last_row_idx].addr;
        let value = trace[last_row_idx].value;

        let padding_size = trace.num_rows - mem_ops.len();
        for i in mem_ops.len()..trace.num_rows {
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
        range_check_data[0] += padding_size as u64;

        // no add extra +1 because index = value - 1
        // RAM_W_ADDR_END - last_addr + 1 - 1 = RAM_W_ADDR_END - last_addr
        range_check_data[(RAM_W_ADDR_END - last_addr) as usize] += 1; // TODO

        // TODO: Perform the range checks
        let range_id = self.std.get_range(BigInt::from(1), BigInt::from(MEMORY_MAX_DIFF), None);
        for (value, &multiplicity) in range_check_data.iter().enumerate() {
            if (multiplicity == 0) {
                continue;
            }
            self.std.range_check(
                F::from_canonical_usize(value + 1),
                F::from_canonical_u64(multiplicity),
                range_id,
            );
        }

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
}

impl<F: PrimeField> MemModule<F> for MemSM<F> {
    fn send_inputs(&self, mem_op: &[MemInput]) {
        self.prove(mem_op);
    }
    fn get_addr_ranges(&self) -> Vec<(u32, u32)> {
        vec![(RAM_ADDR as u32, (RAM_ADDR + RAM_SIZE - 1) as u32)]
    }
    fn get_flush_input_size(&self) -> u32 {
        0
    }
}
