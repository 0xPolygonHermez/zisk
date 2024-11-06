use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use p3_field::PrimeField;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::AirInstance;
use rayon::prelude::*;

use sm_common::create_prover_buffer;
use zisk_core::ZiskRequiredMemory;
use zisk_pil::{InputDataTrace, INPUT_DATA_AIR_IDS, ZISK_AIRGROUP_ID};

pub struct InputDataSM<F: PrimeField> {
    // Witness computation manager
    wcm: Arc<WitnessManager<F>>,

    // Count of registered predecessors
    registered_predecessors: AtomicU32,
}

#[allow(unused, unused_variables)]
impl<F: PrimeField> InputDataSM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let input_data_sm = Self { wcm: wcm.clone(), registered_predecessors: AtomicU32::new(0) };
        let input_data_sm = Arc::new(input_data_sm);

        wcm.register_component(
            input_data_sm.clone(),
            Some(ZISK_AIRGROUP_ID),
            Some(INPUT_DATA_AIR_IDS),
        );

        input_data_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {}
    }

    pub fn prove(&self, mem_accesses: &mut [ZiskRequiredMemory]) {
        // Sort the (full) aligned memory accesses

        let pctx = self.wcm.get_pctx();
        let ectx = self.wcm.get_ectx();
        let sctx = self.wcm.get_sctx();

        let air = pctx.pilout.get_air(ZISK_AIRGROUP_ID, INPUT_DATA_AIR_IDS[0]);

        let num_chunks = (mem_accesses.len() as f64 / (air.num_rows() - 1) as f64).ceil() as usize;

        let mut prover_buffers = Mutex::new(vec![Vec::new(); num_chunks]);
        let mut offsets = vec![0; num_chunks];
        let mut global_idxs = vec![0; num_chunks];

        for i in 0..num_chunks {
            if let (true, global_idx) = self.wcm.get_ectx().dctx.write().unwrap().add_instance(
                ZISK_AIRGROUP_ID,
                INPUT_DATA_AIR_IDS[0],
                1,
            ) {
                let (buffer, offset) = create_prover_buffer::<F>(
                    &ectx,
                    &sctx,
                    ZISK_AIRGROUP_ID,
                    INPUT_DATA_AIR_IDS[0],
                );
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
    /// - `mem_inputs`: A slice of all `ZiskRequiredMemory` inputs
    pub fn prove_instance(
        &self,
        mem_ops: &[ZiskRequiredMemory],
        mem_first_row: ZiskRequiredMemory,
        segment_id: usize,
        is_last_segment: bool,
        mut prover_buffer: Vec<F>,
        offset: u64,
        global_idx: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pctx = self.wcm.get_pctx();
        let sctx = self.wcm.get_sctx();

        // STEP2: Process the memory inputs and convert them to AIR instances
        let air = pctx.pilout.get_air(ZISK_AIRGROUP_ID, INPUT_DATA_AIR_IDS[0]);

        let max_rows_per_segment = air.num_rows() - 1;

        assert!(mem_ops.len() > 0 && mem_ops.len() <= max_rows_per_segment);

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

        //println! {"InputDataSM::prove_instance() mem_ops.len={} prover_buffer.len={} air.num_rows={}", mem_ops.len(), prover_buffer.len(), air.num_rows()};
        let mut trace =
            InputDataTrace::<F>::map_buffer(&mut prover_buffer, air.num_rows(), offset as usize)
                .unwrap();

        //let segment_id_field = F::from_canonical_u64(segment_id as u64);
        //let is_last_segment_field = F::from_bool(is_last_segment);

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

        trace[0].addr = F::from_canonical_u64(mem_first_row.address);
        trace[0].step = F::from_canonical_u64(mem_first_row.step);
        trace[0].sel = F::zero();

        let value = match mem_first_row.width {
            1 => mem_first_row.value as u8 as u64,
            2 => mem_first_row.value as u16 as u64,
            4 => mem_first_row.value as u32 as u64,
            8 => mem_first_row.value,
            _ => panic!("Invalid width"),
        };
        let (val0, val1, val2, val3) = self.get_u16_values(value);
        trace[0].value = [
            F::from_canonical_u16(val0),
            F::from_canonical_u16(val1),
            F::from_canonical_u16(val2),
            F::from_canonical_u16(val3),
        ];
        trace[0].addr_changes = F::zero();

        // STEP2. Add all the memory operations to the buffer
        for (idx, mem_op) in mem_ops.iter().enumerate() {
            let i = idx + 1;
            if mem_op.is_write {
                panic! {"InputDataSM::prove_instance() Input data operation is write"};
            }

            trace[i].addr = F::from_canonical_u64(mem_op.address); // n-byte address, real address = addr * MEM_BYTES
            trace[i].step = F::from_canonical_u64(mem_op.step);
            trace[i].sel = F::one();

            let value = match mem_op.width {
                1 => mem_op.value as u8 as u64,
                2 => mem_op.value as u16 as u64,
                4 => mem_op.value as u32 as u64,
                8 => mem_op.value,
                _ => panic!("Invalid width"),
            };
            let (val0, val1, val2, val3) = self.get_u16_values(value);
            trace[i].value = [
                F::from_canonical_u16(val0),
                F::from_canonical_u16(val1),
                F::from_canonical_u16(val2),
                F::from_canonical_u16(val3),
            ];

            let addr_changes = trace[i - 1].addr != trace[i].addr;
            trace[i].addr_changes = F::from_bool(addr_changes);

            //println! {"InputDataSM::prove_instance() i={} mem op={} addr_changes={}", i, mem_op.to_text(), addr_changes}
        }

        // STEP3. Add dummy rows to the output vector to fill the remaining rows
        //PADDING: At end of memory fill with same addr, incrementing step, same value, sel = 0
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
            //trace[i].wr = F::zero();

            trace[i].value = value;

            trace[i].addr_changes = F::zero();
            //trace[i].same_value = F::one();
            //trace[i].first_addr_access_is_read = F::zero();
        }

        let mut air_instance = AirInstance::new(
            self.wcm.get_sctx(),
            ZISK_AIRGROUP_ID,
            INPUT_DATA_AIR_IDS[0],
            Some(segment_id),
            prover_buffer,
        );

        /*air_instance.set_airvalue(
            &sctx,
            "InputData.mem_segment",
            F::from_canonical_u64(segment_id as u64),
        );
        air_instance.set_airvalue(
            &sctx,
            "ImputData.mem_last_segment",
            F::from_bool(is_last_segment),
        );*/

        pctx.air_instance_repo.add_air_instance(air_instance, Some(global_idx));

        Ok(())
    }

    fn get_u16_values(&self, value: u64) -> (u16, u16, u16, u16) {
        (value as u16, (value >> 16) as u16, (value >> 32) as u16, (value >> 48) as u16)
    }
}

impl<F: PrimeField> WitnessComponent<F> for InputDataSM<F> {}
