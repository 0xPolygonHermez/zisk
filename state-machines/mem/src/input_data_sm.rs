use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use crate::{
    MemAirValues, MemInput, MemModule, MemPreviousSegment, MEMORY_MAX_DIFF, MEM_BYTES_BITS,
};
use num_bigint::BigInt;
use p3_field::PrimeField;
use pil_std_lib::Std;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::AirInstance;
use zisk_core::{INPUT_ADDR, MAX_INPUT_SIZE};
use zisk_pil::{InputDataTrace, INPUT_DATA_AIR_IDS, ZISK_AIRGROUP_ID};

const INPUT_W_ADDR_INIT: u32 = INPUT_ADDR as u32 >> MEM_BYTES_BITS;
const INPUT_W_ADDR_END: u32 = (INPUT_ADDR + MAX_INPUT_SIZE - 1) as u32 >> MEM_BYTES_BITS;

#[allow(clippy::assertions_on_constants)]
const _: () = {
    assert!(
        (MAX_INPUT_SIZE - 1) >> MEM_BYTES_BITS as u64 <= MEMORY_MAX_DIFF,
        "INPUT_DATA is too large"
    );
    assert!(
        INPUT_ADDR + MAX_INPUT_SIZE - 1 <= 0xFFFF_FFFF,
        "INPUT_DATA memory exceeds the 32-bit addressable range"
    );
};

pub struct InputDataSM<F: PrimeField> {
    // Witness computation manager
    wcm: Arc<WitnessManager<F>>,

    // STD
    std: Arc<Std<F>>,

    // Count of registered predecessors
    registered_predecessors: AtomicU32,
}

#[allow(unused, unused_variables)]
impl<F: PrimeField> InputDataSM<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>, std: Arc<Std<F>>) -> Arc<Self> {
        let pctx = wcm.get_pctx();
        let air = pctx.pilout.get_air(ZISK_AIRGROUP_ID, INPUT_DATA_AIR_IDS[0]);
        let input_data_sm =
            Self { wcm: wcm.clone(), std: std.clone(), registered_predecessors: AtomicU32::new(0) };
        let input_data_sm = Arc::new(input_data_sm);

        wcm.register_component(
            input_data_sm.clone(),
            Some(ZISK_AIRGROUP_ID),
            Some(INPUT_DATA_AIR_IDS),
        );
        std.register_predecessor();

        input_data_sm
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

        if (inputs.is_empty()) {
            pctx.set_proof_value("enable_input_data", F::zero());
            return;
        }

        pctx.set_proof_value("enable_input_data", F::one());

        let ectx = wcm.get_ectx();
        let sctx = wcm.get_sctx();

        // PRE: proxy calculate if exists jmp on step out-of-range, adding internal inputs
        // memory only need to process these special inputs, but inputs no change. At end of
        // inputs proxy add an extra internal input to jump to last address

        let air_id = INPUT_DATA_AIR_IDS[0];
        let air = pctx.pilout.get_air(ZISK_AIRGROUP_ID, air_id);
        let air_rows = air.num_rows();

        // at least one row to go
        let count = inputs.len();
        let count_rem = count % air_rows;
        let num_segments = (count / air_rows) + if count_rem > 0 { 1 } else { 0 };

        let mut prover_buffers = Mutex::new(vec![Vec::new(); num_segments]);
        let mut global_idxs = vec![0; num_segments];

        #[allow(clippy::needless_range_loop)]
        for i in 0..num_segments {
            // TODO: Review
            if let (true, global_idx) =
                ectx.dctx.write().unwrap().add_instance(ZISK_AIRGROUP_ID, air_id, 1)
            {
                let trace: InputDataTrace<'_, _> = InputDataTrace::new(air_rows);
                let mut buffer = trace.buffer.unwrap();
                prover_buffers.lock().unwrap()[i] = buffer;
                global_idxs[i] = global_idx;
            }
        }

