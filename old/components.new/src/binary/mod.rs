use common::{AirInstance, AirInstanceWitnessComputation, ExecutionCtx, ProofCtx, WitnessPilOut};
use log::trace;

pub struct BinarySM<'a, F> {
    _phantom: std::marker::PhantomData<&'a F>,
}

impl<'a, F> BinarySM<'a, F> {
    const MY_NAME: &'static str = "BinarySM";

    pub fn new() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }
}

#[allow(dead_code)]
pub struct BinarySMMetadata {}

#[allow(unused_variables)]
impl<'a, F> AirInstanceWitnessComputation<'a, F> for BinarySM<'a, F> {
    fn start_proof(&self, proof_ctx: &mut ProofCtx<F>, execution_ctx: &ExecutionCtx, pilout: &WitnessPilOut) {
        trace!("{}: ··· Starting proof", Self::MY_NAME);

        // TO BE REMOVED For testing purposes only we decide to add some mock data here.
        // let air_id = pilout.find_air_id_by_name(0, "Binary_18").unwrap();
        if execution_ctx.air_instances_map {
            proof_ctx.air_instances_map.add_air_instance(
                0,
                AirInstance {
                    air_group_id: 0,
                    air_id: 2, // Hardcoded, to be removed
                    instance_id: None,
                    meta: Some(Box::new(BinarySMMetadata {})),
                },
            );
        }
    }

    fn end_proof(&self, proof_ctx: &ProofCtx<F>) {
        trace!("Ending proof for BinarySM");
    }

    fn calculate_witness(&self, stage: u32, proof_ctx: &ProofCtx<F>, air_instance: &AirInstance) {
        trace!("Calculating witness for BinarySM at stage {}", stage);
    }
}
