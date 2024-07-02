extern crate pil2_stark;

use log::trace;
use pil2_stark::*;
// use proofman::trace;

pub struct ArithSM<F> {
    _phantom: std::marker::PhantomData<F>,
}

#[allow(dead_code, unused_variables)]
impl<F> ArithSM<F> {
    const MY_NAME: &'static str = "ArithSM ";

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

// async fn async_task(id: usize) -> usize {
//     println!("Task {} started", id);
//     sleep(Duration::from_secs(id as u64)).await;
//     println!("Task {} completed", id);
//     id
// }

#[allow(dead_code)]
pub struct ArithSMMetadata {}

#[allow(unused_variables)]
impl<F> AirInstanceWitnessComputation<F> for ArithSM<F> {
    fn start_proof(&self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx) {
        trace!("{}: ··· Starting proof", Self::MY_NAME);

        // For testing purposes only we decide to add some mock data here.
        proof_ctx.air_instance_map = true;
        if execution_ctx.air_instances_map {
            let mut xxx = Vec::new();
            xxx.push(AirInstance {
                airgroup_id: 0,
                air_id: 0,
                instance_id: 0,
                meta: Some(Box::new(ArithSMMetadata {})),
            });
        }
    }

    fn end_proof(&self, proof_ctx: &ProofCtx<F>) {
        trace!("Ending proof for ArithSM");
    }

    fn calculate_witness(&self, stage: u32, proof_ctx: &ProofCtx<F>, air_instance: &AirInstance) {
        trace!("Calculating witness for ArithSM at stage {}", stage);
    }
}