        #[allow(clippy::needless_range_loop)]
        for segment_id in 0..num_segments {
            let is_last_segment = segment_id == num_segments - 1;
            let input_offset = segment_id * air_rows;
            let previous_segment = if (segment_id == 0) {
                MemPreviousSegment { addr: INPUT_W_ADDR_INIT, step: 0, value: 0 }
            } else {
                MemPreviousSegment {
                    addr: inputs[input_offset - 1].addr,
                    step: inputs[input_offset - 1].step,
                    value: inputs[input_offset - 1].value,
                }
            };
            let input_end =
                if (input_offset + air_rows) > count { count } else { input_offset + air_rows };
            let mem_ops = &inputs[input_offset..input_end];
            let prover_buffer = std::mem::take(&mut prover_buffers.lock().unwrap()[segment_id]);

            self.prove_instance(
                mem_ops,
                segment_id,
                is_last_segment,
                &previous_segment,
                prover_buffer,
                air_rows,
                global_idxs[segment_id],
            );
        }
    }

    /// Finalizes the witness accumulation process and triggers the proof generation.
    ///
    /// This method is invoked by the executor when no further witness data remains to be added.
    ///
    /// # Parameters
    ///
    /// - `mem_inputs`: A slice of all `ZiskRequiredMemory` inputs
    #[allow(clippy::too_many_arguments)]
    pub fn prove_instance(
        &self,
        mem_ops: &[MemInput],
        segment_id: usize,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        mut prover_buffer: Vec<F>,
        air_mem_rows: usize,
        global_idx: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert!(
            !mem_ops.is_empty() && mem_ops.len() <= air_mem_rows,
            "InputDataSM: mem_ops.len()={} out of range {}",
            mem_ops.len(),
            air_mem_rows
        );

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

        //println! {"InputDataSM::prove_instance() mem_ops.len={} prover_buffer.len={}
        // air.num_rows={}", mem_ops.len(), prover_buffer.len(), air.num_rows()};
        let mut trace =
            InputDataTrace::<F>::map_buffer(&mut prover_buffer, air_mem_rows, 0).unwrap();

        let mut range_check_data: Vec<u64> = vec![0; 1 << 16];

        let mut air_values = MemAirValues {
            segment_id: segment_id as u32,
            is_first_segment: segment_id == 0,
            is_last_segment,
            previous_segment_addr: previous_segment.addr,
            previous_segment_step: previous_segment.step,
            previous_segment_value: [
                previous_segment.value as u32,
                (previous_segment.value >> 32) as u32,
            ],
            ..MemAirValues::default()
        };

        // range of instance
        let range_id = self.std.get_range(BigInt::from(1), BigInt::from(MEMORY_MAX_DIFF), None);
        self.std.range_check(
            F::from_canonical_u32(previous_segment.addr - INPUT_W_ADDR_INIT + 1),
            F::one(),
            range_id,
        );

        // Fill the remaining rows
        let mut last_addr: u32 = previous_segment.addr;
        let mut last_step: u64 = previous_segment.step;
        let mut last_value: u64 = previous_segment.value;

        for (i, mem_op) in mem_ops.iter().enumerate() {
            trace[i].addr = F::from_canonical_u32(mem_op.addr);
            trace[i].step = F::from_canonical_u64(mem_op.step);
            trace[i].sel = F::from_bool(!mem_op.is_internal);

            let value = mem_op.value;
            let value_words = self.get_u16_values(value);
            for j in 0..4 {
                range_check_data[value_words[j] as usize] += 1;
                trace[i].value_word[j] = F::from_canonical_u16(value_words[j]);
            }

            let addr_changes = last_addr != mem_op.addr;
            trace[i].addr_changes =
                if addr_changes || (i == 0 && segment_id == 0) { F::one() } else { F::zero() };

            last_addr = mem_op.addr;
            last_step = mem_op.step;
            last_value = mem_op.value;
        }

        // STEP3. Add dummy rows to the output vector to fill the remaining rows
        //PADDING: At end of memory fill with same addr, incrementing step, same value, sel = 0
        let last_row_idx = mem_ops.len() - 1;
        let addr = trace[last_row_idx].addr;
        let value = trace[last_row_idx].value_word;

        let padding_size = air_mem_rows - mem_ops.len();
        for i in mem_ops.len()..air_mem_rows {
            last_step += 1;

            // TODO CHECK
            // trace[i].mem_segment = segment_id_field;
            // trace[i].mem_last_segment = is_last_segment_field;

            trace[i].addr = addr;
            trace[i].step = F::from_canonical_u64(last_step);
            trace[i].sel = F::zero();

            trace[i].value_word = value;

            trace[i].addr_changes = F::zero();
        }

        air_values.segment_last_addr = last_addr;
        air_values.segment_last_step = last_step;
        air_values.segment_last_value[0] = last_value as u32;
        air_values.segment_last_value[1] = (last_value >> 32) as u32;

        self.std.range_check(
            F::from_canonical_u32(INPUT_W_ADDR_END - last_addr + 1),
            F::one(),
            range_id,
        );

        // range of chunks
        let range_id = self.std.get_range(BigInt::from(0), BigInt::from((1 << 16) - 1), None);
        for (value, &multiplicity) in range_check_data.iter().enumerate() {
            if (multiplicity == 0) {
                continue;
            }

            self.std.range_check(
                F::from_canonical_usize(value),
                F::from_canonical_u64(multiplicity),
                range_id,
            );
        }
        for value_chunk in &value {
            self.std.range_check(*value_chunk, F::from_canonical_usize(padding_size), range_id);
        }

        let wcm = self.wcm.clone();
        let pctx = wcm.get_pctx();
        let sctx = wcm.get_sctx();

        let mut air_instance = AirInstance::new(
            self.wcm.get_sctx(),
            ZISK_AIRGROUP_ID,
            INPUT_DATA_AIR_IDS[0],
            Some(segment_id),
            prover_buffer,
        );

        self.set_airvalues("InputData", &mut air_instance, &air_values);

        pctx.air_instance_repo.add_air_instance(air_instance, Some(global_idx));

        Ok(())
    }

    fn get_u16_values(&self, value: u64) -> [u16; 4] {
        [value as u16, (value >> 16) as u16, (value >> 32) as u16, (value >> 48) as u16]
    }
    fn set_airvalues(
        &self,
        prefix: &str,
        air_instance: &mut AirInstance<F>,
        air_values: &MemAirValues,
    ) {
        air_instance.set_airvalue(
            format!("{}.segment_id", prefix).as_str(),
            None,
            F::from_canonical_u32(air_values.segment_id),
        );
        air_instance.set_airvalue(
            format!("{}.is_first_segment", prefix).as_str(),
            None,
            F::from_bool(air_values.is_first_segment),
        );
        air_instance.set_airvalue(
            format!("{}.is_last_segment", prefix).as_str(),
            None,
            F::from_bool(air_values.is_last_segment),
        );
        air_instance.set_airvalue(
            format!("{}.previous_segment_addr", prefix).as_str(),
            None,
            F::from_canonical_u32(air_values.previous_segment_addr),
        );
        air_instance.set_airvalue(
            format!("{}.previous_segment_step", prefix).as_str(),
            None,
            F::from_canonical_u64(air_values.previous_segment_step),
        );
        air_instance.set_airvalue(
            format!("{}.segment_last_addr", prefix).as_str(),
            None,
            F::from_canonical_u32(air_values.segment_last_addr),
        );
        air_instance.set_airvalue(
            format!("{}.segment_last_step", prefix).as_str(),
            None,
            F::from_canonical_u64(air_values.segment_last_step),
        );
        let count = air_values.previous_segment_value.len();
        for i in 0..count {
            air_instance.set_airvalue(
                format!("{}.previous_segment_value", prefix).as_str(),
                Some(vec![i as u64]),
                F::from_canonical_u32(air_values.previous_segment_value[i]),
            );
            air_instance.set_airvalue(
                format!("{}.segment_last_value", prefix).as_str(),
                Some(vec![i as u64]),
                F::from_canonical_u32(air_values.segment_last_value[i]),
            );
        }
    }
}

impl<F: PrimeField> MemModule<F> for InputDataSM<F> {
    fn send_inputs(&self, mem_op: &[MemInput]) {
        self.prove(mem_op);
    }
    fn get_addr_ranges(&self) -> Vec<(u32, u32)> {
        vec![(INPUT_ADDR as u32, (INPUT_ADDR + MAX_INPUT_SIZE - 1) as u32)]
    }
    fn get_flush_input_size(&self) -> u32 {
        0
    }
}

impl<F: PrimeField> WitnessComponent<F> for InputDataSM<F> {}
