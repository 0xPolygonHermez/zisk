use common::{AirInstance, AirInstanceWitnessComputation, ExecutionCtx, ProofCtx, WitnessPilOut};
use goldilocks::AbstractField;
use log::trace;

use crate::component::{Component, ComponentOutput};

pub struct MemorySM<'a, F> {
    _phantom: std::marker::PhantomData<&'a F>,
}

impl<'a, F> MemorySM<'a, F> {
    const MY_NAME: &'static str = "MemSM   ";
    const DEFAULT_ID: u16 = 4;

    pub fn new() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }
}
#[allow(dead_code)]
pub struct MemSMMetadata {}

#[allow(unused_variables)]
impl<'a, F> AirInstanceWitnessComputation<'a, F> for MemorySM<'a, F> {
    fn start_proof(&self, proof_ctx: &mut ProofCtx<F>, execution_ctx: &ExecutionCtx, pilout: &WitnessPilOut) {
        trace!("{}: ··· Starting proof", Self::MY_NAME);

        // TO BE REMOVED For testing purposes only we decide to add some mock data here.
        // let air_id = pilout.find_air_id_by_name(0, "Arith_16").unwrap();
        if execution_ctx.air_instances_map {
            proof_ctx.air_instances_map.add_air_instance(
                0,
                AirInstance {
                    air_group_id: 0,
                    air_id: 7, // Hardcoded, to be removed
                    instance_id: None,
                    meta: Some(Box::new(MemSMMetadata {})),
                },
            );
        }
    }

    fn end_proof(&self, proof_ctx: &ProofCtx<F>) {
        trace!("Ending proof for MemSM");
    }

    fn calculate_witness(&self, stage: u32, proof_ctx: &ProofCtx<F>, air_instance: &AirInstance) {
        trace!("Calculating witness for MemSM at stage {}", stage);
    }
}

impl<'a, F: AbstractField> Component<F> for MemorySM<'a, F> {
    fn init(&mut self) {}

    fn finish(&mut self) {}

    fn get_default_id(&self) -> u16 {
        Self::DEFAULT_ID
    }

    fn calculate_free_input(&self, _values: Vec<F>) -> ComponentOutput<F> {
        ComponentOutput::Single(F::one())
    }

    fn verify(&self, _values: Vec<F>) -> bool {
        unimplemented!()
    }
}
