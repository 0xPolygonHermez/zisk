use std::sync::Arc;

use crate::{MemInput, MemModule, MemPreviousSegment, MEM_BYTES_BITS, SEGMENT_ADDR_MAX_RANGE};

use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace};
use zisk_common::SegmentId;
use zisk_core::{INPUT_ADDR, MAX_INPUT_SIZE};
use zisk_pil::{InputDataAirValues, InputDataTrace};

pub const INPUT_DATA_W_ADDR_INIT: u32 = INPUT_ADDR as u32 >> MEM_BYTES_BITS;
pub const INPUT_DATA_W_ADDR_END: u32 = (INPUT_ADDR + MAX_INPUT_SIZE - 1) as u32 >> MEM_BYTES_BITS;

#[allow(clippy::assertions_on_constants)]
const _: () = {
    assert!(
        INPUT_ADDR + MAX_INPUT_SIZE - 1 <= 0xFFFF_FFFF,
        "INPUT_DATA memory exceeds the 32-bit addressable range"
    );
};

pub struct InputDataSM<F: PrimeField64> {
    /// PIL2 standard library
    std: Arc<Std<F>>,
}

#[allow(unused, unused_variables)]
impl<F: PrimeField64> InputDataSM<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        Arc::new(Self { std: std.clone() })
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
}

impl<F: PrimeField64> MemModule<F> for InputDataSM<F> {
    fn get_addr_range(&self) -> (u32, u32) {
        (INPUT_DATA_W_ADDR_INIT, INPUT_DATA_W_ADDR_END)
    }

    // TODO PRE: proxy calculate if exists jmp on step out-of-range, adding internal inputs
    // memory only need to process these special inputs, but inputs no change. At end of
    // inputs proxy add an extra internal input to jump to last address

    /// Finalizes the witness accumulation process and triggers the proof generation.
    ///
    /// This method is invoked by the executor when no further witness data remains to be added.
    ///
    /// # Parameters
    ///
    /// - `mem_inputs`: A slice of all `ZiskRequiredMemory` inputs
    fn compute_witness(
        &self,
        mem_ops: &[MemInput],
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
    ) -> AirInstance<F> {
        let mut trace = InputDataTrace::<F>::new_from_vec(trace_buffer);

        debug_assert!(
            !mem_ops.is_empty() && mem_ops.len() <= trace.num_rows(),
            "InputDataSM: mem_ops.len()={} out of range {}",
            mem_ops.len(),
            trace.num_rows()
        );

        let mut range_check_data: Vec<u32> = vec![0; 1 << 16];

        // range of instance
        let range_id = self.std.get_range(0, SEGMENT_ADDR_MAX_RANGE as i64, None);
        self.std.range_check((previous_segment.addr - INPUT_DATA_W_ADDR_INIT) as i64, 1, range_id);

        let mut last_addr: u32 = previous_segment.addr;
        let mut last_step: u64 = previous_segment.step;
        let mut last_value: u64 = previous_segment.value;
        let mut i = 0;

        for mem_op in mem_ops.iter() {
            let distance = mem_op.addr - last_addr;

            if i >= trace.num_rows {
                break;
            }

            if distance > 1 {
                // check if has enough rows to complete the internal reads + regular memory
                let mut internal_reads = distance - 1;
                let incomplete = (i + internal_reads as usize) >= trace.num_rows;
                if incomplete {
                    internal_reads = (trace.num_rows - i) as u32;
                }

                trace[i].addr_changes = F::ONE;
                last_addr += 1;
                trace[i].addr = F::from_u32(last_addr);

                // the step, value of internal reads isn't relevant
                last_step = 0;
                trace[i].step = F::ZERO;
                trace[i].sel = F::ZERO;

                // setting value to zero, is not relevant for internal reads
                last_value = 0;
                for j in 0..4 {
                    trace[i].value_word[j] = F::ZERO;
                }
                i += 1;

                for _j in 1..internal_reads {
                    trace[i] = trace[i - 1];
                    last_addr += 1;
                    trace[i].addr = F::from_u32(last_addr);

                    i += 1;
                }
                range_check_data[0] += 4 * internal_reads;
                if incomplete {
                    break;
                }
            }

            trace[i].addr = F::from_u32(mem_op.addr);
            trace[i].step = F::from_u64(mem_op.step);
            trace[i].sel = F::ONE;
            trace[i].is_free_read = F::from_bool(mem_op.addr == INPUT_DATA_W_ADDR_INIT);

            let value = mem_op.value;
            let value_words = self.get_u16_values(value);
            for j in 0..4 {
                range_check_data[value_words[j] as usize] += 1;
                trace[i].value_word[j] = F::from_u16(value_words[j]);
            }

            let addr_changes = last_addr != mem_op.addr;
            trace[i].addr_changes = F::from_bool(addr_changes);

            last_addr = mem_op.addr;
            last_step = mem_op.step;
            last_value = mem_op.value;
            i += 1;
        }
        let count = i;

        // STEP3. Add dummy rows to the output vector to fill the remaining rows
        //PADDING: At end of memory fill with same addr, incrementing step, same value, sel = 0
        let last_row_idx = count - 1;
        let addr = trace[last_row_idx].addr;
        let is_free_read = F::from_bool(last_addr == INPUT_DATA_W_ADDR_INIT);
        let value = trace[last_row_idx].value_word;

        let padding_size = trace.num_rows() - count;
        for i in count..trace.num_rows() {
            last_step += 1;

            // TODO CHECK
            // trace[i].mem_segment = segment_id_field;
            // trace[i].mem_last_segment = is_last_segment_field;

            trace[i].addr = addr;
            trace[i].step = F::from_u64(last_step);
            trace[i].sel = F::ZERO;
            trace[i].value_word = value;
            trace[i].is_free_read = is_free_read;

            trace[i].addr_changes = F::ZERO;
        }

        self.std.range_check((INPUT_DATA_W_ADDR_END - last_addr) as i64, 1, range_id);

        // range of chunks
        let range_id = self.std.get_range(0, (1 << 16) - 1, None);
        for value_chunk in &value {
            let value = value_chunk.as_canonical_u64();
            range_check_data[value as usize] += padding_size as u32;
        }
        self.std.range_checks(range_check_data, range_id);

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

        AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values))
    }
}
