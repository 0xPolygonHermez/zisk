use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use p3_field::PrimeField;

use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};

use proofman_common as common;
pub use proofman_macros::trace;

use crate::Range;

// PIL Helpers
trace!(SpecifiedRanges0Row, SpecifiedRanges0Trace<F> {
    mul: [F; 32], // TODO: This number cannot be hardcorded, it depens on the air that instantiates the range check
});

pub struct SpecifiedRanges<F: PrimeField> {
    air_group_id: usize,
    air_id: usize,
    inputs: Mutex<HashMap<Range<F>, HashMap<F, F>>>, // range -> value -> multiplicity
}

impl<F: PrimeField> SpecifiedRanges<F> {
    const MY_NAME: &'static str = "SpecifiedRanges";

    pub fn new(wcm: &mut WitnessManager<F>, air_group_id: usize, air_id: usize) -> Arc<Self> {
        let specified_ranges = Arc::new(Self {
            air_group_id,
            air_id,
            inputs: Mutex::new(HashMap::new()),
        });

        wcm.register_component(
            specified_ranges.clone(),
            Some(air_group_id),
            Some(&[air_id]),
        );

        specified_ranges
    }

    pub fn update_inputs(&self, value: F, range: Range<F>) {
        let mut inputs_specified = self.inputs.lock().unwrap();
        let range = inputs_specified.entry(range).or_insert(HashMap::new());

        // Update the value
        *range.entry(value).or_insert(F::zero()) += F::one();
    }
}

impl<F: PrimeField> WitnessComponent<F> for SpecifiedRanges<F> {
    fn start_proof(&self, pctx: &ProofCtx<F>, ectx: &ExecutionCtx, _sctx: &SetupCtx) {
        let (buffer_size, _) = ectx
            .buffer_allocator
            .as_ref()
            .get_buffer_info("SpecifiedRanges".into(), self.air_id)
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
            "SpecifiedRanges".to_string(),
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
                .get_buffer_info("SpecifiedRanges".to_string(), air_instance.air_id)
                .unwrap();
            let mut buffer = air_instance.buffer.as_mut().unwrap();

            // Update the multiplicity column
            let inputs = self.inputs.lock().unwrap();
            let mut trace =
                SpecifiedRanges0Trace::map_buffer(&mut buffer, num_rows, offsets[0] as usize)
                    .unwrap();

            for k in 0..trace[0].mul.len() {
                let range = inputs
                    .keys()
                    .nth(k)
                    .expect("Rc::calculate_trace() range not found")
                    .clone();
                let min = range.0;
                let max = range.1;
                for i in 0..num_rows {
                    // Ranges doesn't necessarily have to be a power of two
                    // so we must adjust the multiplicity to that case
                    if F::from_canonical_usize(i) >= max - min + F::one() {
                        trace[k].mul[i] = F::zero();
                    } else {
                        trace[k].mul[i] = *inputs
                            .get(&range)
                            .unwrap()
                            .clone()
                            .entry(F::from_canonical_usize(i))
                            .or_insert(F::zero());
                    }
                }
            }
        }

        log::info!(
            "{}: Completed witness computation for AIR '{}' at stage {}",
            Self::MY_NAME,
            "SpecifiedRanges".to_string(),
            stage
        );
    }
}
