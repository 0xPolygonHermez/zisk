use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use crate::{MemInput, MemModule};
use num_bigint::BigInt;
use num_traits::cast::ToPrimitive;
use p3_field::PrimeField;
use pil_std_lib::Std;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::AirInstance;
use rayon::prelude::*;

use sm_common::create_prover_buffer;
use zisk_pil::{RomDataTrace, ROM_DATA_AIR_IDS, ZISK_AIRGROUP_ID};

const MEM_INITIAL_ADDRESS: u32 = 0x80000000;
const MEM_FINAL_ADDRESS: u32 = MEM_INITIAL_ADDRESS + 128 * 1024 * 1024;
const MEMORY_MAX_DIFF: u32 = 0x1000000;

pub struct RomDataSM<F: PrimeField> {
    // Witness computation manager
    wcm: Arc<WitnessManager<F>>,

    // STD
    std: Arc<Std<F>>,

    num_rows: usize,
    // Count of registered predecessors
    registered_predecessors: AtomicU32,
}

#[allow(unused, unused_variables)]
impl<F: PrimeField> RomDataSM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>, std: Arc<Std<F>>) -> Arc<Self> {
        let pctx = wcm.get_pctx();
        let air = pctx.pilout.get_air(ZISK_AIRGROUP_ID, ROM_DATA_AIR_IDS[0]);
        let rom_data_sm = Self {
            wcm: wcm.clone(),
            std,
            num_rows: air.num_rows(),
            registered_predecessors: AtomicU32::new(0),
        };
        let rom_data_sm = Arc::new(rom_data_sm);

        wcm.register_component(rom_data_sm.clone(), Some(ZISK_AIRGROUP_ID), Some(ROM_DATA_AIR_IDS));

        rom_data_sm.register_predecessor();

