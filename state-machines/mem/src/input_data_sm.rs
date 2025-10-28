use std::sync::Arc;

use crate::{MemInput, MemModule, MemPreviousSegment};
use mem_common::{MEM_BYTES_BITS, SEGMENT_ADDR_MAX_RANGE};

use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace};
use zisk_common::SegmentId;
use zisk_core::{INPUT_ADDR, MAX_INPUT_SIZE};
use zisk_pil::InputDataAirValues;
#[cfg(not(feature = "packed"))]
use zisk_pil::InputDataTrace;
#[cfg(feature = "packed")]
use zisk_pil::InputDataTracePacked;

#[cfg(feature = "packed")]
type InputDataTraceType<F> = InputDataTracePacked<F>;

#[cfg(not(feature = "packed"))]
type InputDataTraceType<F> = InputDataTrace<F>;

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
    fn is_dual(&self) -> bool {
        false
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
        let mut trace = InputDataTraceType::<F>::new_from_vec(trace_buffer);

        let num_rows = InputDataTraceType::<F>::NUM_ROWS;
        debug_assert!(
            !mem_ops.is_empty() && mem_ops.len() <= num_rows,
            "InputDataSM: mem_ops.len()={} out of range {}",
            mem_ops.len(),
            num_rows
        );

        let mut range_check_data: Vec<u32> = vec![0; 1 << 16];

        // range of instance
        let range_id = self.std.get_range_id(0, SEGMENT_ADDR_MAX_RANGE as i64, None);
        self.std.range_check(range_id, (previous_segment.addr - INPUT_DATA_W_ADDR_INIT) as i64, 1);

        let mut last_addr: u32 = previous_segment.addr;
        let mut last_step: u64 = previous_segment.step;
        let mut last_value: u64 = previous_segment.value;
        let mut i = 0;

        for mem_op in mem_ops.iter() {
            let distance = mem_op.addr - last_addr;

            if i >= num_rows {
                break;
            }

            if distance > 1 {
                // check if has enough rows to complete the internal reads + regular memory
                let mut internal_reads = distance - 1;
                let incomplete = (i + internal_reads as usize) >= num_rows;
                if incomplete {
                    internal_reads = (num_rows - i) as u32;
                }

                trace[i].set_addr_changes(true);
                last_addr += 1;
                trace[i].set_addr(last_addr);

                // the step, value of internal reads isn't relevant
                last_step = 0;
                trace[i].set_step(0);
                trace[i].set_sel(false);

                // setting value to zero, is not relevant for internal reads
                last_value = 0;
                for j in 0..4 {
                    trace[i].set_value_word(j, 0);
                }
                i += 1;

                for _j in 1..internal_reads {
                    trace[i] = trace[i - 1];
                    last_addr += 1;
                    trace[i].set_addr(last_addr);

                    i += 1;
                }
                range_check_data[0] += 4 * internal_reads;
                if incomplete {
                    break;
                }
            }

            trace[i].set_addr(mem_op.addr);
            trace[i].set_step(mem_op.step);
            trace[i].set_sel(true);
            trace[i].set_is_free_read(mem_op.addr == INPUT_DATA_W_ADDR_INIT);

            let value = mem_op.value;
            let value_words = self.get_u16_values(value);
            for j in 0..4 {
                range_check_data[value_words[j] as usize] += 1;
                trace[i].set_value_word(j, value_words[j]);
            }

            let addr_changes = last_addr != mem_op.addr;
            trace[i].set_addr_changes(addr_changes);

            last_addr = mem_op.addr;
            last_step = mem_op.step;
            last_value = mem_op.value;
            i += 1;
        }
        let count = i;

        // STEP3. Add dummy rows to the output vector to fill the remaining rows
        //PADDING: At end of memory fill with same addr, incrementing step, same value, sel = 0
        let last_row_idx = count - 1;
        let addr = trace[last_row_idx].get_addr();
        let is_free_read = last_addr == INPUT_DATA_W_ADDR_INIT;

        let padding_size = num_rows - count;
        for i in count..num_rows {
            last_step += 1;

            // TODO CHECK
            // trace[i].mem_segment = segment_id_field;
            // trace[i].mem_last_segment = is_last_segment_field;

            trace[i].set_addr(addr);
            trace[i].set_step(last_step);
            trace[i].set_sel(false);
            for j in 0..4 {
                let value = trace[last_row_idx].get_value_word(j);
                trace[i].set_value_word(j, value);
            }
            trace[i].set_is_free_read(is_free_read);

            trace[i].set_addr_changes(false);
        }

        self.std.range_check(range_id, (INPUT_DATA_W_ADDR_END - last_addr) as i64, 1);

        // range of chunks
        let range_id = self.std.get_range_id(0, (1 << 16) - 1, None);
        for j in 0..4 {
            let value = trace[last_row_idx].get_value_word(j);
            range_check_data[value as usize] += padding_size as u32;
        }
        self.std.range_checks(range_id, range_check_data);

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
