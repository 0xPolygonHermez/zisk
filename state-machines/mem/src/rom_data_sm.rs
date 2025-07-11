use std::sync::Arc;

use crate::{MemInput, MemModule, MemPreviousSegment, MEM_BYTES_BITS, SEGMENT_ADDR_MAX_RANGE};
use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace};
use zisk_common::SegmentId;
use zisk_core::{ROM_ADDR, ROM_ADDR_MAX};
use zisk_pil::{RomDataAirValues, RomDataTrace};

pub const ROM_DATA_W_ADDR_INIT: u32 = ROM_ADDR as u32 >> MEM_BYTES_BITS;
pub const ROM_DATA_W_ADDR_END: u32 = ROM_ADDR_MAX as u32 >> MEM_BYTES_BITS;

const _: () = {
    assert!(ROM_ADDR_MAX <= 0xFFFF_FFFF, "ROM_DATA memory exceeds the 32-bit addressable range");
};

pub struct RomDataSM<F: PrimeField64> {
    /// PIL2 standard library
    std: Arc<Std<F>>,
}

#[allow(unused, unused_variables)]
impl<F: PrimeField64> RomDataSM<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self { std: std.clone() })
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
}

impl<F: PrimeField64> MemModule<F> for RomDataSM<F> {
    fn get_addr_range(&self) -> (u32, u32) {
        (ROM_DATA_W_ADDR_INIT, ROM_DATA_W_ADDR_END)
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
        let mut trace = RomDataTrace::<F>::new_from_vec(trace_buffer);
        let num_rows = RomDataTrace::<F>::NUM_ROWS;
        assert!(
            !mem_ops.is_empty() && mem_ops.len() <= num_rows,
            "RomDataSM: mem_ops.len()={} out of range {}",
            mem_ops.len(),
            num_rows
        );

        // range of instance
        let range_id = self.std.get_range(0, SEGMENT_ADDR_MAX_RANGE as i64, None);
        self.std.range_check((previous_segment.addr - ROM_DATA_W_ADDR_INIT) as i64, 1, range_id);

        // Fill the remaining rows
        let mut last_addr: u32 = previous_segment.addr;
        let mut last_step: u64 = previous_segment.step;
        let mut last_value: u64 = previous_segment.value;

        if segment_id == 0 && !mem_ops.is_empty() && mem_ops[0].addr > ROM_DATA_W_ADDR_INIT {
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
            if distance > 1 {
                let mut internal_reads = distance - 1;

                // println!(
                //     "INTERNAL_READS[{},{}] {} 0x{:X},{} LAST:0x{:X}",
                //     segment_id,
                //     i,
                //     internal_reads,
                //     mem_op.addr * 8,
                //     mem_op.step,
                //     last_addr * 8
                // );

                // check if has enough rows to complete the internal reads + regular memory
                let incomplete = (i + internal_reads as usize) >= num_rows;
                if incomplete {
                    internal_reads = (num_rows - i) as u32;
                }

                trace[i].addr_changes = F::ONE;
                last_addr += 1;
                trace[i].addr = F::from_u32(last_addr);
                trace[i].value = [F::ZERO, F::ZERO];
                trace[i].sel = F::ZERO;
                // the step, value of internal reads isn't relevant
                trace[i].step = F::ZERO;
                i += 1;

                for _j in 1..internal_reads {
                    trace[i] = trace[i - 1];
                    last_addr += 1;
                    trace[i].addr = F::from_u32(last_addr);
                    i += 1;
                }
                if incomplete {
                    break;
                }
            }
            trace[i].addr = F::from_u32(mem_op.addr);
            trace[i].step = F::from_u64(mem_op.step);
            trace[i].sel = F::ONE;

            let (low_val, high_val) = self.get_u32_values(mem_op.value);
            trace[i].value = [F::from_u32(low_val), F::from_u32(high_val)];

            let addr_changes = last_addr != mem_op.addr;
            trace[i].addr_changes =
                if addr_changes || (i == 0 && segment_id == 0) { F::ONE } else { F::ZERO };

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
            trace[count].addr_changes = F::ZERO;
            trace[count].sel = F::ZERO;

            for i in count + 1..num_rows {
                trace[i] = trace[i - 1];
            }
        }

        self.std.range_check((ROM_DATA_W_ADDR_END - last_addr) as i64, 1, range_id);

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

        AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values))
    }
}
