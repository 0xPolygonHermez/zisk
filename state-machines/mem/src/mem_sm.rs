use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

// const MEM_INITIAL_ADDRESS: u32 = 0xA0000000;
// const MEM_FINAL_ADDRESS: u32 = MEM_INITIAL_ADDRESS + 128 * 1024 * 1024;
use crate::{MemInput, MemModule};
use p3_field::PrimeField;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::AirInstance;
use rayon::prelude::*;

use sm_common::create_prover_buffer;
use zisk_pil::{MemTrace, MEM_AIR_IDS, ZISK_AIRGROUP_ID};

pub struct MemSM<F: PrimeField> {
    // Witness computation manager
    wcm: Arc<WitnessManager<F>>,

    num_rows: usize,
    // Count of registered predecessors
    registered_predecessors: AtomicU32,
}

#[allow(unused, unused_variables)]
impl<F: PrimeField> MemSM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let pctx = wcm.get_pctx();
        let air = pctx.pilout.get_air(ZISK_AIRGROUP_ID, MEM_AIR_IDS[0]);
        let mem_sm = Self {
            wcm: wcm.clone(),
            num_rows: air.num_rows(),
            registered_predecessors: AtomicU32::new(0),
        };
        let mem_sm = Arc::new(mem_sm);

        wcm.register_component(mem_sm.clone(), Some(ZISK_AIRGROUP_ID), Some(MEM_AIR_IDS));

        mem_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {}
    }

    pub fn prove(&self, mem_accesses: &[MemInput]) {
        // Sort the (full) aligned memory accesses

        let pctx = self.wcm.get_pctx();
        let ectx = self.wcm.get_ectx();
        let sctx = self.wcm.get_sctx();

        let air = pctx.pilout.get_air(ZISK_AIRGROUP_ID, MEM_AIR_IDS[0]);

        let num_chunks = (mem_accesses.len() as f64 / (air.num_rows() - 1) as f64).ceil() as usize;

        let mut prover_buffers = Mutex::new(vec![Vec::new(); num_chunks]);
        let mut offsets = vec![0; num_chunks];
        let mut global_idxs = vec![0; num_chunks];

        for i in 0..num_chunks {
            if let (true, global_idx) = self.wcm.get_ectx().dctx.write().unwrap().add_instance(
                ZISK_AIRGROUP_ID,
                MEM_AIR_IDS[0],
                1,
            ) {
                let (buffer, offset) =
                    create_prover_buffer::<F>(&ectx, &sctx, ZISK_AIRGROUP_ID, MEM_AIR_IDS[0]);

                prover_buffers.lock().unwrap()[i] = buffer;
                offsets[i] = offset;
                global_idxs[i] = global_idx;
            }
        }
        mem_accesses.par_chunks(air.num_rows() - 1).enumerate().for_each(
            |(segment_id, mem_ops)| {
                let mem_first_row = if segment_id == 0 {
                    mem_accesses.last().unwrap().clone()
                } else {
                    mem_accesses[segment_id * ((air.num_rows() - 1) - 1)].clone()
                };

                let prover_buffer = std::mem::take(&mut prover_buffers.lock().unwrap()[segment_id]);

                self.prove_instance(
                    mem_ops,
                    mem_first_row,
                    segment_id,
                    segment_id == mem_accesses.len() - 1,
                    prover_buffer,
                    offsets[segment_id],
                    global_idxs[segment_id],
                );
            },
        );
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
        mem_first_row: MemInput,
        segment_id: usize,
        is_last_segment: bool,
        mut prover_buffer: Vec<F>,
        offset: u64,
        global_idx: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pctx = self.wcm.get_pctx();
        let sctx = self.wcm.get_sctx();

        // STEP2: Process the memory inputs and convert them to AIR instances
        let air = pctx.pilout.get_air(ZISK_AIRGROUP_ID, MEM_AIR_IDS[0]);

        let max_rows_per_segment = air.num_rows() - 1;

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
            MemTrace::<F>::map_buffer(&mut prover_buffer, air.num_rows(), offset as usize).unwrap();

        // STEP1. Add the first row to the output vector as equal to the last row of the previous
        // segment CASE: last row of segment is read
        //
        // S[n-1]    wr = 0, sel = 1, addr, step, value
        // S+1[0]    wr = 0, sel = 0, addr, step, value
        //
        // CASE: last row of segment is write
        //
        // S[n-1]    wr = 1, sel = 1, addr, step, value
        // S+1[0]    wr = 0, sel = 0, addr, step, value

        // TODO CHECK
        // trace[0].mem_segment = segment_id_field;
        // trace[0].mem_last_segment = is_last_segment_field;

        trace[0].addr = F::from_canonical_u32(mem_first_row.address);
        trace[0].step = F::from_canonical_u64(mem_first_row.step);
        trace[0].sel = F::zero();
        trace[0].wr = F::zero();

        let value = mem_first_row.value;
        let (low_val, high_val) = self.get_u32_values(value);
        trace[0].value = [F::from_canonical_u32(low_val), F::from_canonical_u32(high_val)];
        trace[0].addr_changes = F::zero();

        trace[0].same_value = F::zero();
        trace[0].first_addr_access_is_read = F::zero();

        // STEP2. Add all the memory operations to the buffer
        for (idx, mem_op) in mem_ops.iter().enumerate() {
            let i = idx + 1;
            // TODO CHECK
            // trace[i].mem_segment = segment_id_field;
            // trace[i].mem_last_segment = is_last_segment_field;

            let mem_addr = mem_op.address >> 3;
            trace[i].addr = F::from_canonical_u32(mem_addr); // n-byte address, real address = addr * MEM_BYTES
            trace[i].step = F::from_canonical_u64(mem_op.step);
            trace[i].sel = F::one();
            trace[i].wr = F::from_bool(mem_op.is_write);

            let (low_val, high_val) = self.get_u32_values(mem_op.value);
            trace[i].value = [F::from_canonical_u32(low_val), F::from_canonical_u32(high_val)];

            let addr_changes = trace[i - 1].addr != trace[i].addr;
            trace[i].addr_changes = if addr_changes { F::one() } else { F::zero() };

            let same_value = trace[i - 1].value[0] == trace[i].value[0] &&
                trace[i - 1].value[1] == trace[i].value[1];
            trace[i].same_value = if same_value { F::one() } else { F::zero() };

            let first_addr_access_is_read = addr_changes && !mem_op.is_write;
            trace[i].first_addr_access_is_read =
                if first_addr_access_is_read { F::one() } else { F::zero() };
            assert!(trace[i].sel.is_zero() || trace[i].sel.is_one());
        }

        // STEP3. Add dummy rows to the output vector to fill the remaining rows
        //PADDING: At end of memory fill with same addr, incrementing step, same value, sel = 0, rd
        // = 1, wr = 0
        let last_row_idx = mem_ops.len();
        let addr = trace[last_row_idx].addr;
        let mut step = trace[last_row_idx].step;
        let value = trace[last_row_idx].value;

        for i in (mem_ops.len() + 1)..air.num_rows() {
            step += F::one();

            // TODO CHECK
            // trace[i].mem_segment = segment_id_field;
            // trace[i].mem_last_segment = is_last_segment_field;

            trace[i].addr = addr;
            trace[i].step = step;
            trace[i].sel = F::zero();
            trace[i].wr = F::zero();

            trace[i].value = value;

            trace[i].addr_changes = F::zero();
            trace[i].same_value = F::one();
            trace[i].first_addr_access_is_read = F::zero();
        }

        let mut air_instance = AirInstance::new(
            self.wcm.get_sctx(),
            ZISK_AIRGROUP_ID,
            MEM_AIR_IDS[0],
            Some(segment_id),
            prover_buffer,
        );

        air_instance.set_airvalue(
            &sctx,
            "Mem.mem_segment",
            F::from_canonical_u64(segment_id as u64),
        );
        air_instance.set_airvalue(&sctx, "Mem.mem_last_segment", F::from_bool(is_last_segment));

        pctx.air_instance_repo.add_air_instance(air_instance, Some(global_idx));

        Ok(())
    }

    fn get_u32_values(&self, value: u64) -> (u32, u32) {
        (value as u32, (value >> 32) as u32)
    }
}

impl<F: PrimeField> MemModule<F> for MemSM<F> {
    fn send_inputs(&self, mem_op: &[MemInput]) {
        self.prove(&mem_op);
    }
    fn get_addr_ranges(&self) -> Vec<(u32, u32)> {
        vec![(0x80000000, 0x80000000 + (1 << 24) - 1), (0xA0000000, 0xA0000000 + (1 << 24) - 1)]
    }
    fn get_flush_input_size(&self) -> u32 {
        self.num_rows as u32
    }
}

impl<F: PrimeField> WitnessComponent<F> for MemSM<F> {}
