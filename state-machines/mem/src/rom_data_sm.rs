use std::sync::Arc;

use crate::{MemInput, MemModule, MemPreviousSegment};
use fields::PrimeField64;
use mem_common::{MEMORY_INIT_STEP, MEM_BYTES_BITS};
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
#[cfg(feature = "debug_mem")]
use std::{
    env,
    fs::File,
    io::{BufWriter, Write},
};
use zisk_common::SegmentId;
use zisk_core::{ROM_ADDR, ROM_ADDR_MAX};
use zisk_pil::{
    RomDataAirValues, RomDataTrace, RomDataTraceRow, RomDataTraceRowOps, RomDataTraceRowPacked,
};

pub const ROM_DATA_W_ADDR_INIT: u32 = ROM_ADDR as u32 >> MEM_BYTES_BITS;
pub const ROM_DATA_W_ADDR_END: u32 = ROM_ADDR_MAX as u32 >> MEM_BYTES_BITS;

const _: () = {
    assert!(ROM_ADDR_MAX <= 0xFFFF_FFFF, "ROM_DATA memory exceeds the 32-bit addressable range");
    assert!(
        (ROM_ADDR_MAX - ROM_ADDR) <= (128 << 20),
        "ROM_DATA is too large. ROM size must be <= 128MB"
    );
};

pub struct RomDataSM<F: PrimeField64> {
    /// PIL2 standard library
    std: Arc<Std<F>>,

    range_24bits_id: usize,
}

#[allow(unused, unused_variables)]
impl<F: PrimeField64> RomDataSM<F> {
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let range_24bits_id =
            std.get_range_id(0, (1 << 24) - 1, None).expect("Failed to get 24 bits range ID");
        Arc::new(Self { range_24bits_id, std: std.clone() })
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
    pub fn save_to_file<R: RomDataTraceRowOps<F>>(trace: &RomDataTrace<R>, file_name: &str) {
        let file = File::create(file_name).unwrap();
        let mut writer = BufWriter::new(file);
        let num_rows = RomDataTrace::<R>::NUM_ROWS;

        for i in 0..num_rows {
            let addr = trace[i].get_addr() * 8;
            let step = trace[i].get_step();
            let values = [trace[i].get_value(0), trace[i].get_value(1)];
            writeln!(writer, "{:#010X} {} {:?} @{}", addr, step, values, (step - 1) >> 20).unwrap();
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
    fn compute_witness(
        &self,
        mem_ops: &[MemInput],
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<AirInstance<F>> {
        if packed {
            self.compute_witness_inner::<RomDataTraceRowPacked<F>>(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
            )
        } else {
            self.compute_witness_inner::<RomDataTraceRow<F>>(
                mem_ops,
                segment_id,
                is_last_segment,
                previous_segment,
                trace_buffer,
            )
        }
    }
}

impl<F: PrimeField64> RomDataSM<F> {
    fn compute_witness_inner<R: RomDataTraceRowOps<F>>(
        &self,
        mem_ops: &[MemInput],
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = RomDataTrace::<R>::new_from_vec(trace_buffer)?;
        let num_rows = RomDataTrace::<R>::NUM_ROWS;
        assert!(
            !mem_ops.is_empty() && mem_ops.len() <= num_rows,
            "RomDataSM: mem_ops.len()={} out of range {}",
            mem_ops.len(),
            num_rows
        );

        // Fill the remaining rows
        let mut last_addr: u32 = previous_segment.addr;
        let mut i = 0;

        for mem_op in mem_ops.iter() {
            trace[i].set_addr(mem_op.addr);
            trace[i].set_step(mem_op.step);

            let (low_val, high_val) = self.get_u32_values(mem_op.value);
            trace[i].set_all_value(&[low_val, high_val]);

            let addr_change = last_addr != mem_op.addr;
            trace[i].set_addr_change(addr_change || (i == 0 && segment_id == 0));

            last_addr = mem_op.addr;
            i += 1;
            if i >= num_rows {
                break;
            }
        }
        let count = i;

        let last_row_idx = count - 1;
        if count < num_rows {
            trace[count] = trace[last_row_idx];
            trace[count].set_addr_change(false);
            trace[count].set_step(MEMORY_INIT_STEP); // make sure the step is different from the last mem_op row

            for i in count + 1..num_rows {
                trace[i] = trace[i - 1];
            }
        }

        assert!(
            is_last_segment || count == num_rows,
            "All intermediate segments must fill all rows"
        );

        let mut air_values = RomDataAirValues::<F>::new();
        air_values.padding_size = F::from_u32((num_rows - count) as u32);
        air_values.segment_id = F::from_usize(segment_id.into());
        air_values.is_first_segment = F::from_bool(segment_id == 0);
        air_values.is_last_segment = F::from_bool(is_last_segment);
        air_values.previous_segment_addr = F::from_u32(previous_segment.addr);
        air_values.segment_last_addr = F::from_u32(last_addr);

        air_values.previous_segment_value[0] = F::from_u32(previous_segment.value as u32);
        air_values.previous_segment_value[1] = F::from_u32((previous_segment.value >> 32) as u32);

        air_values.segment_last_value[0] = F::from_u32(trace[last_row_idx].get_value(0));
        air_values.segment_last_value[1] = F::from_u32(trace[last_row_idx].get_value(1));

        if is_last_segment {
            self.std.range_check_one(self.range_24bits_id, count as u64);
        }

        #[cfg(feature = "debug_mem")]
        {
            let path = env::var("MEM_TRACE_DIR").unwrap_or("tmp/mem_trace".to_string());
            let filename = format!("{path}/rom_trace_{segment_id:04}.txt");
            Self::save_to_file(&trace, &filename);
        }

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut trace).with_air_values(&mut air_values)))
    }
}