        rom_data_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            let pctx = self.wcm.get_pctx();
            self.std.unregister_predecessor(pctx, None);
        }
    }

    pub fn prove(&self, inputs: &[MemInput]) {
        let wcm = self.wcm.clone();
        let pctx = wcm.get_pctx();
        let ectx = wcm.get_ectx();
        let sctx = wcm.get_sctx();

        println!("ROM-DATA: {} inputs", inputs.len());
        let air_mem = pctx.pilout.get_air(ZISK_AIRGROUP_ID, ROM_DATA_AIR_IDS[0]);
        let air_mem_rows = air_mem.num_rows();

        let inputs_len = inputs.len();
        let num_chunks = (inputs_len as f64 / air_mem_rows as f64).ceil() as usize;

        let mut prover_buffers = Mutex::new(vec![Vec::new(); num_chunks]);
        let mut offsets = vec![0; num_chunks];
        let mut global_idxs = vec![0; num_chunks];

        for i in 0..num_chunks {
            if let (true, global_idx) =
                ectx.dctx.write().unwrap().add_instance(ZISK_AIRGROUP_ID, ROM_DATA_AIR_IDS[0], 1)
            {
                let (buffer, offset) =
                    create_prover_buffer::<F>(&ectx, &sctx, ZISK_AIRGROUP_ID, ROM_DATA_AIR_IDS[0]);

                prover_buffers.lock().unwrap()[i] = buffer;
                offsets[i] = offset;
                global_idxs[i] = global_idx;
            }
        }

        for (segment_id, mem_ops) in inputs.chunks(air_mem_rows).enumerate() {
            let is_last_segment = segment_id == num_chunks - 1;

            let prover_buffer = std::mem::take(&mut prover_buffers.lock().unwrap()[segment_id]);

            self.prove_instance(
                mem_ops,
                segment_id,
                is_last_segment,
                prover_buffer,
                offsets[segment_id],
                air_mem_rows,
                global_idxs[segment_id],
            );
        }

        // TODO: Uncomment when sequential works
        // inputs.par_chunks(air_mem_rows - 1).enumerate().for_each(|(segment_id, mem_ops)| {
        //     let mem_first_row = if segment_id == 0 {
        //         inputs.last().unwrap().clone()
        //     } else {
        //         inputs[segment_id * ((air_mem_rows - 1) - 1)].clone()
        //     };

        //     let prover_buffer = std::mem::take(&mut prover_buffers.lock().unwrap()[segment_id]);

        //     self.prove_instance(
        //         mem_ops,
        //         mem_first_row,
        //         segment_id,
        //         segment_id == inputs.len() - 1,
        //         prover_buffer,
        //         offsets[segment_id],
        //         global_idxs[segment_id],
        //     );
        // });
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
        mut prover_buffer: Vec<F>,
        offset: u64,
        air_mem_rows: usize,
        global_idx: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let max_rows_per_segment = air_mem_rows - 1;

        assert!(!mem_ops.is_empty() && mem_ops.len() <= max_rows_per_segment);

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

        let mut trace =
            RomDataTrace::<F>::map_buffer(&mut prover_buffer, air_mem_rows, offset as usize)
                .unwrap();

        let mut range_check_data: Vec<u64> = vec![0; MEMORY_MAX_DIFF as usize];

        // Fill the first row
        const MEM_INITIAL_64_ADDRESS: u32 = MEM_INITIAL_ADDRESS >> 3;

        // Fill the first row
        let first_mem_op = mem_ops.first().unwrap();

        debug_assert!(first_mem_op.address >= MEM_INITIAL_ADDRESS);
        let addr = first_mem_op.address >> 3;

        trace[0].addr = F::from_canonical_u32(addr);
        trace[0].step = F::from_canonical_u64(first_mem_op.step);
        trace[0].sel = F::zero();

        let value = first_mem_op.value;
        let (low_val, high_val) = self.get_u32_values(value);
        trace[0].value = [F::from_canonical_u32(low_val), F::from_canonical_u32(high_val)];
        trace[0].addr_changes = F::zero();

        // Store the value of incremenet so it can be range checked
        println!("addr: {:#X}, initial: {:#X}", addr, MEM_INITIAL_64_ADDRESS);
        let increment = addr - MEM_INITIAL_64_ADDRESS + 1;
        trace[0].increment = F::from_canonical_u32(increment);

        // Store the value of incremenet so it can be range checked
        println!(
            "addr: {:#X}, initial: {:#X}, increment: {:#X}",
            addr, MEM_INITIAL_64_ADDRESS, increment
        );
        range_check_data[increment as usize] += 1; // TODO

        // Fill the remaining rows
        for (idx, mem_op) in mem_ops.iter().enumerate() {
            let i = idx + 1;

            let mem_addr = mem_op.address >> 3;
            trace[i].addr = F::from_canonical_u32(mem_addr); // n-byte address, real address = addr * MEM_BYTES
            trace[i].step = F::from_canonical_u64(mem_op.step);
            trace[i].sel = F::one();

            let (low_val, high_val) = self.get_u32_values(mem_op.value);
            trace[i].value = [F::from_canonical_u32(low_val), F::from_canonical_u32(high_val)];

            let addr_changes = trace[i - 1].addr != trace[i].addr;
            trace[i].addr_changes = if addr_changes { F::one() } else { F::zero() };

            let same_value = trace[i - 1].value[0] == trace[i].value[0] &&
                trace[i - 1].value[1] == trace[i].value[1];

            let increment = if addr_changes {
                trace[i].addr - trace[i - 1].addr
            } else {
                trace[i].step - trace[i - 1].step
            };
            trace[i].increment = increment;

            // Store the value of incremenet so it can be range checked
            let element =
                increment.as_canonical_biguint().to_usize().expect("Cannot convert to usize");
            // range_check_data[element] += 1; // TODO:
        }

        // STEP3. Add dummy rows to the output vector to fill the remaining rows
        // PADDING: At end of memory fill with same addr, incrementing step, same value, sel = 0, rd
        // = 1, wr = 0
        let last_row_idx = mem_ops.len();
        let addr = trace[last_row_idx].addr;
        let mut step = trace[last_row_idx].step;
        let value = trace[last_row_idx].value;

        let padding_size = air_mem_rows - (mem_ops.len() + 1);

        for i in (mem_ops.len() + 1)..air_mem_rows {
            step += F::one();

            // TODO CHECK
            // trace[i].mem_segment = segment_id_field;
            // trace[i].mem_last_segment = is_last_segment_field;

            trace[i].addr = addr;
            trace[i].step = step;
            trace[i].sel = F::zero();

            trace[i].value = value;

            trace[i].addr_changes = F::zero();

            // Set increment to the minimum value so the range check passes
            trace[i].increment = F::one();
        }

        // Store the value of trivial increment so that they can be range checked
        range_check_data[1] += padding_size as u64;

        // TODO: Perform the range checks
        // let std = self.std.clone();
        // let range_id = std.get_range(BigInt::from(1), BigInt::from(MEMORY_MAX_DIFF), None);
        // for (value, &multiplicity) in range_check_data.iter().enumerate() {
        //     std.range_check(
        //         F::from_canonical_usize(value),
        //         F::from_canonical_u64(multiplicity),
        //         range_id,
        //     );
        // }

        let wcm = self.wcm.clone();
        let pctx = wcm.get_pctx();
        let sctx = wcm.get_sctx();

        let mut air_instance = AirInstance::new(
            sctx.clone(),
            ZISK_AIRGROUP_ID,
            ROM_DATA_AIR_IDS[0],
            Some(segment_id),
            prover_buffer,
        );

        air_instance.set_airvalue(
            &sctx,
            "RomData.mem_segment",
            F::from_canonical_u64(segment_id as u64),
        );
        air_instance.set_airvalue(&sctx, "RomData.mem_last_segment", F::from_bool(is_last_segment));

        pctx.air_instance_repo.add_air_instance(air_instance, Some(global_idx));

        Ok(())
    }

    fn get_u32_values(&self, value: u64) -> (u32, u32) {
        (value as u32, (value >> 32) as u32)
    }
}

impl<F: PrimeField> MemModule<F> for RomDataSM<F> {
    fn send_inputs(&self, mem_op: &[MemInput]) {
        self.prove(&mem_op);
    }
    fn get_addr_ranges(&self) -> Vec<(u32, u32)> {
        vec![(MEM_INITIAL_ADDRESS, MEM_FINAL_ADDRESS)]
    }
    fn get_flush_input_size(&self) -> u32 {
        (self.num_rows - 1) as u32
    }
}

impl<F: PrimeField> WitnessComponent<F> for RomDataSM<F> {}
