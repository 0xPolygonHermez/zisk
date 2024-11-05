use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use p3_field::PrimeField;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};
use rayon::Scope;
use sm_common::{MemOp, OpResult, Provable};
use zisk_core::ZiskRequiredMemory;
// use zisk_pil::{Mem0Trace, MEM_AIRGROUP_ID, MEM_AIR_IDS};

const PROVE_CHUNK_SIZE: usize = 1 << 12;

pub struct MemSM<F: PrimeField> {
    // Witness computation manager
    wcm: Arc<WitnessManager<F>>,

    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Inputs
    inputs: Mutex<Vec<MemOp>>,

    _phantom: std::marker::PhantomData<F>,
}

#[allow(unused, unused_variables)]
impl<F: PrimeField> MemSM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let mem_sm = Self {
            wcm: wcm.clone(),
            registered_predecessors: AtomicU32::new(0),
            inputs: Mutex::new(Vec::new()),
            _phantom: std::marker::PhantomData,
        };
        let mem_sm = Arc::new(mem_sm);

        // wcm.register_component(mem_sm.clone(), Some(MEM_AIRGROUP_ID), Some(MEM_AIR_IDS));

        mem_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor(&self) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            // <MemSM<F> as Provable<MemOp, OpResult>>::prove(self, &[], true, scope);
        }
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
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        sctx: Arc<SetupCtx>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // STEP2: Process the memory inputs and convert them to AIR instances
        // let air = pctx.pilout.get_air(MEM_AIRGROUP_ID, MEM_AIR_IDS[0]);

        // let max_rows_per_segment = air.num_rows() - 1;

        // assert!(mem_ops.len() > 0 && mem_ops.len() <= max_rows_per_segment);

        // // In a Mem AIR instance the first row is a dummy row used for the continuations between AIR segments
        // // In a Memory AIR instance, the first row is reserved as a dummy row.
        // // This dummy row is used to facilitate the continuation state between different AIR segments.
        // // It ensures seamless transitions when multiple AIR segments are processed consecutively.
        // // This design avoids discontinuities in memory access patterns and ensures that the memory trace is continuous,
        // // For this reason we use AIR num_rows - 1 as the number of rows in each memory AIR instance

        // // Create a vector of Mem0Row instances, one for each memory operation
        // // Recall that first row is a dummy row used for the continuations between AIR segments
        // // The length of the vector is the number of input memory operations plus one because
        // // in the prove_witnesses method we drain the memory operations in chunks of n - 1 rows

        // let mut trace =
        //     Mem0Trace::<F>::map_buffer(&mut prover_buffer, air.num_rows(), offset as usize)
        //         .unwrap();

        // let segment_id_field = F::from_canonical_u64(segment_id as u64);
        // let is_last_segment_field = F::from_bool(is_last_segment);

        // // STEP1. Add the first row to the output vector as equal to the last row of the previous segment
        // // CASE: last row of segment is read
        // //
        // // S[n-1]    wr = 0, sel = 1, addr, step, value
        // // S+1[0]    wr = 0, sel = 0, addr, step, value
        // //
        // // CASE: last row of segment is write
        // //
        // // S[n-1]    wr = 1, sel = 1, addr, step, value
        // // S+1[0]    wr = 0, sel = 0, addr, step, value

        // trace[0].mem_segment = segment_id_field;
        // trace[0].mem_last_segment = is_last_segment_field;

        // trace[0].addr = F::from_canonical_u64(mem_first_row.address);
        // trace[0].step = F::from_canonical_u64(mem_first_row.step);
        // trace[0].sel = F::zero();
        // trace[0].wr = F::zero();

        // let value = match mem_first_row.width {
        //     1 => mem_first_row.value as u8 as u64,
        //     2 => mem_first_row.value as u16 as u64,
        //     4 => mem_first_row.value as u32 as u64,
        //     8 => mem_first_row.value,
        //     _ => panic!("Invalid width"),
        // };
        // let (low_val, high_val) = self.get_u32_values(value);
        // trace[0].value = [F::from_canonical_u32(low_val), F::from_canonical_u32(high_val)];
        // trace[0].addr_changes = F::zero();

        // trace[0].same_value = F::zero();
        // trace[0].first_addr_access_is_read = F::zero();

        // // STEP2. Add all the memory operations to the buffer
        // for (idx, mem_op) in mem_ops.iter().enumerate() {
        //     let i = idx + 1;
        //     trace[i].mem_segment = segment_id_field;
        //     trace[i].mem_last_segment = is_last_segment_field;

        //     trace[i].addr = F::from_canonical_u64(mem_op.address); // n-byte address, real address = addr * MEM_BYTES
        //     trace[i].step = F::from_canonical_u64(mem_op.step);
        //     trace[i].sel = F::one();
        //     trace[i].wr = F::from_bool(mem_op.is_write);

        //     let value = match mem_op.width {
        //         1 => mem_op.value as u8 as u64,
        //         2 => mem_op.value as u16 as u64,
        //         4 => mem_op.value as u32 as u64,
        //         8 => mem_op.value,
        //         _ => panic!("Invalid width"),
        //     };
        //     let (low_val, high_val) = self.get_u32_values(value);
        //     trace[i].value = [F::from_canonical_u32(low_val), F::from_canonical_u32(high_val)];
        //     if i == 66587 || i == 66586 {
        //         println!(
        //             "mem_op.value: {:?} value: {:?} width: {}",
        //             mem_op.value, trace[i].value, mem_op.width
        //         );
        //         println!("mem_op: {:?}", mem_op);
        //     }
        //     let addr_changes = trace[i - 1].addr != trace[i].addr;
        //     trace[i].addr_changes = if addr_changes { F::one() } else { F::zero() };

        //     let same_value = trace[i - 1].value[0] == trace[i].value[0]
        //         && trace[i - 1].value[1] == trace[i].value[1];
        //     trace[i].same_value = if same_value { F::one() } else { F::zero() };

        //     let first_addr_access_is_read = addr_changes && !mem_op.is_write;
        //     trace[i].first_addr_access_is_read =
        //         if first_addr_access_is_read { F::one() } else { F::zero() };

        //     if i == 66587 || i == 66586 {
        //         println!("trace[{}]: {:?}", i, trace[i]);
        //     }
        // }

        // // STEP3. Add dummy rows to the output vector to fill the remaining rows
        // //PADDING: At end of memory fill with same addr, incrementing step, same value, sel = 0, rd = 1, wr = 0
        // let last_row_idx = mem_ops.len();
        // let addr = trace[last_row_idx].addr;
        // let mut step = trace[last_row_idx].step;
        // let value = trace[last_row_idx].value;

        // for i in (mem_ops.len() + 1)..air.num_rows() {
        //     step += F::one();

        //     trace[i].mem_segment = segment_id_field;
        //     trace[i].mem_last_segment = is_last_segment_field;

        //     trace[i].addr = addr;
        //     trace[i].step = step;
        //     trace[i].sel = F::zero();
        //     trace[i].wr = F::zero();

        //     trace[i].value = value;

        //     trace[i].addr_changes = F::zero();
        //     trace[i].same_value = F::one();
        //     trace[i].first_addr_access_is_read = F::zero();
        // }

        // let air_instance = AirInstance::new(
        //     self.wcm.get_sctx(),
        //     MEM_AIRGROUP_ID,
        //     MEM_AIR_IDS[0],
        //     Some(segment_id),
        //     prover_buffer,
        // );

        // pctx.air_instance_repo.add_air_instance(air_instance);

        Ok(())
    }

    fn get_u32_values(&self, value: u64) -> (u32, u32) {
        (value as u32, (value >> 32) as u32)
    }
}

