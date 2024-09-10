use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use p3_field::PrimeField;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};

use proofman_common as common;
pub use proofman_macros::trace;

// PIL Helpers
trace!(U16Air0Row, U16Air0Trace<F> {
    mul: F,
});

pub struct U16Air<F> {
    air_group_id: usize,
    air_id: usize,
    inputs: Mutex<HashMap<F, F>>, // value -> multiplicity
}

impl<F: PrimeField> U16Air<F> {
    const MY_NAME: &'static str = "U16Air";

    pub fn new(wcm: &mut WitnessManager<F>, air_group_id: usize, air_id: usize) -> Arc<Self> {
        let u16air = Arc::new(Self {
            air_group_id,
            air_id,
            inputs: Mutex::new(HashMap::new()),
        });

        wcm.register_component(u16air.clone(), Some(air_group_id), Some(&[air_id]));

        u16air
    }

    pub fn update_inputs(&self, value: F) {
        let mut inputs = self.inputs.lock().unwrap();
        *inputs.entry(value).or_insert(F::zero()) += F::one();
    }
}

impl<F: PrimeField> WitnessComponent<F> for U16Air<F> {
    fn start_proof(&self, pctx: &ProofCtx<F>, ectx: &ExecutionCtx, _sctx: &SetupCtx) {
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("U16Air".into(), self.air_id)
            .unwrap();

        let buffer = vec![F::zero(); buffer_size as usize];

        pctx.add_air_instance_ctx(self.air_group_id, self.air_id, None, Some(buffer));
    }

    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: Option<usize>,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        _sctx: &SetupCtx,
    ) {
        log::info!(
            "{}: Initiating witness computation for AIR '{}' at stage {}",
            Self::MY_NAME,
            "U16Air".to_string(),
            stage
        );

        if stage == 1 {
            let air_instances_vec = &mut pctx.air_instances.write().unwrap();
            let air_instance_id = air_instance;
            let air_instance = &mut air_instances_vec[air_instance_id.unwrap()];

            // Get the air associated with the air_instance
            let air_group_id = air_instance.air_group_id;
            let air_id = air_instance.air_id;
            let air = pctx.pilout.get_air(air_group_id, air_id);

            log::info!(
                "{}: Initiating witness computation for AIR '{}' at stage {}",
                Self::MY_NAME,
                air.name().unwrap_or("unknown"),
                stage
            );

            let num_rows = air.num_rows();

            let (_, offsets) = ectx
                .buffer_allocator
                .as_ref()
                .get_buffer_info("U16Air".to_string(), air_instance.air_id)
                .unwrap();
            let mut buffer = air_instance.buffer.as_mut().unwrap();

            // Update the multiplicity column
            let inputs = self.inputs.lock().unwrap();
            let mut trace =
                U16Air0Trace::map_buffer(&mut buffer, num_rows, offsets[0] as usize).unwrap();

            for i in 0..num_rows {
                trace[i].mul = inputs
                    .get(&F::from_canonical_usize(i))
                    .cloned()
                    .unwrap_or(F::zero());
            }
        }

        log::info!(
            "{}: Completed witness computation for AIR '{}' at stage {}",
            Self::MY_NAME,
            "U16Air".to_string(),
            stage
        );
    }
}
