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
    MemInput, MemModule, MEM_BYTES_BITS, MEM_INC_C_BITS, MEM_INC_C_MASK, MEM_INC_C_MAX_RANGE,
    MEM_INC_C_SIZE,
};
use fields::PrimeField64;
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
        trace_buffer: Vec<F>,
    ) -> AirInstance<F> {
        let mut trace = MemTrace::<F>::new_from_vec(trace_buffer);

        // println!(
        //     "[MemSM] segment_id:{} mem_ops:{} rows:{}  [0]{:?} previous_segment:{:?}",
        //     segment_id,
        //     mem_ops.len(),
        //     trace.num_rows,
        //     mem_ops[0],
        //     previous_segment
        // );

        let std = self.std.clone();

        let range_id = std.get_range(0, MEM_INC_C_MAX_RANGE as i64, None);
        let mut range_check_data: Vec<u32> = vec![0; MEM_INC_C_SIZE];

        // use special counter for internal reads
        let distance_base = previous_segment.addr - RAM_W_ADDR_INIT;
        let mut last_addr = previous_segment.addr;
        let mut last_step = previous_segment.step;
        let mut last_value = previous_segment.value;

        let mut i = 0;

        for mem_op in mem_ops {
            let step = mem_op.step;

            if i >= trace.num_rows {
                break;
            }

            // set the common values of trace between internal reads and regular memory operation
            trace[i].addr = F::from_u32(mem_op.addr);
            let addr_changes = last_addr != mem_op.addr;
            trace[i].addr_changes = if addr_changes { F::ONE } else { F::ZERO };

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

            if i >= trace.num_rows {
                break;
            }
            // set specific values of trace for regular memory operation
            let (low_val, high_val) = (mem_op.value as u32, (mem_op.value >> 32) as u32);
            trace[i].value = [F::from_u32(low_val), F::from_u32(high_val)];

            trace[i].step = F::from_u64(step);
            trace[i].sel = F::ONE;

            if addr_changes || mem_op.is_write {
                // in case of read operations of same address, add one to allow many reads
                // over same address and step
                trace[i].read_same_addr = F::ZERO;
                increment -= 1;
            } else {
                trace[i].read_same_addr = F::ONE;
            }
            let lsb_increment = increment & MEM_INC_C_MASK;
            let msb_increment = increment >> MEM_INC_C_BITS;
            trace[i].increment[0] = F::from_usize(lsb_increment);
            trace[i].increment[1] = F::from_usize(msb_increment);
            trace[i].wr = F::from_bool(mem_op.is_write);

            // println!("TRACE[{}] = [0x{:X},{}] {}", i, mem_op.addr * 8, mem_op.step, mem_op.value,);

            #[cfg(feature = "debug_mem")]
            if (lsb_increment >= MEM_INC_C_SIZE) || (msb_increment > MEM_INC_C_SIZE) {
                panic!("MemSM: increment's out of range: {} i:{} addr_changes:{} mem_op.addr:0x{:X} last_addr:0x{:X} mem_op.step:{} last_step:{}",
                    increment, i, addr_changes as u8, mem_op.addr, last_addr, mem_op.step, last_step);
            }

            range_check_data[lsb_increment] += 1;
            range_check_data[msb_increment] += 1;

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
        let step = trace[last_row_idx].step;

        let padding_size = trace.num_rows - count;
        let padding_increment = [F::ZERO, F::ZERO];
        for i in count..trace.num_rows {
            trace[i].addr = addr;
            trace[i].step = step;
            trace[i].sel = F::ZERO;
            trace[i].wr = F::ZERO;

            trace[i].value = value;

            trace[i].addr_changes = F::ZERO;
            trace[i].increment = padding_increment;
            trace[i].read_same_addr = F::ONE;
        }

        if padding_size > 0 {
            // Store the padding range checks
            range_check_data[0] += (2 * padding_size) as u32;
        }

        // no add extra +1 because index = value - 1
        // RAM_W_ADDR_END - last_addr + 1 - 1 = RAM_W_ADDR_END - last_addr
        let distance_end = RAM_W_ADDR_END - last_addr;

        self.std.range_checks(range_check_data, range_id);

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

        // println!("AIR_VALUES[{}]: {:?}", segment_id, air_values);

        let range_16bits_id = std.get_range(0, 0xFFFF, None);

        self.std.range_check(distance_base[0] as i64, 1, range_16bits_id);
        self.std.range_check(distance_base[1] as i64, 1, range_16bits_id);
        self.std.range_check(distance_end[0] as i64, 1, range_16bits_id);
        self.std.range_check(distance_end[1] as i64, 1, range_16bits_id);

        #[cfg(feature = "debug_mem")]
        {
            self.save_to_file(&trace, &format!("/tmp/mem_trace_{}.txt", segment_id));
            println!("[Mem:{}] mem_ops:{} padding:{}", segment_id, mem_ops.len(), padding_size);
        }
        AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values))
    }
}
