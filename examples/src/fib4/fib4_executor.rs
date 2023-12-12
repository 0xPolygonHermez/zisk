use math::FieldElement;
use proofman::executor::Executor;
use proofman::proof_ctx::ProofCtx;
use proofman::trace;
use math::fields::f64::BaseElement;

use log::debug;

pub struct FibonacciExecutor<T> {
    phantom: std::marker::PhantomData<T>,
}

impl<T> FibonacciExecutor<T> {
    pub fn new() -> Self {
        FibonacciExecutor { phantom: std::marker::PhantomData }
    }
}

impl<T: FieldElement> Executor<T> for FibonacciExecutor<T> {
    fn witness_computation(&self, stage_id: u32, _subproof_id: u32, _instance_id: i32, proof_ctx: &ProofCtx<T>) {
        if stage_id != 1 {
            debug!("Nothing to do for stage_id {}", stage_id);
            return;
        }

        // NOTE! This is a hack to get the Fibonacci example working.
        let num_rows = 16;

        trace!(Fibonacci {
            a: BaseElement,
            b: BaseElement
        });
        let mut fibonacci = Fibonacci::new(num_rows);

        fibonacci.a[0] = BaseElement::new(1);
        fibonacci.b[0] = BaseElement::new(1);

        for i in 1..num_rows {
            fibonacci.a[i] = fibonacci.b[i - 1];
            fibonacci.b[i] = fibonacci.a[i - 1] + fibonacci.b[i - 1];
        }

        proof_ctx.add_trace_to_air_instance(0, 0, fibonacci);

        // let mut witness = proof_ctx.witnesses[instance_id as usize].lock().unwrap();
        // let mut witness = witness.borrow_mut();
        // let mut witness = witness.as_any_mut().downcast_mut::<FibonacciWitness>().unwrap();
        // witness.compute_witness(stage_id, subproof_id);
    }
}