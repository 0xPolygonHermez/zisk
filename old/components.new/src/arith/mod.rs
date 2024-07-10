extern crate pil2_stark;

use common::{AirInstance, AirInstanceWitnessComputation, ExecutionCtx, ProofCtx, WitnessPilOut};
use goldilocks::AbstractField;
use log::trace;

use crate::component::{Component, ComponentOutput};
// use proofman::trace;

pub struct ArithSM<'a, F> {
    _phantom: std::marker::PhantomData<&'a F>,
}

#[allow(dead_code, unused_variables)]
impl<'a, F> ArithSM<'a, F> {
    const MY_NAME: &'static str = "ArithSM ";
    const DEFAULT_ID: u16 = 4;

    pub fn new() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }

    fn start(&self) {
        // rangeCheck.start(ctx, ectx);
    }

    fn execute(&self, proof_ctx: &ProofCtx<F>) {
        // arithCtx = proofs.get(ctx.idProof);

        let a = 10;
        let b = 20;
        let c = a * b;

        if true { // Add option in airth context, if arith.callrangeCheck

            // let task = async rangeCheck.check(a).now_or_never()
            // arithCtx.asyncs.push(task);
            // let task = async rangeCheck.check(b).now_or_never()
            // arithCtx.asyncs.push(task);
        }

        // c
    }

    fn end() {
        // join_all(arithCtx.asyncs).await
    }
}

#[allow(dead_code)]
pub struct ArithSMMetadata {}

#[allow(unused_variables)]
impl<'a, F> AirInstanceWitnessComputation<'a, F> for ArithSM<'a, F> {
    fn start_proof(&self, proof_ctx: &mut ProofCtx<F>, execution_ctx: &ExecutionCtx, pilout: &WitnessPilOut) {
        trace!("{}: ··· Starting proof", Self::MY_NAME);

        // TO BE REMOVED For testing purposes only we decide to add some mock data here.
        // let air_id = pilout.find_air_id_by_name(0, "Arith_16").unwrap();
        if execution_ctx.air_instances_map {
            proof_ctx.air_instances_map.add_air_instance(
                0,
                AirInstance {
                    air_group_id: 0,
                    air_id: 0, // Hardcoded, to be removed
                    instance_id: None,
                    meta: Some(Box::new(ArithSMMetadata {})),
                },
            );
            proof_ctx.air_instances_map.add_air_instance(
                0,
                AirInstance {
                    air_group_id: 0,
                    air_id: 1, // Hardcoded, to be removed
                    instance_id: None,
                    meta: Some(Box::new(ArithSMMetadata {})),
                },
            );
        }
    }

    fn end_proof(&self, proof_ctx: &ProofCtx<F>) {
        trace!("Ending proof for ArithSM");
    }

    fn calculate_witness(&self, stage: u32, proof_ctx: &ProofCtx<F>, air_instance: &AirInstance) {
        trace!("Calculating witness for ArithSM at stage {}", stage);
    }
}

#[allow(dead_code)]
impl<'a, F: AbstractField> Component<F> for ArithSM<'a, F> {
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
