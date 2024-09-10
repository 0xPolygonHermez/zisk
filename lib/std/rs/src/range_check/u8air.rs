use std::sync::{Arc, Mutex};

use p3_field::PrimeField;
use num_traits::ToPrimitive;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};

use proofman_common as common;
pub use proofman_macros::trace;

// PIL Helpers
trace!(U8Air0Row, U8Air0Trace<F> {
    mul: F,
});

pub struct U8Air<F> {
    air_group_id: usize,
    air_id: usize,
    inputs: Mutex<Vec<F>>, // value -> multiplicity
}

const PROVE_CHUNK_SIZE: usize = 1 << 10;

impl<F: PrimeField> U8Air<F> {
    const MY_NAME: &'static str = "U8Air";

    pub fn new(wcm: &mut WitnessManager<F>, air_group_id: usize, air_id: usize) -> Arc<Self> {
        let u8air = Arc::new(Self {
            air_group_id,
            air_id,
            inputs: Mutex::new(Vec::new()),
        });

        wcm.register_component(u8air.clone(), Some(air_group_id), Some(&[air_id]));

        u8air
    }

    pub fn update_inputs(&self, value: F, drain: bool, pctx: &ProofCtx<F>, ectx: &ExecutionCtx) {
        if let Ok(mut inputs) = self.inputs.lock() { 
            inputs.push(value);

            while inputs.len() >= PROVE_CHUNK_SIZE || (drain && !inputs.is_empty()) {
                let num_drained = std::cmp::min(PROVE_CHUNK_SIZE, inputs.len());
                let drained_inputs = inputs.drain(..num_drained).collect::<Vec<_>>();

                // TODO! Do it in parallel

                // Update multiplicity with drained inputs
                let air_instances_vec = &mut pctx.air_instances.write().unwrap();
                let air_instance_id = pctx.find_air_instances(self.air_group_id, self.air_id);
                let air_instance = &mut air_instances_vec[air_instance_id[0]];

                // Get the air associated with the air_instance
                let air_group_id = air_instance.air_group_id;
                let air_id = air_instance.air_id;
                let air = pctx.pilout.get_air(air_group_id, air_id);

                log::info!(
                    "{}: Initiating witness computation for AIR '{}' at stage {}",
                    Self::MY_NAME,
                    air.name().unwrap_or("unknown"),
                    1
                );

                let num_rows = air.num_rows();

                let (_, offsets) = ectx
                    .buffer_allocator
                    .as_ref()
                    .get_buffer_info("U8Air".to_string(), air_instance.air_id)
                    .unwrap();
                let mut buffer = air_instance.buffer.as_mut().unwrap();

                // Update the multiplicity column
                let mut trace =
                    U8Air0Trace::map_buffer(&mut buffer, num_rows, offsets[0] as usize).unwrap();

                for i in 0..drained_inputs.len() {
                    let value = drained_inputs[i].as_canonical_biguint().to_usize().expect("Cannot convert to usize");
                    // We can add a sanity check cheking than 0 <= value < num_rows
                    trace[value].mul += F::one();
                }

                log::info!(
                    "{}: Completed witness computation for AIR '{}' at stage {}",
                    Self::MY_NAME,
                    "U8Air".to_string(),
                    1
                );
            }
        }
    }
}

impl<F: PrimeField> WitnessComponent<F> for U8Air<F> {
    fn start_proof(&self, pctx: &ProofCtx<F>, ectx: &ExecutionCtx, _sctx: &SetupCtx) {
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("U8Air".into(), self.air_id)
            .unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        pctx.add_air_instance_ctx(self.air_group_id, self.air_id, None, Some(buffer));
    }

    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: Option<usize>,
        _pctx: &mut ProofCtx<F>,
        _ectx: &ExecutionCtx,
        _sctx: &SetupCtx,
    ) {
        return;
    }
}
