use common::{AirInstance, AirInstanceWitnessComputation, ExecutionCtx, ProofCtx, WitnessPilOut};
use log::trace;
use wcmanager::WitnessExecutor;

pub struct MainSM<'a, F> {
    _phantom: std::marker::PhantomData<&'a F>,
}

impl<'a, F> MainSM<'a, F> {
    const MY_NAME: &'static str = "MainSM  ";

    pub fn new() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }
}
#[allow(dead_code)]
pub struct MainSMMetadata {}

#[allow(unused_variables)]
impl<'a, F> AirInstanceWitnessComputation<'a, F> for MainSM<'a, F> {
    fn start_proof(&self, proof_ctx: &mut ProofCtx<F>, execution_ctx: &ExecutionCtx, pilout: &WitnessPilOut) {
        trace!("{}: ··· Starting proof", Self::MY_NAME);

        // TO BE REMOVED For testing purposes only we decide to add some mock data here.
        // let air_id = pilout.find_air_id_by_name(0, "Arith_16").unwrap();
        if execution_ctx.air_instances_map {
            proof_ctx.air_instances_map.add_air_instance(
                0,
                AirInstance {
                    air_group_id: 0,
                    air_id: 3, // Hardcoded, to be removed
                    instance_id: None,
                    meta: Some(Box::new(MainSMMetadata {})),
                },
            );
            proof_ctx.air_instances_map.add_air_instance(
                0,
                AirInstance {
                    air_group_id: 0,
                    air_id: 4, // Hardcoded, to be removed
                    instance_id: None,
                    meta: Some(Box::new(MainSMMetadata {})),
                },
            );
        }
    }

    fn end_proof(&self, proof_ctx: &ProofCtx<F>) {
        trace!("Ending proof for MainSM");
    }

    fn calculate_witness(&self, stage: u32, proof_ctx: &ProofCtx<F>, air_instance: &AirInstance) {
        trace!("Calculating witness for MainSM at stage {}", stage);
    }
}

#[allow(dead_code, unused_variables)]
impl<'a, F> WitnessExecutor<'a, F> for MainSM<'a, F> {
    fn start_execute(&mut self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx) {
        trace!("{}: ··· Starting execution", Self::MY_NAME);
    }

    fn execute(&mut self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx) {
        trace!("{}: ··· Executing", Self::MY_NAME);
        // arith.startExecute(ctx, ectx);
        // bin.starteExecute(ctx, ectx);
        // let mainCtx = proofs.get(ctx.idProof);
        // ..
        // ..
        // arith.mul(ctx, a, b)
        // ..
        // ..
        // arith.endExecute(ctx);
        // bin.endExecute(ctx);
    }

    fn end_execute(&mut self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx) {
        trace!("{}: ··· Ending execution", Self::MY_NAME);
    }
}
