use math::FieldElement;
use proofman::executor::Executor;
use proofman::proof_ctx::ProofCtx;
use proofman::trace;
use math::fields::f64::BaseElement;

use log::debug;

pub struct FibonacciExecutor<T> {
    pub name: String,
    phantom: std::marker::PhantomData<T>,
}

impl<T> FibonacciExecutor<T> {
    pub fn new(name: String) -> Self {
        FibonacciExecutor {
            name,
            phantom: std::marker::PhantomData
        }
    }
}

impl<T: FieldElement> Executor<T> for FibonacciExecutor<T> {
    fn get_name(&self) -> &str {
        self.name.as_str()
    }

    fn witness_computation(&self, stage_id: u32, _subproof_id: u32, _instance_id: i32, proof_ctx: &ProofCtx<T>) {
        if stage_id != 1 {
            debug!("Nothing to do for stage_id {}", stage_id);
            return;
        }


        // let mut witness = proof_ctx.witnesses[instance_id as usize].lock().unwrap();
        // let mut witness = witness.borrow_mut();
        // let mut witness = witness.as_any_mut().downcast_mut::<FibonacciWitness>().unwrap();
        // witness.compute_witness(stage_id, subproof_id);
    }
}