impl<F: PrimeField> WitnessComponent<F> for MemSM<F> {}

impl<F: PrimeField> Provable<MemOp, OpResult> for MemSM<F> {
    fn prove(&self, operations: &[MemOp], drain: bool, scope: &Scope) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.extend_from_slice(operations);

            while inputs.len() >= PROVE_CHUNK_SIZE || (drain && !inputs.is_empty()) {
                let num_drained = std::cmp::min(PROVE_CHUNK_SIZE, inputs.len());
                let _drained_inputs = inputs.drain(..num_drained).collect::<Vec<_>>();

                scope.spawn(move |_| {
                    // TODO! Implement prove drained_inputs (a chunk of operations)
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    // use p3_field::AbstractField;
    // use p3_goldilocks::Goldilocks;
    // use zisk_core::ZiskRequiredMemory;

    // type GL = Goldilocks;

    // #[test]
    // fn test_calculate_witness_rows() {
    //     let mem_ops = vec![
    //         ZiskRequiredMemory::new(0, true, 0, 1, 0),
    //         ZiskRequiredMemory::new(1, false, 1, 1, 0),
    //         ZiskRequiredMemory::new(2, true, 2, 1, 0),
    //         ZiskRequiredMemory::new(3, false, 3, 1, 0),
    //         ZiskRequiredMemory::new(4, true, 4, 1, 0),
    //         ZiskRequiredMemory::new(5, false, 5, 1, 0),
    //         ZiskRequiredMemory::new(6, true, 6, 1, 0),
    //         ZiskRequiredMemory::new(7, false, 7, 1, 0),
    //         ZiskRequiredMemory::new(8, true, 8, 1, 0),
    //         ZiskRequiredMemory::new(9, false, 9, 1, 0),
    //     ];

    //     let witness_rows = MemWitness::calculate_witness_rows::<GL>(mem_ops, 10, 0, true);

    //     assert_eq!(witness_rows.len(), 10);

    //     // Check the dummy row
    //     assert_eq!(witness_rows[0].mem_segment, GL::from_canonical_u64(0));
    //     assert_eq!(witness_rows[0].mem_last_segment, GL::from_bool(true));
    //     assert_eq!(witness_rows[0].addr, GL::default());
    //     assert_eq!(witness_rows[0].step, GL::default());
    //     assert_eq!(witness_rows[0].sel, GL::default());
    //     assert_eq!(witness_rows[0].wr, GL::default());
    //     assert_eq!(witness_rows[0].value, [GL::default(), GL::default()]);
    //     assert_eq!(witness_rows[0].addr_changes, GL::default());
    //     assert_eq!(witness_rows[0].same_value, GL::default());
    //     assert_eq!(witness_rows[0].first_addr_access_is_read, GL::default());

    //     // Check the remaining rows
    //     for i in 1..10 {
    //         assert_eq!(witness_rows[i].mem_segment, GL::from_canonical_u64(0));
    //         // ...
    //     }
    // }
}
