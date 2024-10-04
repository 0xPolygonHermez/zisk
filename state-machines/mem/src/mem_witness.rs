use std::sync::Arc;

use p3_field::Field;
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};
use rayon::Scope;
use sm_common::create_buffer_fast;
use zisk_core::ZiskRequiredMemory;
use zisk_pil::{Mem0Row, Mem0Trace, MEM_AIRGROUP_ID, MEM_AIR_IDS};

pub struct MemWitness;

impl MemWitness {
    /// Finalizes the witness accumulation process and triggers the proof generation.
    ///
    /// This method is invoked by the executor when no further witness data remains to be added.
    ///
    /// # Parameters
    ///
    /// - `mem_inputs`: A slice of all `ZiskRequiredMemory` inputs
    pub fn prove_witnesses<F: Field>(
        mut mem_ops: Vec<ZiskRequiredMemory>,
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        sctx: Arc<SetupCtx>,
        scope: &Scope,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // STEP1: Sort the memory inputs by address
        mem_ops.sort_by_key(|mem_input| mem_input.address);

        // STEP2: Process the memory inputs in convert them to AIR instances
        let air = pctx.pilout.get_air(MEM_AIRGROUP_ID, MEM_AIR_IDS[0]);

        // In a Mem AIR instance the first row is a dummy row used for the continuations between AIR segments
        // In a Memory AIR instance, the first row is reserved as a dummy row.
        // This dummy row is used to facilitate the continuation state between different AIR segments.
        // It ensures seamless transitions when multiple AIR segments are processed consecutively.
        // This design avoids discontinuities in memory access patterns and ensures that the memory trace is continuous,
        // For this reason we use AIR num_rows - 1 as the number of rows in each memory AIR instance
        let rows_per_segment = air.num_rows() - 1;
        let mut mem_segment_id = 0;

        while mem_ops.len() >= rows_per_segment {
            let num_drained = std::cmp::min(rows_per_segment, mem_ops.len());

            let drained_inputs = mem_ops.drain(..num_drained).collect::<Vec<_>>();

            let is_empty = mem_ops.is_empty();
            let pctx_cloned = pctx.clone();
            let ectx_cloned = ectx.clone();
            let sctx_cloned = sctx.clone();
            let mem_segment_id_cloned = mem_segment_id;
            let air_num_rows = air.num_rows();
            scope.spawn(move |_| {
                let witness_rows = Self::calculate_witness_rows::<F>(
                    drained_inputs,
                    air_num_rows,
                    mem_segment_id,
                    is_empty,
                );

                Self::create_mem_instance::<F>(
                    witness_rows,
                    MEM_AIR_IDS[0],
                    mem_segment_id_cloned,
                    pctx_cloned,
                    ectx_cloned,
                    sctx_cloned,
                );
            });

            mem_segment_id += 1;
        }

        Ok(())
    }

    fn calculate_witness_rows<F: Field>(
        mem_ops: Vec<ZiskRequiredMemory>,
        air_num_rows: usize,
        mem_segment_id: u64,
        is_last_segment: bool,
    ) -> Vec<Mem0Row<F>> {
        // Create a vector of Mem0Row instances, one for each memory operation
        // Recall that first row is a dummy row used for the continuations between AIR segments
        // The length of the vector is the number of input memory operations plus one because
        // in the prove_witnesses method we drain the memory operations in chunks of n - 1 rows

        let mut output: Vec<Mem0Row<F>> = Vec::with_capacity(air_num_rows);

        // STEP1. Add the dummy row to the output vector as the first row.
        // It is used for the continuations between AIR segments
        let dummy_row = Mem0Row::default();

        // TODO Prepare dummy row

        output.push(dummy_row);

        // STEP2. Add the remaining rows to the output vector
        for mem_op in &mem_ops {
            let mut mem_row = Mem0Row::default();

            mem_row.mem_segment = F::from_canonical_u64(mem_segment_id);
            mem_row.mem_last_segment = F::from_bool(is_last_segment);

            mem_row.addr = F::from_canonical_u64(mem_op.address); // n-byte address, real address = addr * MEM_BYTES
            mem_row.step = F::from_canonical_u64(mem_op.step);
            mem_row.sel = F::default(); // binary selector, if wr == 1, sel must be 1
            mem_row.wr = F::from_bool(mem_op.is_write); // binary write flag

            mem_row.value = [F::default(), F::default()];

            mem_row.addr_changes = F::default(); // binary field
            mem_row.same_value = F::default(); // binary field? in mem.pil there is no explicit constraint
            mem_row.first_addr_access_is_read = F::default();

            output.push(mem_row);
        }

        // STEP3. Add dummy rows to the output vector to fill the remaining rows
        //PADDING: At end of memory fill with same addr, incrementing step, same value, sel = 0, rd = 1, wr = 0
        let last_row_idx = mem_ops.len();
        let addr = output[last_row_idx].addr;
        let mut step = output[last_row_idx].step;
        let value = output[last_row_idx].value;
        for _ in mem_ops.len()..air_num_rows {
            let mut mem_row = Mem0Row::default();

            mem_row.mem_segment = F::from_canonical_u64(mem_segment_id);
            mem_row.mem_last_segment = F::from_bool(true);

            mem_row.addr = addr;
            mem_row.step = step;
            mem_row.sel = F::zero();
            mem_row.wr = F::zero();

            mem_row.value = value;

            mem_row.addr_changes = F::default(); // ???
            mem_row.same_value = F::default(); // ???
            mem_row.first_addr_access_is_read = F::default(); // ???

            output.push(Mem0Row::default());

            step += F::one();
        }
        output
    }

