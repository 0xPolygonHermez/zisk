use std::sync::Arc;

use crate::{MemInput, MemModule, MemPreviousSegment};
use fields::PrimeField64;
use mem_common::{MEM_BYTES_BITS, SEGMENT_ADDR_MAX_RANGE};
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace};
#[cfg(feature = "debug_mem")]
use std::{
    env,
    fs::File,
    io::{BufWriter, Write},
};
use zisk_common::SegmentId;
use zisk_core::{ROM_ADDR, ROM_ADDR_MAX};
use zisk_pil::RomDataAirValues;
#[cfg(not(feature = "packed"))]
use zisk_pil::RomDataTrace;
#[cfg(feature = "packed")]
use zisk_pil::RomDataTracePacked;

#[cfg(feature = "packed")]
type RomDataTraceType<F> = RomDataTracePacked<F>;

#[cfg(not(feature = "packed"))]
type RomDataTraceType<F> = RomDataTrace<F>;

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
    #[cfg(feature = "debug_mem")]
    pub fn save_to_file(trace: &RomDataTrace<F>, file_name: &str) {
        let file = File::create(file_name).unwrap();
        let mut writer = BufWriter::new(file);
        let num_rows = RomDataTrace::NUM_ROWS;

        for i in 0..num_rows {
            let addr = trace[i].get_addr() * 8;
            let step = trace[i].get_step();
            let sel = trace[i].get_sel();
            // TODO: chunk_size * 4 = 20
            writeln!(
                writer,
                "{:#010X} {} {:?} S:{sel} @{}",
                addr,
                step,
                trace[i].value,
                (step - 1) >> 20
            )
            .unwrap();
        }
    }
}

impl<F: PrimeField64> MemModule<F> for RomDataSM<F> {
    fn get_addr_range(&self) -> (u32, u32) {
        (ROM_DATA_W_ADDR_INIT, ROM_DATA_W_ADDR_END)
    }
    fn is_dual(&self) -> bool {
        false
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
        let mut trace = RomDataTraceType::<F>::new_from_vec(trace_buffer);
        let num_rows = RomDataTraceType::<F>::NUM_ROWS;
        assert!(
            !mem_ops.is_empty() && mem_ops.len() <= num_rows,
            "RomDataSM: mem_ops.len()={} out of range {}",
            mem_ops.len(),
            num_rows
        );

        // range of instance
        let range_id = self.std.get_range_id(0, SEGMENT_ADDR_MAX_RANGE as i64, None);
        self.std.range_check(range_id, (previous_segment.addr - ROM_DATA_W_ADDR_INIT) as i64, 1);

        // Fill the remaining rows
        let mut last_addr: u32 = previous_segment.addr;
        let mut last_step: u64 = previous_segment.step;
        let mut last_value: u64 = previous_segment.value;

        if segment_id == 0 {
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

                #[cfg(feature = "debug_mem")]
                println!(
                    "INTERNAL_READS[{},{}] {} 0x{:X},{} LAST:0x{:X}",
                    segment_id,
                    i,
                    internal_reads,
                    mem_op.addr * 8,
                    mem_op.step,
                    last_addr * 8
                );

                // check if has enough rows to complete the internal reads + regular memory
                let incomplete = (i + internal_reads as usize) >= num_rows;
                if incomplete {
                    internal_reads = (num_rows - i) as u32;
                }

                trace[i].set_addr_changes(true);
                last_addr += 1;
                trace[i].set_addr(last_addr);
                trace[i].set_value(0, 0);
                trace[i].set_value(1, 0);
                trace[i].set_sel(false);
                // the step, value of internal reads isn't relevant
                trace[i].set_step(0);
                i += 1;

                for _j in 1..internal_reads {
                    trace[i] = trace[i - 1];
                    last_addr += 1;
                    trace[i].set_addr(last_addr);
                    i += 1;
                }
                if incomplete {
                    break;
                }
            }
            trace[i].set_addr(mem_op.addr);
            trace[i].set_step(mem_op.step);
            trace[i].set_sel(true);

            let (low_val, high_val) = self.get_u32_values(mem_op.value);
            trace[i].set_value(0, low_val);
            trace[i].set_value(1, high_val);

            let addr_changes = last_addr != mem_op.addr;
            trace[i].set_addr_changes(addr_changes || (i == 0 && segment_id == 0));

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
            trace[count].set_addr_changes(false);
            trace[count].set_sel(false);

            for i in count + 1..num_rows {
                trace[i] = trace[i - 1];
            }
        }

        self.std.range_check(range_id, (ROM_DATA_W_ADDR_END - last_addr) as i64, 1);

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

        #[cfg(feature = "debug_mem")]
        {
            let path = env::var("MEM_TRACE_DIR").unwrap_or("tmp/mem_trace".to_string());
            let filename = format!("{path}/rom_trace_{segment_id:04}.txt");
            Self::save_to_file(&trace, &filename);
        }

        AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values))
    }
}