    fn create_mem_instance<F: Field>(
        witness_rows: Vec<Mem0Row<F>>,
        air_id: usize,
        mem_segment_id: u64,
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        sctx: Arc<SetupCtx>,
    ) {
        let mem_trace = Mem0Trace::<F>::map_row_vec(witness_rows, false).unwrap();
        let main_trace_buffer = mem_trace.buffer.unwrap();

        let (buffer_size, offsets) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info(&sctx, MEM_AIRGROUP_ID, air_id)
            .unwrap_or_else(|err| panic!("Error getting buffer info: {}", err));

        let mut buffer = create_buffer_fast(buffer_size as usize);

        let start = offsets[0] as usize;
        let end = start + main_trace_buffer.len();
        use rayon::prelude::*;
        buffer[start..end]
            .par_chunks_mut(main_trace_buffer.len() / rayon::current_num_threads())
            .zip(
                main_trace_buffer
                    .par_chunks(main_trace_buffer.len() / rayon::current_num_threads()),
            )
            .for_each(|(buffer_chunk, main_chunk)| {
                buffer_chunk.copy_from_slice(main_chunk);
            });

        let air_instance =
            AirInstance::new(MEM_AIRGROUP_ID, air_id, Some(mem_segment_id as usize), buffer);

        pctx.air_instance_repo.add_air_instance(air_instance);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p3_field::AbstractField;
    use p3_goldilocks::Goldilocks;
    use zisk_core::ZiskRequiredMemory;

    type GL = Goldilocks;

    #[test]
    fn test_calculate_witness_rows() {
        let mem_ops = vec![
            ZiskRequiredMemory::new(0, true, 0, 1, 0),
            ZiskRequiredMemory::new(1, false, 1, 1, 0),
            ZiskRequiredMemory::new(2, true, 2, 1, 0),
            ZiskRequiredMemory::new(3, false, 3, 1, 0),
            ZiskRequiredMemory::new(4, true, 4, 1, 0),
            ZiskRequiredMemory::new(5, false, 5, 1, 0),
            ZiskRequiredMemory::new(6, true, 6, 1, 0),
            ZiskRequiredMemory::new(7, false, 7, 1, 0),
            ZiskRequiredMemory::new(8, true, 8, 1, 0),
            ZiskRequiredMemory::new(9, false, 9, 1, 0),
        ];

        let witness_rows = MemWitness::calculate_witness_rows::<GL>(mem_ops, 10, 0, true);

        assert_eq!(witness_rows.len(), 10);

        // Check the dummy row
        assert_eq!(witness_rows[0].mem_segment, GL::from_canonical_u64(0));
        assert_eq!(witness_rows[0].mem_last_segment, GL::from_bool(true));
        assert_eq!(witness_rows[0].addr, GL::default());
        assert_eq!(witness_rows[0].step, GL::default());
        assert_eq!(witness_rows[0].sel, GL::default());
        assert_eq!(witness_rows[0].wr, GL::default());
        assert_eq!(witness_rows[0].value, [GL::default(), GL::default()]);
        assert_eq!(witness_rows[0].addr_changes, GL::default());
        assert_eq!(witness_rows[0].same_value, GL::default());
        assert_eq!(witness_rows[0].first_addr_access_is_read, GL::default());

        // Check the remaining rows
        for i in 1..10 {
            assert_eq!(witness_rows[i].mem_segment, GL::from_canonical_u64(0));
            // ...
        }
    }
}
